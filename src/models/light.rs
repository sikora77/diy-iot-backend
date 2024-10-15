use crate::schema::devices::dsl::devices as all_devices;
use crate::schema::lights::dsl::lights as all_lights;
use crate::schema::traits::dsl::traits as all_traits;
use crate::schema::{devices, lights, traits};
use diesel::prelude::*;
use diesel::PgConnection;
use uuid::Uuid;

use super::device::Device;

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

// trait BasicDevice {
// 	fn switch_on(&mut self);
// 	fn switch_off(&mut self);
// }
// impl BasicDevice for Light {
// 	fn switch_on(&mut self) {
// 		self.is_on = true;
// 	}
// 	fn switch_off(&mut self) {
// 		self.is_on = false;
// 	}
// }

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
	pub fn remove_device(light_id: Uuid, conn: &mut PgConnection) -> bool {
		let s = diesel::delete(all_lights)
			.filter(lights::light_id.eq(light_id))
			.execute(conn)
			.unwrap();
		return s > 0;
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
		_secret: String,
		_user_id: i32,
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
		_secret: String,
		_user_id: i32,
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
		_secret: String,
		_user_id: i32,
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

	pub fn update_device(
		light_id: Uuid,
		light_state: &LightState,
		db_conn: &mut PgConnection,
		_secret: String,
		_user_id: i32,
	) -> Light {
		let light_after_update = diesel::update(lights::table)
			.set((
				lights::is_on.eq(light_state.is_on),
				lights::brightness.eq(light_state.brightness),
				lights::rgb.eq(light_state.color),
			))
			.filter(lights::light_id.eq(light_id))
			.get_result::<Light>(db_conn);
		// todo implement error handling
		return light_after_update.unwrap();
	}
}
