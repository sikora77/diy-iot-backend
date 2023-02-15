use std::{io, sync::Mutex};

use oxide_auth::{
	endpoint::{Authorizer, Issuer, OwnerConsent, Registrar, Solicitation},
	frontends::simple::endpoint::{FnSolicitor, Vacant},
	primitives::{
		prelude::{AuthMap, RandomGenerator, TokenMap},
		registrar::{Client, ClientMap, RegisteredUrl},
	},
};
use oxide_auth_rocket::{Generic, OAuthFailure, OAuthRequest, OAuthResponse};
use rocket::{http, response::Responder};
use rocket::{
	http::{ContentType, Cookies},
	Data, Response, State,
};

#[path = "../jwt_issuer.rs"]
mod jwt_issuer;
#[path = "../utils.rs"]
mod utils;
use crate::SESSION_STRING;
pub struct MyState {
	registrar: Mutex<ClientMap>,
	authorizer: Mutex<AuthMap<RandomGenerator>>,
	issuer: Mutex<TokenMap<RandomGenerator>>,
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
	cookies: Cookies,
) -> Result<OAuthResponse<'r>, OAuthFailure> {
	let allowed = allow.unwrap_or(false);
	let user_id = cookies.get(SESSION_STRING);
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
		Ok(_grant) => Ok("Hello, world"),
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
			issuer: Mutex::new(TokenMap::new(RandomGenerator::new(16))),
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
