use crate::db::Conn as DbConn;
use crate::models::{NewUser, User};

use crate::models::{Device, LoginUser, NewDevice, UserData};

use rocket_contrib::json::Json;
use serde_json::Value;

use std::io;
use std::sync::Mutex;

use oxide_auth::endpoint::{OwnerConsent, Solicitation};
use oxide_auth::frontends::simple::endpoint::{FnSolicitor, Generic, Vacant};
use oxide_auth::primitives::prelude::*;
use oxide_auth::primitives::registrar::RegisteredUrl;
use oxide_auth_rocket::{OAuthFailure, OAuthRequest, OAuthResponse};
use rocket::http::{ContentType, Cookie, Cookies, SameSite};
use rocket::response::Responder;
use rocket::{http, Data, Response, State};

pub const SESSION_STRING: &str = "session-token";
#[path = "./jwt_issuer.rs"]
mod jwt_issuer;
#[path = "./utils.rs"]
mod utils;

// Returns the devices owned by the user
#[get("/devices")]
pub fn get_devices(cookies: Cookies, conn: DbConn) -> Json<Value> {
	let cookie = cookies.get(SESSION_STRING);
	let user_id = utils::get_user_id_from_cookie(cookie);
	if user_id.is_none() {
		return Json(json!({"error":"not logged in"}));
	}
	let devices = Device::get_devices_by_user(user_id.unwrap(), &conn);
	return Json(json! ({"status":200,"devices":devices}));
}

#[post("/register_device", format = "application/json", data = "<new_device>")]
pub fn register_device(cookies: Cookies, conn: DbConn, new_device: Json<NewDevice>) -> Json<Value> {
	let cookie = cookies.get(SESSION_STRING);
	let user_id = utils::get_user_id_from_cookie(cookie);
	if user_id.is_none() {
		return Json(json!({"error":"not logged in"}));
	}
	// TODO verify the device secret
	let status = Device::insert_device(new_device.into_inner(), &conn);
	return Json(json! ({"status":200,"result":status}));
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
				SESSION_STRING,
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
				"result": "true",
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
		SESSION_STRING,
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
pub fn get_me(conn: DbConn, cookies: Cookies) -> Json<Value> {
	let jwt = cookies.get(SESSION_STRING);
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
