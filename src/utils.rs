#![allow(dead_code)]

use std::env;

use base64::{engine::general_purpose, Engine};
use coap_client::{ClientOptions, HostOptions, RequestOptions, TokioClient};
use diesel::{Connection, PgConnection};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use openssl::{
	hash::MessageDigest,
	pkey::PKey,
	rsa::Rsa,
	sign::{Signer, Verifier},
};
use rocket::http::Cookie;
use tokio::runtime::Runtime;

use crate::{models::Device, JWT_SECRET};

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
pub fn verify_secret(secret: String, data: String) -> bool {
	let signature = &general_purpose::STANDARD_NO_PAD.decode(secret).unwrap();
	let device_secret_key = Rsa::private_key_from_pem(
		env::var("DEVICE_SECRET_KEY")
			.expect("set DEVICE_SECRET_KEY")
			.as_bytes(),
	);
	if device_secret_key.is_err() {
		// return Json(json! ({"error":"something went wrong"}));
	}
	let keypair = device_secret_key.unwrap();
	let keypair = PKey::from_rsa(keypair).unwrap();
	let mut verifier = Verifier::new(MessageDigest::sha256(), &keypair).unwrap();
	verifier.update(data.as_bytes()).unwrap();
	verifier.verify(&signature).unwrap()
}

pub fn handle_startup() {
	let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
	let connection = PgConnection::establish(&database_url)
		.unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
	let devices = Device::get_all_devices(&connection);
	insert_devices_into_coap_server(devices);
}

pub fn insert_devices_into_coap_server(devices: Vec<Device>) {
	let mut host_opts = HostOptions::default();
	host_opts.host = "localhost".to_string();
	host_opts.port = 5683;
	let mut req_opts = RequestOptions::default();
	req_opts.non_confirmable = false;
	let rt = Runtime::new().unwrap();
	rt.block_on(async {
		let mut client = TokioClient::connect(host_opts, &ClientOptions::default()).await;
		for device in devices {
			let resp = client
				.as_mut()
				.unwrap()
				.put(
					"/devices/create",
					Some(device.id.to_string().as_bytes()),
					&req_opts,
				)
				.await;
			// TODO handle errors
			if resp.is_err() {
				//something went wrong when sending the request
			} else {
				// let resp_payload = String::from_utf8(resp.unwrap_or_default());
				if resp.unwrap_or_default().len() != 0 {
				} else {
					// println!("{}", resp_payload.unwrap());
				}
			}
		}
	});
}
