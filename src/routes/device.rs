use crate::db::Conn as DbConn;
use crate::models::{DeviceData, DeviceSignature, Light, LightState, Trait};

use crate::models::{Device, NewDevice};
use crate::utils::{self, create_coap_device};

use rocket_contrib::json::Json;
use serde_json::Value;

use tokio::runtime::Runtime;

use crate::constants::{NON_RGB_LIGHT, RGB_LIGHT};

use super::AuthUser;

// Returns the devices owned by the user
#[get("/devices")]
pub fn get_devices(mut conn: DbConn, user: AuthUser) -> Json<Value> {
	let devices = Device::get_devices_by_user(user.user_id, &mut conn);
	return Json(json! ({"status":200,"devices":devices}));
}

#[get("/full_devices")]
pub fn get_full_devices(mut conn: DbConn, user: AuthUser) -> Json<Value> {
	let lights = Light::get_full_device_data_by_user(user.user_id, &mut conn);
	return Json(json! ({"status":200,"lights":lights}));
}

#[post("/set_on", format = "application/json", data = "<device_data>")]
pub fn set_on(mut conn: DbConn, device_data: Json<DeviceData>, user: AuthUser) -> Json<Value> {
	let user_id = user.user_id;
	let device = Light::get_device_by_id(device_data.device_id, &mut conn);
	if device.is_none() {
		return Json(json!({"success":false,"error":"Device does not exist"}));
	}
	if device_data.is_on.is_none() {
		return Json(json!(
			{"success":false,"error":"Invalid device status"}
		));
	}
	let device = device.unwrap();
	if user_id != device.user_id {
		return Json(json!(
			{"success":false,"error":"You are not the owner of the device"}
		));
	}
	let rt = Runtime::new().unwrap();
	let light_state = LightState {
		brightness: device.brightness,
		color: device.rgb,
		is_on: device_data.is_on.unwrap(),
	};
	let update_result = Light::update_device_is_on(
		device.light_id,
		device_data.is_on.unwrap(),
		&mut conn,
		device.secret,
		device.user_id,
	);
	let resp = rt.block_on(utils::send_device_command(light_state, device.light_id));
	if resp.is_err() {
		return Json(json!(
			{"success":false,"error":resp.unwrap_err().to_string()}
		));
	}
	if &resp.unwrap().header.get_code() == "4.04" {
		return Json(json!(
			{"success":false,"error":"device not found"}
		));
	}
	//println!("{}", resp.unwrap().header.get_code());

	return Json(json!({"success":true}));
}

#[post("/set_color", format = "application/json", data = "<device_data>")]
pub fn set_color(mut conn: DbConn, device_data: Json<DeviceData>, user: AuthUser) -> Json<Value> {
	let user_id = user.user_id;
	let device = Light::get_device_by_id(device_data.device_id, &mut conn);
	if device.is_none() {
		return Json(json!({"success":false,"error":"Device does not exist"}));
	}
	if device_data.color.is_none() {
		return Json(json!(
			{"success":false,"error":"Invalid device color"}
		));
	}
	let device = device.unwrap();
	if user_id != device.user_id {
		return Json(json!(
			{"success":false,"error":"You are not the owner of the device"}
		));
	}
	let update_result = Light::update_device_brightness(
		device.light_id,
		device_data.color.unwrap(),
		&mut conn,
		device.secret,
		device.user_id,
	);
	let rt = Runtime::new().unwrap();
	let light_state = LightState {
		brightness: device.brightness,
		color: device_data.color.unwrap(),
		is_on: true,
	};
	let resp = rt.block_on(utils::send_device_command(light_state, device.light_id));
	if resp.is_err() {
		return Json(json!(
			{"success":false,"error":resp.unwrap_err().to_string()}
		));
	}
	if &resp.unwrap().header.get_code() == "4.04" {
		return Json(json!(
			{"success":false,"error":"device not found"}
		));
	}
	//println!("{}", resp.unwrap().header.get_code());

	return Json(json!({"success":true}));
}

#[post("/set_brightness", format = "application/json", data = "<device_data>")]
pub fn set_brightness(
	mut conn: DbConn,
	device_data: Json<DeviceData>,
	user: AuthUser,
) -> Json<Value> {
	let user_id = user.user_id;
	let device = Light::get_device_by_id(device_data.device_id, &mut conn);
	if device.is_none() {
		return Json(json!({"success":false,"error":"Device does not exist"}));
	}
	if device_data.brightness.is_none() {
		return Json(json!(
			{"success":false,"error":"Invalid device brightness"}
		));
	}
	let device = device.unwrap();
	if user_id != device.user_id {
		return Json(json!(
			{"success":false,"error":"You are not the owner of the device"}
		));
	}
	let update_result = Light::update_device_brightness(
		device.light_id,
		device_data.brightness.unwrap(),
		&mut conn,
		device.secret,
		device.user_id,
	);
	let rt = Runtime::new().unwrap();
	let light_state = LightState {
		brightness: device_data.brightness.unwrap(),
		color: device.rgb,
		is_on: true,
	};
	let resp = rt.block_on(utils::send_device_command(light_state, device.light_id));
	if resp.is_err() {
		return Json(json!(
			{"success":false,"error":resp.unwrap_err().to_string()}
		));
	}
	return Json(json!({"success":true}));
}

#[post("/register_device", format = "application/json", data = "<new_device>")]
pub fn register_device(
	mut conn: DbConn,
	new_device: Json<NewDevice>,
	user: AuthUser,
) -> Json<Value> {
	// Verifies that the device is signed by the private key
	let device_data = DeviceSignature {
		id: new_device.id,
		type_: new_device.type_.clone(),
	};
	let verified = utils::verify_secret(
		new_device.secret.clone(),
		serde_json::to_string(&device_data).unwrap(),
	);
	println!("{}", verified);
	if !verified {
		return Json(json!({"error":"failed to authenticate device"}));
	}
	// Sets the device traits used for google home integrartion
	let traits: Vec<Option<String>> =
		Trait::get_traits_for_device_type(new_device.type_.clone(), &mut conn)
			.iter()
			.map(|trait_| Some(trait_.trait_.clone()))
			.collect();

	let device = Device {
		id: new_device.id,
		user_id: user.user_id,
		type_: new_device.type_.clone(),
		internal_name: new_device.type_.clone() + new_device.id.to_string().as_str(),
		name: new_device.name.clone(),
		nicknames: vec![],
		traits: traits,
	};
	let matcher = match device.type_.as_str() {
		RGB_LIGHT => Some(device.type_.clone()),
		NON_RGB_LIGHT => Some(device.type_.clone()),
		_ => None,
	};
	if matcher.is_none() {
		return Json(json! ({"error":"invalid device type"}));
	}

	let transaction_status = conn.build_transaction().run(|local_conn| {
		let mut status = Light::insert_device(
			device.id,
			local_conn,
			new_device.secret.clone(),
			device.user_id,
		);
		println!("{}", status);
		if status {
			status = Device::insert_device(device, local_conn);
		}
		println!("{}", status);
		if matcher.unwrap().contains("light") {
			let rt = Runtime::new().unwrap();
			let coap_response = rt.block_on(create_coap_device(new_device.id, "lights"));
			if coap_response.is_err() {
				return Err(diesel::result::Error::RollbackTransaction);
			}
			if coap_response.unwrap().header.get_code() != "2.04" {
				return Err(diesel::result::Error::RollbackTransaction);
			}
		}
		if status {
			return Ok(());
		} else {
			return Err(diesel::result::Error::RollbackTransaction);
		}
	});

	return Json(json! ({"status":200,"result":transaction_status.is_ok()}));
}
