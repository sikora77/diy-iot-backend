use crate::schema::devices;
use crate::schema::devices::dsl::devices as all_devices;
use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

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

#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceData {
	pub device_id: Uuid,
	pub brightness: Option<i32>,
	pub color: Option<i32>,
	pub is_on: Option<bool>,
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
