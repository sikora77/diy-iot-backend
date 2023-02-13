use super::db::Conn as DbConn;
use super::models::{NewUser, User};
use crate::jwt_secret;
use crate::models::{Device, LoginUser, UserData};
use jsonwebtoken::{decode, DecodingKey, Validation};
use rocket_contrib::json::Json;
use serde_json::Value;

use std::io;
use std::sync::Mutex;

use oxide_auth::endpoint::{OwnerConsent, Solicitation};
use oxide_auth::frontends::simple::endpoint::{FnSolicitor, Generic, Vacant};
use oxide_auth::primitives::prelude::*;
use oxide_auth::primitives::registrar::RegisteredUrl;
use oxide_auth_rocket::{OAuthFailure, OAuthRequest, OAuthResponse};
use rocket::http::{ContentType, Cookie, Cookies, SameSite, Status};
use rocket::response::{Redirect, Responder};
use rocket::{http, Data, Response, State};

const session_string: &str = "session-token";
#[path = "./jwt_issuer.rs"]
mod JwtIssuer;
#[path = "./utils.rs"]
mod utils;
pub struct MyState {
	registrar: Mutex<ClientMap>,
	authorizer: Mutex<AuthMap<RandomGenerator>>,
	issuer: Mutex<JwtIssuer::JwtIssuer>,
}

#[get("/authorize")]
pub fn authorize<'r>(
	oauth: OAuthRequest<'r>,
	state: State<MyState>,
) -> Result<OAuthResponse<'r>, OAuthFailure> {
	state
		.endpoint()
		.with_solicitor(FnSolicitor(consent_form))
		.authorization_flow()
		.execute(oauth)
		.map_err(|err| err.pack::<OAuthFailure>())
}

#[post("/authorize?<allow>")]
pub fn authorize_consent<'r>(
	oauth: OAuthRequest<'r>,
	allow: Option<bool>,
	state: State<MyState>,
	mut cookies: Cookies,
) -> Result<OAuthResponse<'r>, OAuthFailure> {
	let allowed = allow.unwrap_or(false);
	let user_id = cookies.get(session_string);
	if user_id.is_none() {
		return state
			.endpoint()
			.with_solicitor(FnSolicitor(move |_: &mut _, grant: Solicitation<'_>| {
				consent_decision(allowed, grant, "-1".to_string())
			}))
			.authorization_flow()
			.execute(oauth)
			.map_err(|err| err.pack::<OAuthFailure>());
	}
	let claims = utils::claim_form_jwt(user_id.unwrap().value().to_string());
	if claims.is_none() {
		return state
			.endpoint()
			.with_solicitor(FnSolicitor(move |_: &mut _, grant: Solicitation<'_>| {
				consent_decision(allowed, grant, "-1".to_string())
			}))
			.authorization_flow()
			.execute(oauth)
			.map_err(|err| err.pack::<OAuthFailure>());
	}
	let user_id = claims.unwrap().sub;
	println!("{}", user_id);
	state
		.endpoint()
		.with_solicitor(FnSolicitor(move |_: &mut _, grant: Solicitation<'_>| {
			consent_decision(allowed, grant, user_id.clone())
		}))
		.authorization_flow()
		.execute(oauth)
		.map_err(|err| err.pack::<OAuthFailure>())
}

#[post("/token", data = "<body>")]
pub fn token<'r>(
	mut oauth: OAuthRequest<'r>,
	body: Data,
	state: State<MyState>,
) -> Result<OAuthResponse<'r>, OAuthFailure> {
	oauth.add_body(body);
	state
		.endpoint()
		.access_token_flow()
		.execute(oauth)
		.map_err(|err| err.pack::<OAuthFailure>())
}

#[post("/refresh", data = "<body>")]
pub fn refresh<'r>(
	mut oauth: OAuthRequest<'r>,
	body: Data,
	state: State<MyState>,
) -> Result<OAuthResponse<'r>, OAuthFailure> {
	oauth.add_body(body);
	state
		.endpoint()
		.refresh_flow()
		.execute(oauth)
		.map_err(|err| err.pack::<OAuthFailure>())
}

// Returns the devices owned by the user
#[get("/devices")]
pub fn get_devices(mut cookies: Cookies, conn: DbConn) -> Json<Value> {
	let cookie = cookies.get(session_string);
	let user_id = utils::get_user_id_from_cookie(cookie);
	if (user_id.is_none()) {
		return Json(json!({"error":"not logged in"}));
	}
	let devices = Device::get_devices_by_user(user_id.unwrap(), &conn);
	return Json(json! ({"status":200,"devices":devices}));
}

#[get("/")]
pub fn protected_resource<'r>(
	oauth: OAuthRequest<'r>,
	state: State<MyState>,
) -> impl Responder<'r> {
	const DENY_TEXT: &str = "<html>
This page should be accessed via an oauth token from the client in the example. Click
<a href=\"/api/v1/authorize?response_type=code&client_id=LocalClient\">
here</a> to begin the authorization process.
</html>
";

	let protect = state
		.endpoint()
		.with_scopes(vec!["default-scope".parse().unwrap()])
		.resource_flow()
		.execute(oauth);
	match protect {
		Ok(grant) => Ok("Hello, world"),
		Err(Ok(response)) => {
			let error: OAuthResponse = Response::build_from(response.into())
				.header(ContentType::HTML)
				.sized_body(io::Cursor::new(DENY_TEXT))
				.finalize()
				.into();
			Err(Ok(error))
		}
		Err(Err(err)) => Err(Err(err.pack::<OAuthFailure>())),
	}
}

#[post("/users", format = "application/json")]
pub fn get_all(conn: DbConn) -> Json<Value> {
	let users = User::get_all_users(&conn);
	Json(json!({
		"status": 200,
		"result": users,
	}))
}

#[post("/register", format = "application/json", data = "<new_user>")]
pub fn register(conn: DbConn, new_user: Json<NewUser>, mut cookies: Cookies) -> Json<Value> {
	let status = User::insert_user(new_user.clone(), &conn);
	if !status {
		return Json(json!({"error":"failed to create a user"}));
	}
	let email = new_user.email.clone();
	let user_list = User::get_user_by_email(email, &conn);
	let user = user_list.first();
	return match user {
		Some(user) => {
			let user_id = user.id.to_string();
			let session_cookie = Cookie::build(
				session_string,
				utils::jwt_from_id(
					user_id.clone(),
					(chrono::Utc::now().timestamp() + 365 * 24 * 60 * 60) as usize,
				),
			)
			.path("/")
			.same_site(SameSite::Strict)
			.http_only(true)
			.finish();
			cookies.add(session_cookie);
			Json(json!({
				"status": status,
				"result": "success",
			}))
		}
		None => Json(json!({"error":"failed to find the user"})),
	};
}

// Login the user and send them a jwt containing their user_id
#[post("/login", format = "application/json", data = "<user_data>")]
pub fn login(user_data: Json<LoginUser>, conn: DbConn, mut cookies: Cookies) -> Json<Value> {
	let user = User::get_user_by_email(user_data.clone().email, &conn);
	// Check if user with given email exists
	if user.first().is_none() {
		return Json(json!({
			"error":"invalid email"
		}));
	}
	// Check if password match
	// TODO implement password hashing
	if user.first().unwrap().password != user_data.password {
		return Json(json!({
			"error":"invalid password"
		}));
	}
	let user_id = user.first().unwrap().id.to_string();
	let session_cookie = Cookie::build(
		session_string,
		utils::jwt_from_id(
			user_id.clone(),
			(chrono::Utc::now().timestamp() + 365 * 24 * 60 * 60) as usize,
		),
	)
	.path("/")
	.same_site(SameSite::Strict)
	.http_only(true)
	.finish();
	cookies.add(session_cookie);
	return Json(json!({
		"status":200
	}));
}
#[get("/me", format = "application/json")]
pub fn get_me(conn: DbConn, mut cookies: Cookies) -> Json<Value> {
	let jwt = cookies.get(session_string);
	if jwt.is_none() {
		return Json(json!({"error":"not logged in"}));
	}
	// `token` is a struct with 2 fields: `header` and `claims` where `claims` is your own struct.
	let token = utils::claim_form_jwt(jwt.unwrap().value().to_string());
	if token.is_none() {
		return Json(json!({
			"status": 401,
			"error": "invalid jwt",
		}));
	}
	let user_id_int: i32 = token.unwrap().sub.parse().unwrap();
	println!("{}", user_id_int);
	let user = User::get_user_by_id(user_id_int, &conn);
	if user.first().is_none() {
		return Json(json!({"error":"invalid user_id"}));
	}
	Json(json!({
		"status": 200,
		"result": user,
	}))
}

#[post("/getUser", format = "application/json", data = "<user_data>")]
pub fn find_user(conn: DbConn, user_data: Json<UserData>) -> Json<Value> {
	let email = user_data.email.clone();
	Json(json!({
		"status": 200,
		"result": User::get_user_by_email(email,&conn),
	}))
}

impl MyState {
	pub fn preconfigured() -> Self {
		MyState {
			registrar: Mutex::new(
				vec![Client::public(
					"LocalClient",
					RegisteredUrl::Semantic(
						"http://localhost:8000/clientside/endpoint".parse().unwrap(),
					),
					"default-scope".parse().unwrap(),
				)]
				.into_iter()
				.collect(),
			),
			// Authorization tokens are 16 byte random keys to a memory hash map.
			authorizer: Mutex::new(AuthMap::new(RandomGenerator::new(16))),
			// Bearer tokens are also random generated but 256-bit tokens, since they live longer
			// and this example is somewhat paranoid.
			//
			// We could also use a `TokenSigner::ephemeral` here to create signed tokens which can
			// be read and parsed by anyone, but not maliciously created. However, they can not be
			// revoked and thus don't offer even longer lived refresh tokens.
			issuer: Mutex::new(JwtIssuer::JwtIssuer::new(RandomGenerator::new(16))),
		}
	}

	pub fn endpoint(&self) -> Generic<impl Registrar + '_, impl Authorizer + '_, impl Issuer + '_> {
		Generic {
			registrar: self.registrar.lock().unwrap(),
			authorizer: self.authorizer.lock().unwrap(),
			issuer: self.issuer.lock().unwrap(),
			// Solicitor configured later.
			solicitor: Vacant,
			// Scope configured later.
			scopes: Vacant,
			// `rocket::Response` is `Default`, so we don't need more configuration.
			response: Vacant,
		}
	}
}

fn consent_form<'r>(
	_: &mut OAuthRequest<'r>,
	solicitation: Solicitation,
) -> OwnerConsent<OAuthResponse<'r>> {
	OwnerConsent::InProgress(
		Response::build()
			.status(http::Status::Ok)
			.header(http::ContentType::HTML)
			.sized_body(io::Cursor::new(consent_page_html(
				"/api/v1/authorize",
				solicitation,
			)))
			.finalize()
			.into(),
	)
}

fn consent_decision<'r>(
	allowed: bool,
	_: Solicitation,
	user_id: String,
) -> OwnerConsent<OAuthResponse<'r>> {
	if allowed {
		OwnerConsent::Authorized(user_id.into())
	} else {
		OwnerConsent::Denied
	}
}

pub fn consent_page_html(route: &str, solicitation: Solicitation) -> String {
	macro_rules! template {
		() => {
			"<html>'{0:}' (at {1:}) is requesting permission for '{2:}'
<form method=\"post\">
    <input type=\"submit\" value=\"Accept\" formaction=\"{4:}?{3:}&allow=true\">
    <input type=\"submit\" value=\"Deny\" formaction=\"{4:}?{3:}&deny=true\">
</form>
</html>"
		};
	}

	let grant = solicitation.pre_grant();
	let state = solicitation.state();
	let user_id = 2.to_string();
	let mut extra = vec![
		("response_type", "code"),
		("client_id", grant.client_id.as_str()),
		("redirect_uri", grant.redirect_uri.as_str()),
		("user_id", user_id.as_str()),
	];

	if let Some(state) = state {
		extra.push(("state", state));
	}

	format!(
		template!(),
		grant.client_id,
		grant.redirect_uri,
		grant.scope,
		serde_urlencoded::to_string(extra).unwrap(),
		&route,
	)
}
