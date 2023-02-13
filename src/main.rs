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

use dotenv::dotenv;
use oath_routes::{
	static_rocket_route_info_for_authorize, static_rocket_route_info_for_authorize_consent,
	static_rocket_route_info_for_protected_resource, static_rocket_route_info_for_refresh,
	static_rocket_route_info_for_token, MyState,
};
use routes::*;
use std::env;

mod db;
mod models;
#[path = "routes/oauth.rs"]
mod oath_routes;
mod routes;
mod schema;

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
				login,
				find_user,
				authorize,
				authorize_consent,
				token,
				protected_resource,
				refresh
			],
		)
}

fn main() {
	// let _output = if cfg!(target_os = "windows") {
	//     Command::new("cmd")
	//         .args(&["/C", "cd ui && npm start"])
	//         .spawn()
	//         .expect("Failed to start UI Application")
	// } else {
	//     Command::new("sh")
	//         .arg("-c")
	//         .arg("cd ui && npm start")
	//         .spawn()
	//         .expect("Failed to start UI Application")
	// };
	rocket().launch();
}
