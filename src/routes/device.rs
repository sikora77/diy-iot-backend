use crate::db::Conn as DbConn;
use crate::models::{light::Light, light::LightState, light::Trait};

use crate::models::device::{self, Device, DeviceData, DeviceSignature, NewDevice};
use crate::utils::{self, create_coap_device, remove_coap_device};

use rocket_contrib::json::Json;
use serde_json::Value;

use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::constants::{NON_RGB_LIGHT, RGB_LIGHT};

use super::AuthUser;

// Returns the devices owned by the user
#[get("/devices")]
pub fn get_devices(mut conn: DbConn, user: AuthUser) -> Json<Value> {
    let devices = Device::get_devices_by_user(user.user_id, &mut conn);
    return Json(json!({"status":200,"devices":devices}));
}

#[get("/full_devices")]
pub fn get_full_devices(mut conn: DbConn, user: AuthUser) -> Json<Value> {
    let lights = Light::get_full_device_data_by_user(user.user_id, &mut conn);
    return Json(json!({"status":200,"lights":lights}));
}

fn update_device(user_id: i32, device_data: DeviceData, mut db_conn: DbConn) -> Json<Value> {
    let device = Light::get_device_by_id(device_data.device_id, &mut db_conn);
    if device.is_none() {
        return Json(json!({"success":false,"error":"Device does not exist"}));
    }
    let device = device.unwrap();
    if user_id != device.user_id {
        return Json(json!(
			{"success":false,"error":"You are not the owner of the device"}
		));
    }
    let mut light_state = LightState {
        brightness: device.brightness,
        color: device.rgb,
        is_on: device.is_on,
        removed:false,
    };
    light_state.brightness = match device_data.brightness {
        Some(brightness) => brightness,
        None => device.brightness,
    };
    light_state.is_on = match device_data.is_on {
        Some(on) => on,
        None => device.is_on,
    };
    light_state.color = match device_data.color {
        Some(color) => color,
        None => device.rgb,
    };
    let rt = Runtime::new().unwrap();
    Light::update_device(
        device.light_id,
        &light_state,
        &mut db_conn,
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

    return Json(json!({"success":true}));
}

#[post("/set_on", format = "application/json", data = "<device_data>")]
pub fn set_on(conn: DbConn, device_data: Json<DeviceData>, user: AuthUser) -> Json<Value> {
    let user_id = user.user_id;
    return update_device(user_id, device_data.0, conn);
}

#[post("/set_color", format = "application/json", data = "<device_data>")]
pub fn set_color(conn: DbConn, device_data: Json<DeviceData>, user: AuthUser) -> Json<Value> {
    let user_id = user.user_id;
    return update_device(user_id, device_data.0, conn);
}

#[post("/set_brightness", format = "application/json", data = "<device_data>")]
pub fn set_brightness(conn: DbConn, device_data: Json<DeviceData>, user: AuthUser) -> Json<Value> {
    let user_id = user.user_id;
    return update_device(user_id, device_data.0, conn);
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
    // TODO actually match the errors
    let verified = utils::verify_secret(
        &new_device.secret,
        serde_json::to_string(&device_data).unwrap(),
    );
    if verified.is_err() {
        return Json(json!({"error":"signature is wrong"}));
    }
    let verified = verified.unwrap();
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
        return Json(json!({"error":"invalid device type"}));
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

    return Json(json!({"status":200,"result":transaction_status.is_ok()}));
}

#[get("/is_online/<device_id>", format = "application/json")]
pub fn check_device_online(mut conn: DbConn, device_id: String, user: AuthUser) -> Json<Value> {
    let user_id = user.user_id;
    let device = Light::get_device_by_id(Uuid::parse_str(&device_id).unwrap(), &mut conn);
    if device.is_none() {
        return Json(json!({"success":false,"error":"Device does not exist"}));
    }
    let device = device.unwrap();
    if user_id != device.user_id {
        return Json(json!(
			{"success":false,"error":"You are not the owner of the device"}
		));
    }
    let rt = Runtime::new().unwrap();
    let resp = rt.block_on(utils::check_device_online(device_id));
    return match resp {
        Some(value) => Json(json!({"isOnline":value,"success":true})),
        None => Json(json!({"success":false})),
    };
}

#[derive(Deserialize)]
struct RenameData {
    device_id: Uuid,
    new_name: String,
}

#[allow(private_interfaces)]
#[post("/rename_device", format = "application/json", data = "<device_data>")]
pub fn rename_device(
    mut conn: DbConn,
    device_data: Json<RenameData>,
    user: AuthUser,
) -> Json<Value> {
    let user_id = user.user_id;
    let device_id = device_data.device_id;
    let device = Device::get_device_by_id(device_id, &mut conn);
    match device {
        Some(dev) => {
            if dev.user_id != user_id {
                return Json(
                    json!({"success":false,"error":"You are not the owner of the device"}),
                );
            }
            Device::update_device_name(dev.id, &device_data.new_name, &mut conn);
            return Json(json!({"success":true,"new_name":device_data.new_name}));
        }
        None => {
            return Json(json!({"success":false,"error":"Device does not exist"}));
        }
    }
    // return Json(json!({}));
}

#[derive(Deserialize)]
struct DeleteData {
    device_id: Uuid,
}

#[allow(private_interfaces)]
#[post("/remove_device", format = "application/json", data = "<device_data>")]
pub fn remove_device(
    mut conn: DbConn,
    device_data: Json<DeleteData>,
    user: AuthUser,
) -> Json<Value> {
    let user_id = user.user_id;
    let device_id = device_data.device_id;
    let device = Device::get_device_by_id(device_id, &mut conn);
    let light = Light::get_device_by_id(device_id, &mut conn);
    let device_status = match device {
        Some(dev) => {
            if dev.user_id != user_id {
                Some(Json(
                    json!({"success":false,"error":"You are not the owner of the device"}),
                ))
            } else {
                if Device::remove_device(device_id, &mut conn) {
                    None
                } else {
                    Some(Json(json!({"success":false,"error":"something went wrong"})))
                }
            }
        }
        None => Some(Json(json!({"success":false,"error":"Device does not exist"}))),
    };
    if device_status.is_some() {
        return device_status.unwrap();
    }
    let mut light_id: Uuid;
    let light_status = match light {
        Some(light_dev) => {
            light_id = light_dev.light_id;
            if light_dev.user_id != user_id {
                Some(Json(
                    json!({"success":false,"error":"You are not the owner of the device"})
                ))
            } else {
                if Light::remove_device(device_id, &mut conn) {
                    None
                } else {
                    Some(Json(json!({"success":false,"error":"something went wrong"})))
                }
            }
        }
        None => { return Json(json!({"success":false,"error":"Device does not exist"})) }
    };
    if light_status.is_some() {
        return light_status.unwrap();
    }
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // TODO Check this later
        remove_coap_device(light_id).await;
    });
    Json(json!({"success":true}))
}
