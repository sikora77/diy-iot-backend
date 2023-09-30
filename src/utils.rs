#![allow(dead_code)]

use std::env;

use base64::{engine::general_purpose, Engine};
use coap_client::{ClientOptions, HostOptions, RequestOptions, TokioClient};
use coap_lite::Packet;
use diesel::{Connection, PgConnection};
use dotenv::dotenv;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use openssl::{hash::MessageDigest, pkey::PKey, rsa::Rsa, sign::Verifier};
use rocket::http::{Cookie, Cookies};
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::{
	models::{Device, LightState},
	routes::SESSION_STRING,
	JWT_SECRET,
};

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
		dotenv::var("DEVICE_SECRET_KEY")
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
	dotenv().ok();
	let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
	let mut connection = &mut PgConnection::establish(&database_url)
		.unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
	let devices = Device::get_all_devices(&mut connection);
	insert_devices_into_coap_server(devices);
}

pub fn insert_devices_into_coap_server(devices: Vec<Device>) {
	let mut host_opts = HostOptions::default();
	let mut host_opts = HostOptions::default();
	let ip = env::var("IP").expect("set IP");
	println!("{}", ip);
	host_opts.host = ip;
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
					"/lights/create",
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

pub fn is_user_logged_in(cookies: Cookies) -> Option<i32> {
	let cookie = cookies.get(SESSION_STRING);
	let user_id = get_user_id_from_cookie(cookie);
	println!("{:?}", user_id);
	user_id
}

pub async fn send_device_command(
	light_state: LightState,
	device_id: Uuid,
) -> Result<Packet, coap_client::Error<std::io::Error>> {
	let mut host_opts = HostOptions::default();
	let ip = env::var("IP").expect("set IP");
	host_opts.host = ip;
	host_opts.port = 5683;
	let mut req_opts = RequestOptions::default();
	req_opts.non_confirmable = false;
	let mut client = TokioClient::connect(host_opts, &ClientOptions::default())
		.await
		.unwrap();
	client
		.put_and_get_packet(
			format!("/lights/{}", device_id.to_string()).as_str(),
			Some(serde_json::to_string(&light_state).unwrap().as_bytes()),
			&req_opts,
		)
		.await
}

pub async fn create_coap_device(
	device_id: Uuid,
	device_type: &str,
) -> Result<Packet, coap_client::Error<std::io::Error>> {
	let mut host_opts = HostOptions::default();
	let ip = env::var("IP").expect("set IP");
	host_opts.host = ip;
	host_opts.port = 5683;
	let mut req_opts = RequestOptions::default();
	req_opts.non_confirmable = false;
	let mut client = TokioClient::connect(host_opts, &ClientOptions::default())
		.await
		.unwrap();
	client
		.put_and_get_packet(
			format!("/{}/create", device_type).as_str(),
			Some(device_id.to_string().as_bytes()),
			&req_opts,
		)
		.await
}
