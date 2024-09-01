#![feature(decl_macro, proc_macro_hygiene)]
#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

extern crate diesel;
extern crate dotenv;
extern crate r2d2;
#[macro_use]
extern crate rocket;
extern crate rocket_contrib;
extern crate rocket_cors;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use dotenv::dotenv;
use google_routes::static_rocket_route_info_for_fullfilment;
use oath_routes::{
	static_rocket_route_info_for_authorize, static_rocket_route_info_for_authorize_consent,
	static_rocket_route_info_for_get_token, static_rocket_route_info_for_protected_resource,
	static_rocket_route_info_for_refresh, static_rocket_route_info_for_token, MyState,
};
use rocket::http::Method;
use rocket_cors::{AllowedOrigins, Cors, CorsOptions};
use routes::{
	device::{
		static_rocket_route_info_for_check_device_online, static_rocket_route_info_for_get_devices,
		static_rocket_route_info_for_get_full_devices,
		static_rocket_route_info_for_register_device, static_rocket_route_info_for_remove_device,
		static_rocket_route_info_for_rename_device, static_rocket_route_info_for_set_brightness,
		static_rocket_route_info_for_set_color, static_rocket_route_info_for_set_on,
	},
	user::{
		static_rocket_route_info_for_get_me, static_rocket_route_info_for_login,
		static_rocket_route_info_for_logout, static_rocket_route_info_for_register,
	},
};

//use routes::*;
use std::env;

mod db;
mod models;
#[path = "routes/oauth.rs"]
mod oath_routes;

#[path = "routes/google.rs"]
mod google_routes;
mod schema;

pub mod constants;
pub mod routes;

pub mod utils;

pub const JWT_SECRET: &str = "hewwo-uwu";

fn rocket() -> rocket::Rocket {
	dotenv().ok();
	let database_url = env::var("DATABASE_URL").expect("set DATABASE_URL");

	let pool = db::init_pool(database_url);
	rocket::ignite()
		.manage(pool)
		.manage(MyState::preconfigured())
		.mount(
			"/api/v1/",
			routes![
				register,
				get_me,
				get_devices,
				register_device,
				login,
				fullfilment,
				logout,
				set_brightness,
				set_color,
				set_on,
				get_full_devices,
				check_device_online,
				rename_device,
				remove_device,
			],
		)
		.mount(
			"/oauth",
			routes![
				token,
				authorize,
				authorize_consent,
				protected_resource,
				refresh,
				get_token,
			],
		)
		.mount("/", rocket_cors::catch_all_options_routes())
		.manage(make_cors())
		.attach(make_cors())
}
fn make_cors() -> Cors {
	let frontend_addr = env::var("FRONTEND_ADDR").expect("set FRONTEND_ADDR");
	let allowed_origins = AllowedOrigins::some_exact(&[
		frontend_addr,
		"http://localhost:3000".to_owned(),
		//"http://localhost:8000",
		"http://sikora-laptop.local:3000".to_owned(),
		// "http://sikora-laptop.local:8080".to_owned(),
		"http://sikora-laptop.local:8000".to_owned(),
		"http://sikora-laptop.local:22070".to_owned(),
	]);

	CorsOptions {
		allowed_origins,
		allowed_methods: vec![Method::Get, Method::Post, Method::Options]
			.into_iter()
			.map(From::from)
			.collect(), // 1.
		// allowed_headers: AllowedHeaders::some(&[
		// 	"Authorization",
		// 	"Accept",
		// 	"Access-Control-Allow-Origin",
		// ]),
		allow_credentials: true,
		..Default::default()
	}
	.to_cors()
	.expect("error while building CORS")
}

fn main() {
	utils::handle_startup();
	rocket().launch();
}
