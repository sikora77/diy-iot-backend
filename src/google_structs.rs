use std::collections::HashMap;

use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct GoogleRequest {
	pub requestId: String,
	pub inputs: Vec<Input>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GoogleResponse {
	pub requestId: String,
	pub payload: Payload,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Payload {
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
#[derive(Clone, Serialize, Deserialize)]
pub struct Input {
	pub intent: String,
	pub payload: QueryRequestPayload,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceData {
	pub id: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct QueryRequestPayload {
	pub devices: Vec<DeviceData>,
}
