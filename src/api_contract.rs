use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleContract {
    pub server_time_ms: u64,
    pub cycle_start_ms: u64,
    pub work_minutes: u32,
    pub break_minutes: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastMessage {
    pub preset_id: String,
    pub label: String,
    pub message: String,
    pub sender: String,
    pub sent_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MessagesResponse {
    pub messages: Vec<BroadcastMessage>,
}
