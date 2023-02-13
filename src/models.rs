use super::schema::devices::dsl::devices as all_devices;
use super::schema::users::dsl::users as all_users;
use super::schema::{devices, users};
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

#[derive(Serialize, Deserialize, Queryable)]
#[diesel(belongs_to(User))]
pub struct Device {
	pub id: Uuid,
	pub type_: String,
	pub user_id: i32,
	pub secret: String,
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

// this is to insert users to database
#[derive(Serialize, Deserialize, Insertable)]
#[table_name = "devices"]
pub struct NewDevice {
	pub id: Uuid,
	pub type_: String,
	pub user_id: i32,
	pub secret: String,
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

	pub fn insert_device(device: NewDevice, conn: &PgConnection) -> bool {
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
