use crate::loaders::natives::parse_server_address;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub const DEFAULT_PING_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_SLP_JSON_LEN: i32 = 65536;
const SLP_PROTOCOL_VERSION: i32 = 763;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatusPayload {
    pub address: String,
    pub online: bool,
    pub latency_ms: Option<u64>,
    pub players_online: Option<u32>,
    pub players_max: Option<u32>,
    pub motd: Option<String>,
    pub version_name: Option<String>,
}

impl ServerStatusPayload {
    pub fn offline(address: &str) -> Self {
        Self {
            address: address.to_string(),
            online: false,
            latency_ms: None,
            players_online: None,
            players_max: None,
            motd: None,
            version_name: None,
        }
    }
}

pub fn encode_varint(mut value: i32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5);
    loop {
        let mut temp = (value & 0x7F) as u8;
        value = ((value as u32) >> 7) as i32;
        if value != 0 {
            temp |= 0x80;
        }
        buf.push(temp);
        if value == 0 {
            break;
        }
    }
    buf
}

pub async fn read_varint<R: AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<i32> {
    let mut num_read = 0;
    let mut result: i32 = 0;
    loop {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte).await?;
        let read = byte[0];
        let value = (read & 0x7F) as i32;
        result |= value << (7 * num_read);

        num_read += 1;
        if num_read > 5 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "VarInt is too big",
            ));
        }
        if (read & 0x80) == 0 {
            break;
        }
    }
    Ok(result)
}

pub fn strip_minecraft_formatting(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            let _ = chars.next();
        } else {
            result.push(c);
        }
    }
    result.trim().to_string()
}

fn extract_motd(desc: &serde_json::Value) -> Option<String> {
    match desc {
        serde_json::Value::String(s) => {
            let clean = strip_minecraft_formatting(s);
            (!clean.is_empty()).then_some(clean)
        }
        serde_json::Value::Object(map) => {
            let mut combined = String::new();
            if let Some(serde_json::Value::String(text)) = map.get("text") {
                combined.push_str(text);
            }
            if let Some(serde_json::Value::Array(extra)) = map.get("extra") {
                for item in extra {
                    if let Some(s) = item
                        .as_str()
                        .or_else(|| item.get("text").and_then(|v| v.as_str()))
                    {
                        combined.push_str(s);
                    }
                }
            }
            let clean = strip_minecraft_formatting(&combined);
            (!clean.is_empty()).then_some(clean)
        }
        _ => None,
    }
}

pub fn build_slp_handshake_and_request(host: &str, port: u16) -> Vec<u8> {
    let mut handshake_body = Vec::with_capacity(32 + host.len());
    // Packet ID: 0x00 (Handshake)
    handshake_body.extend_from_slice(&encode_varint(0x00));
    // Protocol Version
    handshake_body.extend_from_slice(&encode_varint(SLP_PROTOCOL_VERSION));
    // Server Host String (VarInt len + UTF-8 bytes)
    let host_bytes = host.as_bytes();
    handshake_body.extend_from_slice(&encode_varint(host_bytes.len() as i32));
    handshake_body.extend_from_slice(host_bytes);
    // Server Port (u16 big-endian)
    handshake_body.extend_from_slice(&port.to_be_bytes());
    // Next State: 1 (Status)
    handshake_body.extend_from_slice(&encode_varint(1));

    let mut packet = Vec::with_capacity(handshake_body.len() + 8);
    // Framed Handshake Packet
    packet.extend_from_slice(&encode_varint(handshake_body.len() as i32));
    packet.extend_from_slice(&handshake_body);

    // Framed Status Request Packet (Length = 1, Packet ID = 0x00)
    packet.push(0x01);
    packet.push(0x00);

    packet
}

async fn measure_slp_pong_rtt(stream: &mut TcpStream) -> Option<u64> {
    let ping_start = Instant::now();
    let mut ping_packet = Vec::with_capacity(10);
    ping_packet.push(0x09); // Packet length: 1 (ID) + 8 (i64 payload)
    ping_packet.push(0x01); // Packet ID: 0x01
    ping_packet.extend_from_slice(&123456789i64.to_be_bytes());

    stream.write_all(&ping_packet).await.ok()?;
    stream.flush().await.ok()?;

    let _pong_len = tokio::time::timeout(Duration::from_millis(400), read_varint(stream))
        .await
        .ok()?
        .ok()?;
    if read_varint(stream).await.ok()? != 0x01 {
        return None;
    }

    let mut pong_payload = [0u8; 8];
    stream.read_exact(&mut pong_payload).await.ok()?;
    Some(ping_start.elapsed().as_millis() as u64)
}

async fn perform_slp_ping(address: &str) -> std::io::Result<ServerStatusPayload> {
    let (host, port) = parse_server_address(address);
    let target = format!("{}:{}", host, port);

    let start = Instant::now();
    let mut stream = TcpStream::connect(&target).await?;
    let _ = stream.set_nodelay(true);

    let request_bytes = build_slp_handshake_and_request(&host, port);
    stream.write_all(&request_bytes).await?;
    stream.flush().await?;

    // Read Status Response Packet
    let _packet_len = read_varint(&mut stream).await?;
    let packet_id = read_varint(&mut stream).await?;
    if packet_id != 0x00 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Unexpected SLP packet ID: {}", packet_id),
        ));
    }

    let json_len = read_varint(&mut stream).await?;
    if json_len <= 0 || json_len > MAX_SLP_JSON_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid SLP JSON length: {}", json_len),
        ));
    }

    let mut json_buf = vec![0u8; json_len as usize];
    stream.read_exact(&mut json_buf).await?;

    let mut latency_ms = start.elapsed().as_millis() as u64;
    if let Some(rtt) = measure_slp_pong_rtt(&mut stream).await {
        latency_ms = rtt;
    }

    let parsed: serde_json::Value = serde_json::from_slice(&json_buf).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Malformed SLP JSON: {}", e),
        )
    })?;

    let players_online = parsed
        .get("players")
        .and_then(|p| p.get("online"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let players_max = parsed
        .get("players")
        .and_then(|p| p.get("max"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let version_name = parsed
        .get("version")
        .and_then(|v| v.get("name"))
        .and_then(|n| n.as_str())
        .map(strip_minecraft_formatting);

    let motd = parsed.get("description").and_then(extract_motd);

    Ok(ServerStatusPayload {
        address: address.to_string(),
        online: true,
        latency_ms: Some(latency_ms.max(1)),
        players_online,
        players_max,
        motd,
        version_name,
    })
}

pub async fn ping_server(address: &str, timeout_duration: Duration) -> ServerStatusPayload {
    match tokio::time::timeout(timeout_duration, perform_slp_ping(address)).await {
        Ok(Ok(payload)) => payload,
        _ => ServerStatusPayload::offline(address),
    }
}
