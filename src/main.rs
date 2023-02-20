#![feature(plugin, decl_macro, proc_macro_hygiene)]
#![allow(proc_macro_derive_resolution_fallback, unused_attributes)]

#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate r2d2;
extern crate r2d2_diesel;
#[macro_use]
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use db::Pool;
use diesel::{Connection, PgConnection};
use dotenv::dotenv;
use google_routes::static_rocket_route_info_for_fullfilment;
use models::Device;
use oath_routes::{
	static_rocket_route_info_for_authorize, static_rocket_route_info_for_authorize_consent,
	static_rocket_route_info_for_protected_resource, static_rocket_route_info_for_refresh,
	static_rocket_route_info_for_token, MyState,
};
use r2d2_diesel::ConnectionManager;
use routes::*;
use std::env;

mod db;
mod models;
#[path = "routes/oauth.rs"]
mod oath_routes;

#[path = "routes/google.rs"]
mod google_routes;
mod routes;
mod schema;
#[path = "utils.rs"]
mod utils;

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
				get_all,
				register,
				get_me,
				get_devices,
				register_device,
				login,
				fullfilment,
				find_user,
				authorize,
				authorize_consent,
				logout,
				token,
				protected_resource,
				refresh,
				get_token,
			],
		)
}

fn main() {
	utils::handle_startup();
	rocket().launch();
}
