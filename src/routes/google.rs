use crate::db::Conn as DbConn;
use crate::google_routes::google_structs::{
	DeviceAttributes, GoogleDevice, GoogleResponse, NameStruct, SyncPayload,
};
use coap_client::{ClientOptions, HostOptions, RequestOptions, TokioClient};
use oxide_auth_rocket::{OAuthFailure, OAuthRequest, OAuthResponse};
use rocket::http::Status;
use rocket::response::Responder;
use rocket::{http::ContentType, Response, State};
use std::collections::HashMap;
use std::io;
use tokio::runtime::Runtime;

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
use self::google_structs::{
	Color, CommandsResponse, DeviceData, ExecutePayload, GoogleRequest, LightState, QueryPayload,
	States,
};

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
			match input.intent.as_str() {
				"action.devices.SYNC" => {
					let response = handle_sync(request_id, user_id, conn);
					Ok(Json(
						json! ({"requestId":response.requestId,"payload":response.payload}),
					))
				}
				"action.devices.QUERY" => {
					let response = handle_query(request_id, user_id, conn);
					Ok(Json(
						json! ({"requestId":response.requestId,"payload":response.payload}),
					))
				}
				"action.devices.EXECUTE" => {
					let response = handle_execute(request.into_inner(), user_id, conn);
					Ok(Json(
						json! ({"requestId":response.requestId,"payload":response.payload}),
					))
				}
				_ => {
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

fn handle_sync(request_id: String, user_id: i32, mut conn: DbConn) -> GoogleResponse<SyncPayload> {
	let devices: Vec<GoogleDevice> = Device::get_devices_by_user(user_id, &mut conn)
		.iter()
		.filter_map(|device| {
			let traits: Vec<String> = device
				.traits
				.iter()
				.filter(|trait_| trait_.is_some())
				.map(|trait_| return trait_.clone().unwrap())
				.collect();
			return match device.type_.as_str() {
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

fn handle_query(
	request_id: String,
	user_id: i32,
	mut conn: DbConn,
) -> GoogleResponse<QueryPayload> {
	println!("{}", user_id);
	let mut devices = HashMap::new();
	for device in Light::get_devices_by_user(user_id, &mut conn).iter() {
		let state = LightState {
			online: true,
			on: Some(device.is_on),
			brightness: Some(device.brightness),
			color: Some(Color {
				spectrumRGB: device.rgb,
			}),
		};
		devices.insert(device.light_id.to_string(), States::Light(state));
	}
	GoogleResponse {
		requestId: request_id.clone(),
		payload: QueryPayload { devices: devices },
	}
}
fn handle_execute(
	request: GoogleRequest,
	user_id: i32,
	mut conn: DbConn,
) -> GoogleResponse<ExecutePayload> {
	let mut command_outputs: Vec<CommandsResponse> = vec![];
	let commands = request
		.inputs
		.first()
		.unwrap()
		.payload
		.as_ref()
		.unwrap()
		.commands
		.as_ref()
		.unwrap()
		.iter();
	let user_devices = Device::get_devices_by_user(user_id, &mut conn);
	for command in commands {
		for execution in command.execution.iter() {
			match execution.command.as_str() {
				"action.devices.commands.OnOff" => {
					// let output = task::block_in_place(|| handle_on_off(command.devices.as_ref()));
					let rt = Runtime::new().unwrap();
					let mut devices = rt.block_on(handle_on_off(
						command.devices.as_ref(),
						user_devices.as_ref(),
					));
					command_outputs.append(&mut devices);
				}
				_ => {}
			}
		}
	}
	GoogleResponse {
		requestId: request.requestId,
		payload: ExecutePayload {
			commands: command_outputs,
		},
	}
}

async fn handle_on_off(
	devices: &Vec<DeviceData>,
	user_devices: &Vec<Device>,
) -> Vec<CommandsResponse> {
	let mut success = CommandsResponse {
		ids: vec![],
		status: "SUCCESS".to_string(),
		states: Some(States::Light(LightState {
			online: true,
			on: Some(true),
			brightness: None,
			color: None,
		})),
		errorCode: None,
	};
	let mut failure = CommandsResponse {
		ids: vec![],
		status: "ERROR".to_string(),
		states: None,
		errorCode: Some("deviceOffline".to_string()),
	};
	let mut host_opts = HostOptions::default();
	host_opts.host = "192.168.33.108".to_string();
	host_opts.port = 5683;
	let mut req_opts = RequestOptions::default();
	req_opts.non_confirmable = false;
	let mut client = TokioClient::connect(host_opts, &ClientOptions::default()).await;
	let device_map: Vec<String> = user_devices
		.iter()
		.map(|device| device.id.to_string())
		.collect();
	if client.is_err() {
	} else {
		for device in devices.iter() {
			println!("{:?}", device);
			if !device_map.contains(&device.id) {
			} else {
				let resp = client
					.as_mut()
					.unwrap()
					.put(format!("/devices/{}", device.id).as_str(), None, &req_opts)
					.await;
				//TODO handle errors
				if resp.is_err() {
					//something went wrong when sending the request
				} else {
					// Check the response
					if resp.unwrap_or_default().len() != 0 {
						failure.ids.push(device.id.clone());
					} else {
						// println!("{}", resp_payload.unwrap());
						success.ids.push(device.id.clone());
					}
				}
			}
		}
	}
	return vec![success, failure];
}
