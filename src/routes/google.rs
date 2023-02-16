use crate::db::Conn as DbConn;
use crate::google_routes::google_structs::{
	DeviceAttributes, GoogleDevice, GoogleResponse, NameStruct, SyncPayload,
};
use oxide_auth_rocket::{OAuthFailure, OAuthRequest, OAuthResponse};
use rocket::http::Status;
use rocket::response::Responder;
use rocket::{http::ContentType, Response, State};

use std::collections::HashMap;
use std::hash::Hash;
use std::io;

#[path = "../constants.rs"]
mod constants;
#[path = "../google_structs.rs"]
mod google_structs;
#[path = "../jwt_issuer.rs"]
mod jwt_issuer;
#[path = "../utils.rs"]
mod utils;
use crate::models::{Device, Light};
use crate::oath_routes::MyState;

use rocket_contrib::json::Json;

use self::constants::{NON_RGB_LIGHT, RGB_LIGHT};
use self::google_structs::{Color, GoogleRequest, HeaterState, LightState, QueryPayload, States};

#[post("/fullfilment", format = "application/json", data = "<request>")]
pub fn fullfilment<'r>(
	oauth: OAuthRequest<'r>,
	state: State<MyState>,
	request: Json<GoogleRequest>,
	conn: DbConn,
) -> impl Responder<'r> {
	let protect = state
		.endpoint()
		.with_scopes(vec!["default-scope".parse().unwrap()])
		.resource_flow()
		.execute(oauth);
	match protect {
		Ok(grant) => {
			println!("{}", grant.owner_id);
			let user_id = grant.owner_id.parse::<i32>().unwrap();
			let request_id = request.requestId.clone();

			let input = request.inputs.first().unwrap();
			if input.intent == "action.devices.SYNC" {
				let response = handle_sync(request_id, user_id, conn);
				Ok(Json(
					json! ({"requestId":response.requestId,"payload":response.payload}),
				))
			} else if input.intent == "action.devices.QUERY" {
				let response = handle_query(request_id, user_id, conn);
				Ok(Json(
					json! ({"requestId":response.requestId,"payload":response.payload}),
				))
			} else {
				let response = GoogleResponse {
					requestId: request_id.clone(),
					payload: SyncPayload {
						agentUserId: None,
						devices: None,
						errorCode: Some("notSupported".to_string()),
						status: Some("ERROR".to_string()),
					},
				};
				Ok(Json(
					json! ({"requestId":response.requestId,"payload":response.payload}),
				))
			}
		}
		Err(Ok(response)) => {
			let error: OAuthResponse = Response::build_from(response.into())
				.header(ContentType::HTML)
				.status(Status::Unauthorized)
				.sized_body(io::Cursor::new("".to_string()))
				.finalize()
				.into();
			Err(Ok(error))
		}
		Err(Err(err)) => Err(Err(err.pack::<OAuthFailure>())),
	}
}

fn handle_sync(request_id: String, user_id: i32, conn: DbConn) -> GoogleResponse<SyncPayload> {
	let devices: Vec<GoogleDevice> = Device::get_devices_by_user(user_id, &conn)
		.iter()
		.filter_map(|device| {
			let traits: Vec<String> = device
				.traits
				.iter()
				.filter(|trait_| trait_.is_some())
				.map(|trait_| return trait_.clone().unwrap())
				.collect();
			return match device.type_.as_str().clone() {
				NON_RGB_LIGHT => {
					let device_type = "action.devices.types.LIGHT".to_string();
					Some(GoogleDevice {
						id: device.id,
						type_: device_type,
						traits: traits,
						name: NameStruct {
							defaultNames: vec![device.internal_name.clone()],
							name: device.name.clone(),
							nicknames: vec![],
						},
						willReportState: false,
						attributes: DeviceAttributes {
							colorModel: None,
							colorTemperatureRange: None,
						},
					})
				}
				RGB_LIGHT => {
					let device_type = "action.devices.types.LIGHT".to_string();
					Some(GoogleDevice {
						id: device.id,
						type_: device_type,
						traits: traits,
						name: NameStruct {
							defaultNames: vec![device.internal_name.clone()],
							name: device.name.clone(),
							nicknames: vec![],
						},
						willReportState: false,
						attributes: DeviceAttributes {
							colorModel: Some("rgb".to_string()),
							colorTemperatureRange: None,
						},
					})
				}
				_ => None,
			};
		})
		.collect();
	GoogleResponse {
		requestId: request_id.clone(),
		payload: SyncPayload {
			agentUserId: Some(user_id.to_string()),
			devices: Some(devices),
			errorCode: None,
			status: None,
		},
	}
}

fn handle_query(request_id: String, user_id: i32, conn: DbConn) -> GoogleResponse<QueryPayload> {
	let mut devices = HashMap::new();
	for device in Light::get_devices_by_user(user_id, &conn).iter() {
		let state = LightState {
			status: Some("SUCCESS".to_string()),
			online: true,
			on: device.is_on,
			brightness: Some(device.brightness),
			color: Some(Color {
				spectrumRGB: device.rgb,
			}),
		};
		devices.insert(device.light_id.to_string(), States::Light(state));
	}
	let test = HeaterState {
		status: Some("SUCCESS".to_string()),
		online: true,
		on: true,
		temp: Some(100),
	};
	devices.insert("2".to_string(), States::Heater(test));
	GoogleResponse {
		requestId: request_id.clone(),
		payload: QueryPayload { devices: devices },
	}
}
