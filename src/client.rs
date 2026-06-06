use crate::api_contract::{BroadcastMessage, MessagesResponse, ScheduleContract};
use crate::timer::{TimerState, state_from_schedule};
use std::time::Duration;

const DEFAULT_API_BASE: &str = "https://nomad-timer.hyeon.space/api";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServerClock {
    server_time_ms: u64,
    local_sample_ms: u64,
    cycle_start_ms: u64,
}

impl ServerClock {
    pub fn from_sample(server_time_ms: u64, local_sample_ms: u64, cycle_start_ms: u64) -> Self {
        Self {
            server_time_ms,
            local_sample_ms,
            cycle_start_ms,
        }
    }

    pub fn server_now_ms(self, local_now_ms: u64) -> u64 {
        self.server_time_ms
            .saturating_add(local_now_ms.saturating_sub(self.local_sample_ms))
    }

    pub fn timer_state(self, local_now_ms: u64) -> TimerState {
        state_from_schedule(self.server_now_ms(local_now_ms), self.cycle_start_ms)
    }
}

pub fn parse_schedule_response(body: &str) -> serde_json::Result<ScheduleContract> {
    serde_json::from_str(body)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiBase {
    base_url: String,
}

impl ApiBase {
    pub fn parse(raw: &str) -> std::io::Result<Self> {
        let trimmed = raw.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "API base URL is empty",
            ));
        }
        let base_url = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_owned()
        } else {
            format!("http://{trimmed}")
        };
        Ok(Self { base_url })
    }

    pub fn production() -> Self {
        Self {
            base_url: DEFAULT_API_BASE.to_owned(),
        }
    }

    pub fn url(&self, path: &str) -> String {
        let clean_path = path.trim_start_matches('/');
        format!("{}/{clean_path}", self.base_url)
    }
}

pub fn fetch_server_clock(server: &str, local_sample_ms: u64) -> std::io::Result<ServerClock> {
    let base = ApiBase::parse(server)?;
    let body = get_http_json(&base.url("/schedule"))?;
    let schedule = parse_schedule_response(&body)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    Ok(ServerClock::from_sample(
        schedule.server_time_ms,
        local_sample_ms,
        schedule.cycle_start_ms,
    ))
}

pub fn parse_messages_response(body: &str) -> serde_json::Result<Vec<BroadcastMessage>> {
    serde_json::from_str::<MessagesResponse>(body).map(|response| response.messages)
}

pub fn fetch_recent_messages(server: &str) -> std::io::Result<Vec<BroadcastMessage>> {
    let base = ApiBase::parse(server)?;
    let body = get_http_json(&base.url("/messages"))?;
    parse_messages_response(&body)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

pub fn post_preset_broadcast(
    server: &str,
    preset_id: &str,
    sender: &str,
) -> std::io::Result<BroadcastMessage> {
    let base = ApiBase::parse(server)?;
    let response = post_json(&base.url("/broadcast"), &broadcast_payload(preset_id, sender))?;
    serde_json::from_str::<BroadcastMessage>(&response)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

pub fn broadcast_payload(preset_id: &str, sender: &str) -> String {
    serde_json::json!({
        "presetId": preset_id,
        "sender": sender,
    })
    .to_string()
}

pub fn default_api_base() -> &'static str {
    DEFAULT_API_BASE
}

fn get_http_json(url: &str) -> std::io::Result<String> {
    let agent = http_agent();
    agent
        .get(url)
        .call()
        .and_then(|mut response| response.body_mut().read_to_string())
        .map_err(http_error)
}

fn post_json(url: &str, payload: &str) -> std::io::Result<String> {
    let agent = http_agent();
    agent
        .post(url)
        .header("Content-Type", "application/json")
        .send(payload)
        .and_then(|mut response| response.body_mut().read_to_string())
        .map_err(http_error)
}

fn http_agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(3)))
        .timeout_connect(Some(Duration::from_secs(2)))
        .timeout_recv_body(Some(Duration::from_secs(2)))
        .build();
    config.into()
}

fn http_error(error: ureq::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_schedule_response_into_server_clock() {
        // Given: the server schedule endpoint returns the global UTC contract.
        let body = r#"{"serverTimeMs":1780655673927,"cycleStartMs":1780653600000,"workMinutes":50,"breakMinutes":10}"#;

        // When: the widget parses the schedule response.
        let schedule = parse_schedule_response(body).expect("schedule should parse");

        // Then: it has the server timestamp and cycle anchor needed for synced timing.
        assert_eq!(schedule.server_time_ms, 1_780_655_673_927);
        assert_eq!(schedule.cycle_start_ms, 1_780_653_600_000);
    }

    #[test]
    fn server_clock_estimates_current_server_time_from_local_offset() {
        // Given: server time was sampled from /schedule at a known local instant.
        let clock = ServerClock::from_sample(2_000, 1_000, 0);

        // When: local time advances.
        let server_now = clock.server_now_ms(1_250);

        // Then: the estimated server time preserves the server/client offset.
        assert_eq!(server_now, 2_250);
    }

    #[test]
    fn api_base_builds_https_schedule_and_message_urls() {
        // Given: the production API base includes scheme, host, and /api prefix.
        let base = ApiBase::parse("https://nomad-timer.hyeon.space/api")
            .expect("production API base should parse");

        // When: endpoint URLs are built.
        let schedule_url = base.url("/schedule");
        let messages_url = base.url("/messages");

        // Then: requests target the deployed API prefix instead of raw localhost TCP.
        assert_eq!(
            schedule_url,
            "https://nomad-timer.hyeon.space/api/schedule"
        );
        assert_eq!(
            messages_url,
            "https://nomad-timer.hyeon.space/api/messages"
        );
    }

    #[test]
    fn parses_messages_response_for_widget_chat_feed() {
        // Given: the server returns recent preset broadcasts.
        let body = r#"{"messages":[{"presetId":"stretch","label":"기지개","message":"어깨 펴고 기지개 한 번!","sender":"alice","sentAtMs":1780679000000}]}"#;

        // When: the widget parses the response.
        let messages = parse_messages_response(body).expect("messages should parse");

        // Then: the latest preset chat can be shown in the widget.
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].sender, "alice");
        assert_eq!(messages[0].label, "기지개");
    }

    #[test]
    fn broadcast_payload_uses_camel_case_preset_contract() {
        // Given: the Windows widget sends a prepared preset reaction.
        let payload = broadcast_payload("water", "windows-widget");

        // When: the JSON body is inspected.
        let value: serde_json::Value =
            serde_json::from_str(&payload).expect("payload should be valid JSON");

        // Then: it matches the server's public preset contract.
        assert_eq!(value["presetId"], "water");
        assert_eq!(value["sender"], "windows-widget");
    }
}
