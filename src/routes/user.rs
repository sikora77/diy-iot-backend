use crate::db::Conn as DbConn;
use crate::models::{Me, NewUser, RegisterUser, User};

use crate::models::LoginUser;
use crate::utils::jwt_from_id;

use rocket_contrib::json::Json;
use serde_json::Value;

use argon2::{
	password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
	Argon2,
};

use rocket::http::{Cookie, Cookies, SameSite};

use super::{AuthUser, SESSION_STRING};

#[post("/register", format = "application/json", data = "<new_user>")]
pub fn register(
	mut conn: DbConn,
	mut new_user: Json<RegisterUser>,
	mut cookies: Cookies,
) -> Json<Value> {
	if new_user.rep_password != new_user.password {
		return Json(json!({
			"errors":
				[{
					"field":"password",
					"message":"passwords dont match"
				}]
		}));
	}
	let password = new_user.password.as_bytes();
	let salt = SaltString::generate(&mut OsRng);
	let argon2 = Argon2::default();
	let password_hash = argon2.hash_password(password, &salt);
	if password_hash.is_err() {
		return Json(json!({
			"errors":
				[{
					"field":"password",
					"message":"something went wrong when trying to hash a password"
				}]
		}));
	}
	new_user.password = password_hash.unwrap().to_string();
	let status = User::insert_user(new_user.clone().into(), &mut conn);
	if status.is_err() {
		let error_str = status.unwrap_err().to_string();
		return match error_str.as_str() {
			"duplicate key value violates unique constraint \"users_email_key\"" => Json(json!({
				"errors":
					[{
						"field":"email",
						"message":"user with this email already exists"
					}]
			})),
			_ => Json(json!({
				"errors":
					[{
						"field":"password",
						"message":"failed to create a user"
					}]
			})),
		};
	}
	let email = new_user.email.clone();
	let user_list = User::get_user_by_email(email, &mut conn);
	let user = user_list.first();
	return match user {
		Some(user) => {
			let user_id = user.id.to_string();
			let session_cookie = Cookie::build(
				SESSION_STRING,
				jwt_from_id(
					user_id.clone(),
					(chrono::Utc::now().timestamp() + 365 * 24 * 60 * 60) as usize,
				),
			)
			.path("/")
			.same_site(SameSite::None)
			.secure(false)
			.http_only(true)
			.finish();
			// TODO in prod make cookies secure
			cookies.add(session_cookie);
			Json(json!({
				"worked":true
			}))
		}
		// This should be literally impossible
		None => Json(json!({
			"errors":
				[{
					"field":"password",
					"message":"failed to create a user"
				}]
		})),
	};
}

// Login the user and send them a jwt containing their user_id
#[post("/login", format = "application/json", data = "<user_data>")]
pub fn login(user_data: Json<LoginUser>, mut conn: DbConn, mut cookies: Cookies) -> Json<Value> {
	let user = User::get_user_by_email(user_data.clone().email, &mut conn);
	// Check if user with given email exists
	if user.first().is_none() {
		return Json(json!({
			"errors":
				[{
					"field":"email",
					"message":"invalid email"
				}]
		}));
	}
	// Check if password match
	let parsed_hash = PasswordHash::new(&user.first().unwrap().password);
	if parsed_hash.is_err() {
		return Json(json!({
			"errors":
				[{
					"field":"password",
					"message":"invalid password"
				}]
		}));
	}
	if Argon2::default()
		.verify_password(user_data.password.as_bytes(), &parsed_hash.unwrap())
		.is_err()
	{
		return Json(json!({
			"errors":
				[{
					"field":"password",
					"message":"invalid password"
				}]
		}));
	}

	let user_id = user.first().unwrap().id.to_string();
	let session_cookie = Cookie::build(
		SESSION_STRING,
		jwt_from_id(
			user_id.clone(),
			(chrono::Utc::now().timestamp() + 365 * 24 * 60 * 60) as usize,
		),
	)
	.path("/api/v1")
	.same_site(SameSite::None)
	.secure(false)
	.http_only(true)
	.finish();
	cookies.add(session_cookie);
	return Json(json!({
		"worked":true
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
pub fn get_me(mut conn: DbConn, user: AuthUser) -> Json<Value> {
	let user_id_int: i32 = user.user_id;
	println!("{}", user_id_int);
	let user = User::get_user_by_id(user_id_int, &mut conn);
	if user.first().is_none() {
		return Json(json!({"error":"invalid user_id"}));
	}
	let user = user.first().unwrap();
	let return_user = Me {
		id: user.id,
		email: user.email.clone(),
		first_name: user.first_name.clone(),
		last_name: user.last_name.clone(),
	};
	Json(json!(return_user))
}
