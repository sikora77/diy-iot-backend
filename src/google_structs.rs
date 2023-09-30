#![allow(non_snake_case)]

use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;
//Google request structs
#[derive(Clone, Serialize, Deserialize)]
pub struct GoogleRequest {
	pub requestId: String,
	pub inputs: Vec<Input>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Input {
	pub intent: String,
	pub payload: Option<Payload>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Payload {
	pub devices: Option<Vec<DeviceData>>,
	pub commands: Option<Vec<Command>>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DeviceData {
	pub id: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Command {
	pub devices: Vec<DeviceData>,
	pub execution: Vec<Execution>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Execution {
	pub command: String,
	pub params: Params,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Params {
	pub on: Option<bool>,
	pub color: Option<Color>,
	pub brightness: Option<i32>,
}
//Structs used to respond to SYNC requests
#[derive(Clone, Serialize, Deserialize)]
pub struct GoogleResponse<T> {
	pub requestId: String,
	pub payload: T,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SyncPayload {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub agentUserId: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub devices: Option<Vec<GoogleDevice>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub errorCode: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct GoogleDevice {
	pub id: Uuid,
	#[serde(rename(serialize = "type"))]
	pub type_: String,
	pub traits: Vec<String>,
	pub name: NameStruct,
	pub willReportState: bool,
	pub attributes: DeviceAttributes,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct NameStruct {
	pub defaultNames: Vec<String>,
	pub name: String,
	pub nicknames: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceAttributes {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub colorModel: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub colorTemperatureRange: Option<HashMap<String, i32>>,
}
// Structs used to respond to QUERY requests

#[derive(Clone, Serialize, Deserialize)]
pub struct LightState {
	pub online: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub on: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub brightness: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub color: Option<Color>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct HeaterState {
	pub online: bool,
	pub on: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub temp: Option<i32>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum States {
	Light(LightState),
	Heater(HeaterState),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Color {
	pub spectrumRGB: i32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct QueryPayload {
	pub devices: HashMap<String, States>,
}
// Structs used to respond to EXECUTE REQUESTS
#[derive(Clone, Serialize, Deserialize)]
pub struct ExecutePayload {
	pub commands: Vec<CommandsResponse>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct CommandsResponse {
	pub ids: Vec<String>,
	pub status: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub states: Option<States>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub errorCode: Option<String>,
}
