#![allow(dead_code)]

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::Cookie;

use crate::JWT_SECRET;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
	pub sub: String,
	pub exp: usize,
}

pub fn jwt_from_id(user_id: String, timestamp: usize) -> String {
	let claims = Claims {
		sub: user_id,
		exp: timestamp,
	};
	let token = encode(
		&Header::default(),
		&claims,
		&EncodingKey::from_secret(JWT_SECRET.as_ref()),
	);
	return token.unwrap();
}

pub fn claim_form_jwt(jwt: String) -> Option<Claims> {
	let token = decode::<Claims>(
		&jwt,
		&DecodingKey::from_secret(JWT_SECRET.as_ref()),
		&Validation::default(),
	);
	if token.is_err() {
		return None;
	}
	return Some(token.unwrap().claims);
}
pub fn get_user_id_from_cookie(cookie: Option<&Cookie>) -> Option<i32> {
	if cookie.is_none() {
		return None;
	}
	let cookie_value = cookie.unwrap().value();
	let claim = claim_form_jwt(cookie_value.to_string());
	if claim.is_none() {
		return None;
	}
	let return_value = claim.unwrap().sub.parse::<i32>();
	if return_value.is_err() {
		return None;
	}
	return Some(return_value.unwrap());
}
