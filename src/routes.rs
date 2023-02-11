use super::db::Conn as DbConn;
use super::models::{NewUser, User};
use crate::models::UserData;
use rocket_contrib::json::Json;
use serde_json::Value;

use std::io;
use std::sync::Mutex;

use oxide_auth::endpoint::{OwnerConsent, Solicitation};
use oxide_auth::frontends::simple::endpoint::{FnSolicitor, Generic, Vacant};
use oxide_auth::primitives::prelude::*;
use oxide_auth::primitives::registrar::RegisteredUrl;
use oxide_auth_rocket::{OAuthFailure, OAuthRequest, OAuthResponse};

use rocket::http::{ContentType, Status};
use rocket::response::{Redirect, Responder};
use rocket::{http, Data, Response, State};

#[path = "./jwt_issuer.rs"]
mod JwtIssuer;
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
) -> Result<OAuthResponse<'r>, OAuthFailure> {
	let allowed = allow.unwrap_or(false);
	state
		.endpoint()
		.with_solicitor(FnSolicitor(move |_: &mut _, grant: Solicitation<'_>| {
			consent_decision(allowed, grant)
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

#[post("/newUser", format = "application/json", data = "<new_user>")]
pub fn new_user(conn: DbConn, new_user: Json<NewUser>) -> Json<Value> {
	let status = User::insert_user(new_user.into_inner(), &conn);
	Json(json!({
		"status": status,
		"result": User::get_all_users(&conn).first(),
	}))
}

#[post("/getUser", format = "application/json", data = "<user_data>")]
pub fn find_user(conn: DbConn, user_data: Json<UserData>) -> Json<Value> {
	Json(json!({
		"status": 200,
		"result": User::get_user_by_username(user_data.into_inner(),&conn),
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

fn consent_decision<'r>(allowed: bool, _: Solicitation) -> OwnerConsent<OAuthResponse<'r>> {
	if allowed {
		OwnerConsent::Authorized("dummy user".into())
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
