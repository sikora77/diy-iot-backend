use super::schema::devices::dsl::devices as all_devices;
use super::schema::lights::dsl::lights as all_lights;
use super::schema::traits::dsl::traits as all_traits;
use super::schema::users::dsl::users as all_users;
use super::schema::{devices, lights, traits, users};
use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;
// this is to get users from the database
#[derive(Serialize, Queryable)]
pub struct User {
	pub id: i32,
	pub email: String,
	pub password: String,
	pub first_name: String,
	pub last_name: String,
}

#[derive(Serialize, Deserialize, Queryable, Insertable, Clone)]
#[diesel(belongs_to(User))]
pub struct Device {
	pub id: Uuid,
	pub type_: String,
	pub user_id: i32,
	pub internal_name: String,
	pub name: String,
	pub nicknames: Vec<Option<String>>,
	pub traits: Vec<Option<String>>,
}

// decode request data
#[derive(Deserialize)]
pub struct UserData {
	pub email: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LoginUser {
	pub email: String,
	pub password: String,
}
// this is to insert users to database
#[derive(Serialize, Deserialize, Insertable, Clone)]
#[table_name = "users"]
pub struct NewUser {
	pub email: String,
	pub password: String,
	pub first_name: String,
	pub last_name: String,
}
#[derive(Serialize, Deserialize, Queryable, Insertable, Clone)]
#[diesel(belongs_to(User))]
#[table_name = "lights"]
pub struct Light {
	pub light_id: Uuid,
	pub rgb: i32,
	pub brightness: i32,
	pub is_on: bool,
	pub user_id: i32,
	pub secret: String,
}

#[derive(Serialize, Deserialize, Queryable, Insertable, Clone)]
#[table_name = "traits"]
pub struct Trait {
	pub id: i32,
	pub device_type: String,
	pub trait_: String,
}

impl Trait {
	pub fn get_traits_for_device_type(device_type: String, conn: &PgConnection) -> Vec<Trait> {
		all_traits
			.filter(traits::device_type.eq(device_type))
			.load::<Trait>(conn)
			.expect("error!")
	}
}

impl Light {
	fn new(_light_id: Uuid, secret: String, user_id: i32) -> Self {
		return Self {
			light_id: _light_id,
			rgb: 255 * 255 * 255,
			brightness: 255,
			is_on: true,
			secret: secret,
			user_id,
		};
	}
	pub fn insert_device(
		_light_id: Uuid,
		conn: &PgConnection,
		secret: String,
		user_id: i32,
	) -> bool {
		diesel::insert_into(lights::table)
			.values(&Light::new(_light_id, secret, user_id))
			.execute(conn)
			.is_ok()
	}
	pub fn get_devices_by_user(user_id: i32, conn: &PgConnection) -> Vec<Light> {
		all_lights
			.filter(lights::user_id.eq(user_id))
			.load::<Light>(conn)
			.expect("error!")
	}
}
// this is to insert users to database
#[derive(Serialize, Deserialize)]
pub struct NewDevice {
	pub id: Uuid,
	pub type_: String,
	pub secret: String,
	pub name: String,
}
#[derive(Serialize, Deserialize)]
pub struct DeviceSignature {
	pub id: Uuid,
	pub type_: String,
}

impl User {
	pub fn get_all_users(conn: &PgConnection) -> Vec<User> {
		all_users
			.order(users::id.desc())
			.load::<User>(conn)
			.expect("error!")
	}

	pub fn insert_user(user: NewUser, conn: &PgConnection) -> bool {
		diesel::insert_into(users::table)
			.values(&user)
			.execute(conn)
			.is_ok()
	}

	pub fn get_user_by_email(email: String, conn: &PgConnection) -> Vec<User> {
		all_users
			.filter(users::email.eq(email))
			.load::<User>(conn)
			.expect("error!")
	}
	pub fn get_user_by_id(id: i32, conn: &PgConnection) -> Vec<User> {
		all_users
			.filter(users::id.eq(id))
			.load::<User>(conn)
			.expect("error!")
	}
}

impl Device {
	pub fn get_all_devices(conn: &PgConnection) -> Vec<Device> {
		all_devices
			.order(devices::id.desc())
			.load::<Device>(conn)
			.expect("error!")
	}

	pub fn insert_device(device: Device, conn: &PgConnection) -> bool {
		diesel::insert_into(devices::table)
			.values(&device)
			.execute(conn)
			.is_ok()
	}

	pub fn get_devices_by_user(user_id: i32, conn: &PgConnection) -> Vec<Device> {
		all_devices
			.filter(devices::user_id.eq(user_id))
			.load::<Device>(conn)
			.expect("error!")
	}
}
