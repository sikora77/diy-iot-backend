use std::env;

use crate::db::Conn as DbConn;
use crate::models::{DeviceSignature, Light, NewUser, Trait, User};

use crate::models::{Device, LoginUser, NewDevice, UserData};

use rocket_contrib::json::Json;
use serde_json::Value;

use argon2::{
	password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
	Argon2,
};

use self::constants::{NON_RGB_LIGHT, RGB_LIGHT};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::{Signer, Verifier};
use rocket::http::{Cookie, Cookies, SameSite};
use rocket::response::content::Html;

pub const SESSION_STRING: &str = "session-token";
#[path = "./jwt_issuer.rs"]
mod jwt_issuer;
#[path = "./utils.rs"]
mod utils;

#[path = "./constants.rs"]
mod constants;

// Returns the devices owned by the user
#[get("/devices")]
pub fn get_devices(cookies: Cookies, conn: DbConn) -> Json<Value> {
	let cookie = cookies.get(SESSION_STRING);
	let user_id = utils::get_user_id_from_cookie(cookie);
	println!("{:?}", user_id);
	if user_id.is_none() {
		return Json(json!({"error":"not logged in"}));
	}
	let devices = Device::get_devices_by_user(user_id.unwrap(), &conn);
	return Json(json! ({"status":200,"devices":devices}));
}

// #[get("/device/<device_id>")]
// pub fn get_device() {}
#[post("/register_device", format = "application/json", data = "<new_device>")]
pub fn register_device(cookies: Cookies, conn: DbConn, new_device: Json<NewDevice>) -> Json<Value> {
	let cookie = cookies.get(SESSION_STRING);
	let user_id = utils::get_user_id_from_cookie(cookie);
	if user_id.is_none() {
		return Json(json!({"error":"not logged in"}));
	}
	// Verifies that the device is signed by my private key
	let device_data = DeviceSignature {
		id: new_device.id,
		type_: new_device.type_.clone(),
	};
	let verified = utils::verify_secret(
		new_device.secret.clone(),
		serde_json::to_string(&device_data).unwrap(),
	);
	println!("{}", verified);
	if !verified {
		return Json(json!({"error":"failed to authenticate device"}));
	}
	// Sets the device traits used for google home integrartion
	let traits: Vec<Option<String>> =
		Trait::get_traits_for_device_type(new_device.type_.clone(), &conn)
			.iter()
			.map(|trait_| Some(trait_.trait_.clone()))
			.collect();

	let device = Device {
		id: new_device.id,
		user_id: user_id.unwrap(),
		type_: new_device.type_.clone(),
		internal_name: new_device.type_.clone() + new_device.id.to_string().as_str(),
		name: new_device.name.clone(),
		nicknames: vec![],
		traits: traits,
	};
	let matcher = match device.type_.as_str() {
		RGB_LIGHT => Some(device.type_.clone()),
		NON_RGB_LIGHT => Some(device.type_.clone()),
		_ => None,
	};
	if matcher.is_none() {
		return Json(json! ({"error":"invalid device type"}));
	}

	let transaction_status = conn.build_transaction().run(|| {
		let mut status =
			Light::insert_device(device.id, &conn, new_device.secret.clone(), device.user_id);
		println!("{}", status);
		if status {
			status = Device::insert_device(device, &conn);
		}
		println!("{}", status);
		if status {
			return Ok(());
		} else {
			return Err(diesel::result::Error::__Nonexhaustive);
		}
	});
	return Json(json! ({"status":200,"result":transaction_status.is_ok()}));
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
pub fn register(conn: DbConn, mut new_user: Json<NewUser>, mut cookies: Cookies) -> Json<Value> {
	let password = new_user.password.as_bytes();
	let salt = SaltString::generate(&mut OsRng);
	let argon2 = Argon2::default();
	let password_hash = argon2.hash_password(password, &salt);
	if password_hash.is_err() {
		return Json(json!({"error":"something went wrong when trying to hash a password"}));
	}
	new_user.password = password_hash.unwrap().to_string();
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
				"result": true,
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
	let parsed_hash = PasswordHash::new(&user.first().unwrap().password);
	if parsed_hash.is_err() {
		return Json(json!({
			"error":"invalid password"
		}));
	}
	if Argon2::default()
		.verify_password(user_data.password.as_bytes(), &parsed_hash.unwrap())
		.is_err()
	{
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
#[post("/logout")]
pub fn logout(mut cookies: Cookies) -> Json<Value> {
	cookies.remove(Cookie::named(SESSION_STRING));
	return Json(json!({
		"status":200,
		"result":true,
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

#[get("/getToken?<code>")]
pub fn get_token(conn: DbConn, code: String) -> Html<String> {
	let string1 = "<html><head><script>var details={redirect_uri:'http://localhost:8000/api/v1/getToken',code:'".to_string();
	let string2 = "',client_id:'LocalClient',grant_type:'authorization_code'},formBody=[];for(var property in details){var o=encodeURIComponent(property),t=encodeURIComponent(details[property]);formBody.push(o+'='+t)}fetch('http://192.168.33.108:8000/api/v1/token',{method:'POST',headers:{'Content-Type':'application/x-www-form-urlencoded;charset=UTF-8'},body:formBody=formBody.join('&')}).then(o=>o.json()).then(o=>console.log(o.access_token));</script></head></html>";
	let final_string = string1 + code.as_str() + string2;
	Html(final_string)
}
