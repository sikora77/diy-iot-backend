use crate::models::Device;
use crate::utils::is_user_logged_in;

use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request};

use rocket::http::Status;

pub const SESSION_STRING: &str = "session-token";

pub struct AuthUser {
	pub user_id: i32,
}

pub mod device;
pub mod user;

#[derive(Debug)]
pub enum UserError {
	UserNotLoggedIn,
}

impl<'a, 'r> FromRequest<'a, 'r> for AuthUser {
	type Error = UserError;
	fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
		match is_user_logged_in(request.cookies()) {
			None => Outcome::Failure((Status::Unauthorized, UserError::UserNotLoggedIn)),
			Some(x) => Outcome::Success(AuthUser { user_id: x }),
		}
	}
}
#[derive(Debug)]
pub enum DeviceError {
	DeviceNotOwned,
}

// #[derive(Serialize, Deserialize)]
// struct Response<Resp, Err> {
// 	response: Option<Resp>,
// 	errors: Option<Vec<Err>>,
// }

// impl<'a, 'r> FromRequest<'a, 'r> for Device {
// 	type Error = DeviceError;
// 	fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
// 		request.
// 	}
// }
