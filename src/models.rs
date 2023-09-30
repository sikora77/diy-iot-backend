use super::schema::devices::dsl::devices as all_devices;
use super::schema::lights::dsl::lights as all_lights;
use super::schema::traits::dsl::traits as all_traits;
use super::schema::users::dsl::users as all_users;
use super::schema::{devices, lights, traits, users};
use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
// use diesel::sql_types::Uuid;
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
#[derive(Serialize, Deserialize)]
pub struct Me {
	pub id: i32,
	pub email: String,
	pub first_name: String,
	pub last_name: String,
}

#[derive(Serialize, Deserialize, Queryable, Insertable, Clone, Selectable)]
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
#[derive(Serialize, Deserialize, Clone)]
pub struct RegisterUser {
	pub email: String,
	pub password: String,
	pub rep_password: String,
	pub first_name: String,
	pub last_name: String,
}

// this is to insert users to database
#[derive(Serialize, Deserialize, Insertable, Clone, Selectable)]
#[table_name = "users"]
pub struct NewUser {
	pub email: String,
	pub password: String,
	pub first_name: String,
	pub last_name: String,
}
impl std::convert::From<RegisterUser> for NewUser {
	fn from(value: RegisterUser) -> Self {
		Self {
			email: value.email,
			password: value.password,
			first_name: value.first_name,
			last_name: value.last_name,
		}
	}
}
#[derive(Serialize, Deserialize, Queryable, Insertable, Clone, Selectable)]
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

#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceData {
	pub device_id: Uuid,
	pub brightness: Option<i32>,
	pub color: Option<i32>,
	pub is_on: Option<bool>,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct LightState {
	pub is_on: bool,
	pub brightness: i32,
	pub color: i32,
}

#[derive(Serialize, Deserialize, Queryable, Insertable, Clone)]
#[table_name = "traits"]
pub struct Trait {
	pub id: i32,
	pub device_type: String,
	pub trait_: String,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct FullLight {
	pub id: Uuid,
	pub type_: String,
	pub name: String,
	pub nicknames: Vec<Option<String>>,
	pub rgb: i32,
	pub brightness: i32,
	pub is_on: bool,
}

trait BasicDevice {
	fn switch_on(&mut self);
	fn switch_off(&mut self);
}
impl BasicDevice for Light {
	fn switch_on(&mut self) {
		self.is_on = true;
	}
	fn switch_off(&mut self) {
		self.is_on = false;
	}
}

impl Trait {
	pub fn get_traits_for_device_type(device_type: String, conn: &mut PgConnection) -> Vec<Trait> {
		diesel::QueryDsl::filter(all_traits, traits::device_type.eq(device_type))
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
		conn: &mut PgConnection,
		secret: String,
		user_id: i32,
	) -> bool {
		diesel::insert_into(lights::table)
			.values(&Light::new(_light_id, secret, user_id))
			.execute(conn)
			.is_ok()
	}
	pub fn update_device_brightness(
		_light_id: Uuid,
		brightness: i32,
		conn: &mut PgConnection,
		secret: String,
		user_id: i32,
	) -> Light {
		let light_after_update = diesel::update(lights::table)
			.set(lights::brightness.eq(brightness))
			.filter(lights::light_id.eq(_light_id))
			.get_result::<Light>(conn);
		// todo implement error handling
		return light_after_update.unwrap();
	}
	pub fn update_device_color(
		_light_id: Uuid,
		color: i32,
		conn: &mut PgConnection,
		secret: String,
		user_id: i32,
	) -> Light {
		let light_after_update = diesel::update(lights::table)
			.set(lights::brightness.eq(color))
			.filter(lights::light_id.eq(_light_id))
			.get_result::<Light>(conn);
		// todo implement error handling
		return light_after_update.unwrap();
	}
	pub fn update_device_is_on(
		_light_id: Uuid,
		is_on: bool,
		conn: &mut PgConnection,
		secret: String,
		user_id: i32,
	) -> Light {
		let light_after_update = diesel::update(lights::table)
			.set(lights::is_on.eq(is_on))
			.filter(lights::light_id.eq(_light_id))
			.get_result::<Light>(conn);
		// todo implement error handling
		return light_after_update.unwrap();
	}
	pub fn get_devices_by_user(user_id: i32, conn: &mut PgConnection) -> Vec<Light> {
		diesel::query_dsl::methods::FilterDsl::filter(all_lights, lights::user_id.eq(user_id))
			.load::<Light>(conn)
			.expect("error!")
	}
	pub fn get_device_by_id(device_id: Uuid, conn: &mut PgConnection) -> Option<Light> {
		let light = diesel::query_dsl::methods::FilterDsl::filter(
			all_lights,
			lights::light_id.eq(device_id),
		)
		.load::<Light>(conn)
		.expect("error");
		return light.first().cloned();
	}
	pub fn get_full_device_data_by_user(user_id: i32, conn: &mut PgConnection) -> Vec<FullLight> {
		// .select(lights::columns::rgb)
		// 	.select(lights::columns::is_on)
		// 	.select(lights::columns::secret)
		// 	.select(lights::columns::brightness)
		let lights =
			diesel::query_dsl::methods::FilterDsl::filter(all_lights, devices::user_id.eq(user_id))
				.left_join(all_devices.on(devices::id.eq(lights::light_id)))
				.select((Light::as_select(), Option::<Device>::as_select()))
				.load::<(Light, Option<Device>)>(conn)
				.expect("error");

		let data: Vec<FullLight> = lights
			.iter()
			.map(|dev| {
				let device_info = dev.1.clone().unwrap();
				FullLight {
					id: device_info.id,
					type_: device_info.type_,
					brightness: dev.0.brightness,
					name: device_info.name,
					nicknames: device_info.nicknames,
					rgb: dev.0.rgb,
					is_on: dev.0.is_on,
				}
			})
			.collect();
		return data;
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
	pub fn get_all_users(conn: &mut PgConnection) -> Vec<User> {
		all_users
			.order(users::id.desc())
			.load::<User>(conn)
			.expect("error!")
	}

	pub fn insert_user(
		user: NewUser,
		conn: &mut PgConnection,
	) -> Result<usize, diesel::result::Error> {
		diesel::insert_into(users::table)
			.values(&user)
			.execute(conn)
	}

	pub fn get_user_by_email(email: String, conn: &mut PgConnection) -> Vec<User> {
		diesel::query_dsl::methods::FilterDsl::filter(all_users, users::email.eq(email))
			.load::<User>(conn)
			.expect("error!")
	}
	pub fn get_user_by_id(id: i32, conn: &mut PgConnection) -> Vec<User> {
		diesel::query_dsl::methods::FilterDsl::filter(all_users, users::id.eq(id))
			.load::<User>(conn)
			.expect("error!")
	}
}

impl Device {
	pub fn get_all_devices(conn: &mut PgConnection) -> Vec<Device> {
		all_devices
			.order(devices::id.desc())
			.load::<Device>(conn)
			.expect("error!")
	}

	pub fn insert_device(device: Device, conn: &mut PgConnection) -> bool {
		diesel::insert_into(devices::table)
			.values(&device)
			.execute(conn)
			.is_ok()
	}

	pub fn get_devices_by_user(user_id: i32, conn: &mut PgConnection) -> Vec<Device> {
		diesel::query_dsl::methods::FilterDsl::filter(all_devices, devices::user_id.eq(user_id))
			.load::<Device>(conn)
			.expect("error!")
	}
	pub fn get_device_by_id(device_id: Uuid, conn: &mut PgConnection) -> Option<Device> {
		let device =
			diesel::query_dsl::methods::FilterDsl::filter(all_devices, devices::id.eq(device_id))
				.load::<Device>(conn)
				.expect("msg");
		if device.first().is_none() {
			return None;
		}
		Some(device.first().unwrap().clone())
	}
	pub fn get_device_owner(device_id: Uuid, conn: &mut PgConnection) -> Option<i32> {
		let device =
			diesel::query_dsl::methods::FilterDsl::filter(all_devices, devices::id.eq(device_id))
				.load::<Device>(conn)
				.expect("msg");

		if device.first().is_none() {
			return None;
		}
		Some(device.first().unwrap().user_id)
	}
}
