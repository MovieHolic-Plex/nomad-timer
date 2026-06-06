use crate::api_contract::{BroadcastMessage, MessagesResponse, ScheduleContract};
use crate::presets::{Preset, find_preset};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_MESSAGES: usize = 20;
const MAX_BODY_BYTES: usize = 512;
const MAX_REQUEST_BYTES: usize = 2048;
const MAX_SENDER_BYTES: usize = 32;
const HOUR_MS: u64 = 60 * 60 * 1000;
const READ_CHUNK_BYTES: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub ok: bool,
    pub reason: &'static str,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BroadcastError {
    UnknownPreset,
    InvalidSender,
}

#[derive(Default)]
pub struct BroadcastStore {
    messages: VecDeque<BroadcastMessage>,
}

impl BroadcastStore {
    pub fn post_preset(
        &mut self,
        preset_id: &str,
        sender: &str,
        sent_at_ms: u64,
    ) -> Result<BroadcastMessage, BroadcastError> {
        let clean_sender = sender.trim();
        if !is_safe_sender(clean_sender) {
            return Err(BroadcastError::InvalidSender);
        }
        let preset = find_preset(preset_id).ok_or(BroadcastError::UnknownPreset)?;
        let message = message_from_preset(preset, clean_sender, sent_at_ms);
        self.messages.push_front(message.clone());
        while self.messages.len() > MAX_MESSAGES {
            let _ = self.messages.pop_back();
        }
        Ok(message)
    }

    pub fn recent_messages(&self) -> Vec<BroadcastMessage> {
        self.messages.iter().cloned().collect()
    }
}

pub fn schedule_contract(server_time_ms: u64) -> ScheduleContract {
    ScheduleContract {
        server_time_ms,
        cycle_start_ms: server_time_ms - (server_time_ms % HOUR_MS),
        work_minutes: 50,
        break_minutes: 10,
    }
}

pub fn current_epoch_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        Err(_) => 0,
    }
}

pub fn run_server(addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    let store = Arc::new(Mutex::new(BroadcastStore::default()));
    for incoming in listener.incoming() {
        let mut stream = incoming?;
        let shared_store = Arc::clone(&store);
        handle_stream(&mut stream, &shared_store)?;
    }
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
struct HttpRequest {
    first_line: String,
    body: String,
}

fn message_from_preset(preset: Preset, sender: &str, sent_at_ms: u64) -> BroadcastMessage {
    BroadcastMessage {
        preset_id: preset.id.to_owned(),
        label: preset.label.to_owned(),
        message: preset.message.to_owned(),
        sender: sender.to_owned(),
        sent_at_ms,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BroadcastRequest {
    preset_id: String,
    sender: String,
}

fn handle_stream(
    stream: &mut TcpStream,
    store: &Arc<Mutex<BroadcastStore>>,
) -> std::io::Result<()> {
    let request = read_http_request(stream)?;
    let method_path = request_method_path(&request.first_line);
    let body = request.body.as_str();

    match method_path {
        Some(("GET", "/health")) => {
            write_json(stream, 200, &HealthResponse { ok: true })
        }
        Some(("GET", "/schedule")) => {
            write_json(stream, 200, &schedule_contract(current_epoch_ms()))
        }
        Some(("GET", "/messages")) => {
            let messages = match store.lock() {
                Ok(guard) => guard.recent_messages(),
                Err(_) => Vec::new(),
            };
            write_json(stream, 200, &MessagesResponse { messages })
        }
        Some(("POST", "/broadcast")) => {
            let parsed = serde_json::from_str::<BroadcastRequest>(body);
            let result = match parsed {
                Ok(payload) => match store.lock() {
                    Ok(mut guard) => {
                        guard.post_preset(&payload.preset_id, &payload.sender, current_epoch_ms())
                    }
                    Err(_) => Err(BroadcastError::UnknownPreset),
                },
                Err(_) => {
                    return write_error(stream, "invalidJson");
                }
            };
            match result {
                Ok(message) => write_json(stream, 201, &message),
                Err(BroadcastError::UnknownPreset) => write_error(stream, "unknownPreset"),
                Err(BroadcastError::InvalidSender) => write_error(stream, "invalidSender"),
            }
        }
        _ => write_json(
            stream,
            404,
            &ErrorResponse {
                ok: false,
                reason: "notFound",
            },
        ),
    }
}

fn request_method_path(first_line: &str) -> Option<(&str, &str)> {
    let mut parts = first_line.split_whitespace();
    let method = parts.next()?;
    let path = normalize_path(parts.next()?);
    Some((method, path))
}

fn normalize_path(path: &str) -> &str {
    path.strip_prefix("/api")
        .filter(|suffix| suffix.starts_with('/'))
        .unwrap_or(path)
}

fn write_error(stream: &mut TcpStream, reason: &'static str) -> std::io::Result<()> {
    write_json(stream, 400, &ErrorResponse { ok: false, reason })
}

fn read_http_request(reader: &mut impl Read) -> std::io::Result<HttpRequest> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; READ_CHUNK_BYTES];
    let mut expected_len = None;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "request is too large",
            ));
        }
        if let Some((head_len, content_len)) = request_lengths(&bytes) {
            if content_len > MAX_BODY_BYTES || head_len + content_len > MAX_REQUEST_BYTES {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "request body is too large",
                ));
            }
            expected_len = Some(head_len + content_len);
        }
        if let Some(total_len) = expected_len
            && bytes.len() >= total_len
        {
            break;
        }
    }

    let request = String::from_utf8_lossy(&bytes);
    let mut parts = request.splitn(2, "\r\n\r\n");
    let head = parts.next().unwrap_or_default();
    let body = parts.next().unwrap_or_default();
    let first_line = head.lines().next().unwrap_or_default().to_owned();
    Ok(HttpRequest {
        first_line,
        body: body.to_owned(),
    })
}

fn request_lengths(bytes: &[u8]) -> Option<(usize, usize)> {
    let marker = b"\r\n\r\n";
    let head_end = bytes
        .windows(marker.len())
        .position(|window| window == marker)?
        + marker.len();
    let head = String::from_utf8_lossy(&bytes[..head_end]);
    let content_len = head
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find_map(|(name, value)| {
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);
    Some((head_end, content_len))
}

fn is_safe_sender(sender: &str) -> bool {
    !sender.is_empty()
        && sender.len() <= MAX_SENDER_BYTES
        && sender
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn write_json<T: Serialize>(stream: &mut TcpStream, status: u16, body: &T) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    let body = serde_json::to_string(body)?;
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_endpoint_returns_utc_cycle_contract() {
        // Given: a fixed server epoch inside a UTC hour.
        let now_ms = 1_780_632_345_000;

        // When: the schedule response is built.
        let response = schedule_contract(now_ms);

        // Then: every client receives the same absolute 50/10 cycle contract.
        assert_eq!(response.work_minutes, 50);
        assert_eq!(response.break_minutes, 10);
        assert_eq!(response.server_time_ms, now_ms);
        assert!(
            response.cycle_start_ms <= now_ms,
            "cycle start must not be in the future"
        );
        assert_eq!(
            response.cycle_start_ms % (60 * 60 * 1000),
            0,
            "cycle must be anchored to the UTC hour"
        );
    }

    #[test]
    fn preset_broadcast_round_trips_and_rejects_unknown_preset() {
        // Given: a fresh in-memory broadcast store.
        let mut store = BroadcastStore::default();

        // When: a known preset is broadcast.
        let accepted = store.post_preset("rest-start", "tester", 1_780_632_345_000);

        // Then: the message is stored and visible to later clients.
        assert!(accepted.is_ok(), "known preset should be accepted");
        assert_eq!(
            accepted
                .expect("accepted preset should return its broadcast")
                .label,
            "쉬러가요"
        );
        let recent = store.recent_messages();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].preset_id, "rest-start");

        // When: an unknown preset is submitted.
        let rejected = store.post_preset("free-text-is-not-allowed", "tester", 1_780_632_346_000);

        // Then: arbitrary chat cannot be broadcast.
        assert!(rejected.is_err(), "unknown presets must be rejected");
    }

    #[test]
    fn http_reader_collects_split_post_body() {
        // Given: a valid broadcast POST whose body arrives after the first read.
        let request = b"POST /broadcast HTTP/1.1\r\nContent-Length: 44\r\n\r\n{\"presetId\":\"rest-start\",\"sender\":\"qa-user\"}";
        let mut reader = ChunkedReader::new(request, 24);

        // When: the complete HTTP request is read from a stream-like source.
        let parsed = read_http_request(&mut reader).expect("split request should parse");

        // Then: the body is complete and can be accepted by the broadcast handler.
        assert_eq!(
            parsed.body,
            "{\"presetId\":\"rest-start\",\"sender\":\"qa-user\"}"
        );
    }

    #[test]
    fn http_reader_rejects_oversized_broadcast_body() {
        // Given: a request declares a body larger than the broadcast endpoint accepts.
        let request = b"POST /broadcast HTTP/1.1\r\nContent-Length: 513\r\n\r\n{}";
        let mut reader = ChunkedReader::new(request, 128);

        // When: the request is read.
        let parsed = read_http_request(&mut reader);

        // Then: the server refuses the oversized request before parsing JSON.
        assert!(parsed.is_err(), "oversized requests must be rejected");
    }

    #[test]
    fn preset_broadcast_rejects_untrusted_sender_names() {
        // Given: a fresh store accepts only compact client identifiers.
        let mut store = BroadcastStore::default();

        // When: a sender tries to submit display text instead of an identifier.
        let rejected = store.post_preset("water", "very long sender name with spaces", 1);

        // Then: the server rejects it instead of reflecting arbitrary sender text.
        assert!(rejected.is_err(), "sender names must be bounded identifiers");
    }

    #[test]
    fn api_prefixed_routes_match_root_routes() {
        // Given: local QA may call the server directly with the deployed /api prefix.
        let schedule = request_method_path("GET /api/schedule HTTP/1.1");
        let broadcast = request_method_path("POST /api/broadcast HTTP/1.1");
        let root = request_method_path("GET /messages HTTP/1.1");

        // When: HTTP first lines are normalized.
        let schedule_path = schedule.expect("schedule route should parse");
        let broadcast_path = broadcast.expect("broadcast route should parse");
        let root_path = root.expect("root route should parse");

        // Then: /api-prefixed routes hit the same handlers as root routes.
        assert_eq!(schedule_path, ("GET", "/schedule"));
        assert_eq!(broadcast_path, ("POST", "/broadcast"));
        assert_eq!(root_path, ("GET", "/messages"));
    }

    struct ChunkedReader<'a> {
        bytes: &'a [u8],
        chunk_size: usize,
        offset: usize,
    }

    impl<'a> ChunkedReader<'a> {
        fn new(bytes: &'a [u8], chunk_size: usize) -> Self {
            Self {
                bytes,
                chunk_size,
                offset: 0,
            }
        }
    }

    impl Read for ChunkedReader<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            if self.offset >= self.bytes.len() {
                return Ok(0);
            }
            let len = self
                .chunk_size
                .min(buffer.len())
                .min(self.bytes.len() - self.offset);
            buffer[..len].copy_from_slice(&self.bytes[self.offset..self.offset + len]);
            self.offset += len;
            Ok(len)
        }
    }
}
