use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::PingResult;

const TIMEOUT: Duration = Duration::from_secs(5);
/// Protocol version accepted by modern 1.7+ status endpoints (1.21.8-ish).
const PROTOCOL_VERSION: i32 = 772;

#[derive(Debug, Deserialize)]
struct StatusResponse {
    version: Option<StatusVersion>,
    players: Option<StatusPlayers>,
    description: Option<Value>,
    favicon: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StatusVersion {
    name: Option<String>,
    protocol: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct StatusPlayers {
    online: Option<i64>,
    max: Option<i64>,
}

fn write_varint(buf: &mut Vec<u8>, mut value: i32) {
    loop {
        if (value & !0x7F) == 0 {
            buf.push(value as u8);
            return;
        }
        buf.push(((value & 0x7F) | 0x80) as u8);
        value = ((value as u32) >> 7) as i32;
    }
}

fn read_varint_from<R: Read>(r: &mut R) -> Result<i32> {
    let mut num_read = 0;
    let mut result = 0i32;
    loop {
        let mut byte = [0u8; 1];
        r.read_exact(&mut byte).context("read varint byte")?;
        let value = byte[0] as i32;
        result |= (value & 0x7F) << (7 * num_read);
        num_read += 1;
        if num_read > 5 {
            bail!("varint too long");
        }
        if (value & 0x80) == 0 {
            break;
        }
    }
    Ok(result)
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    write_varint(buf, bytes.len() as i32);
    buf.extend_from_slice(bytes);
}

fn write_packet(stream: &mut TcpStream, packet_id: i32, payload: &[u8]) -> Result<()> {
    let mut inner = Vec::new();
    write_varint(&mut inner, packet_id);
    inner.extend_from_slice(payload);
    let mut frame = Vec::new();
    write_varint(&mut frame, inner.len() as i32);
    frame.extend_from_slice(&inner);
    stream.write_all(&frame).context("write packet")?;
    stream.flush().ok();
    Ok(())
}

fn read_packet(stream: &mut TcpStream) -> Result<(i32, Vec<u8>)> {
    let length = read_varint_from(stream)? as usize;
    if length == 0 || length > 2 * 1024 * 1024 {
        bail!("invalid packet length {length}");
    }
    let mut data = vec![0u8; length];
    stream.read_exact(&mut data).context("read packet body")?;
    let mut cursor = std::io::Cursor::new(&data);
    let packet_id = read_varint_from(&mut cursor)?;
    let pos = cursor.position() as usize;
    Ok((packet_id, data[pos..].to_vec()))
}

fn extract_motd(description: &Option<Value>) -> String {
    match description {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => chat_component_to_legacy(other),
    }
}

fn chat_component_to_legacy(v: &Value) -> String {
    let mut out = String::new();
    append_chat(v, &mut out, None);
    out
}

fn append_chat(v: &Value, out: &mut String, parent_color: Option<&str>) {
    match v {
        Value::String(s) => out.push_str(s),
        Value::Array(arr) => {
            for item in arr {
                append_chat(item, out, parent_color);
            }
        }
        Value::Object(map) => {
            let color = map
                .get("color")
                .and_then(|c| c.as_str())
                .or(parent_color);
            if let Some(c) = color {
                if let Some(code) = named_color_to_code(c) {
                    out.push('§');
                    out.push(code);
                }
            }
            if map.get("bold").and_then(|b| b.as_bool()) == Some(true) {
                out.push_str("§l");
            }
            if map.get("italic").and_then(|b| b.as_bool()) == Some(true) {
                out.push_str("§o");
            }
            if map.get("underlined").and_then(|b| b.as_bool()) == Some(true) {
                out.push_str("§n");
            }
            if map.get("strikethrough").and_then(|b| b.as_bool()) == Some(true) {
                out.push_str("§m");
            }
            if map.get("obfuscated").and_then(|b| b.as_bool()) == Some(true) {
                out.push_str("§k");
            }
            if let Some(text) = map.get("text").and_then(|t| t.as_str()) {
                out.push_str(text);
            }
            if let Some(extra) = map.get("extra") {
                append_chat(extra, out, color);
            }
            if let Some(translate) = map.get("translate").and_then(|t| t.as_str()) {
                out.push_str(translate);
                if let Some(with) = map.get("with").and_then(|w| w.as_array()) {
                    for arg in with {
                        out.push(' ');
                        append_chat(arg, out, color);
                    }
                }
            }
        }
        _ => {}
    }
}

fn named_color_to_code(name: &str) -> Option<char> {
    Some(match name {
        "black" => '0',
        "dark_blue" => '1',
        "dark_green" => '2',
        "dark_aqua" | "dark_cyan" => '3',
        "dark_red" => '4',
        "dark_purple" => '5',
        "gold" | "dark_yellow" => '6',
        "gray" | "grey" => '7',
        "dark_gray" | "dark_grey" => '8',
        "blue" => '9',
        "green" => 'a',
        "aqua" | "cyan" => 'b',
        "red" => 'c',
        "light_purple" | "purple" | "magenta" => 'd',
        "yellow" => 'e',
        "white" => 'f',
        // hex colors (#rrggbb) — approximate to nearest legacy if needed later; skip code
        _ if name.starts_with('#') => return None,
        _ => return None,
    })
}

fn decode_favicon(data_url: &str) -> Option<Vec<u8>> {
    const PREFIX: &str = "data:image/png;base64,";
    let b64 = data_url.strip_prefix(PREFIX)?;
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(b64).ok()
}

/// One Java SLP ping against `addr`. `handshake_host` is the hostname sent in Handshake.
pub fn ping(handshake_host: &str, addr: SocketAddr) -> Result<PingResult> {
    let mut stream = TcpStream::connect_timeout(&addr, TIMEOUT)
        .with_context(|| format!("TCP connect {addr}"))?;
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    stream.set_nodelay(true).ok();

    // Handshake
    let mut hs = Vec::new();
    write_varint(&mut hs, PROTOCOL_VERSION);
    write_string(&mut hs, handshake_host);
    hs.extend_from_slice(&addr.port().to_be_bytes());
    write_varint(&mut hs, 1); // nextState = status
    write_packet(&mut stream, 0x00, &hs)?;

    // Status Request
    write_packet(&mut stream, 0x00, &[])?;

    let (id, payload) = read_packet(&mut stream)?;
    if id != 0x00 {
        bail!("expected status response 0x00, got {id:#x}");
    }
    let mut cur = std::io::Cursor::new(&payload);
    let json_len = read_varint_from(&mut cur)? as usize;
    let start = cur.position() as usize;
    let end = start + json_len;
    if end > payload.len() {
        bail!("status JSON truncated");
    }
    let json = std::str::from_utf8(&payload[start..end]).context("status JSON utf8")?;
    let status: StatusResponse = serde_json::from_str(json).context("parse status JSON")?;

    // Ping / Pong for RTT
    let payload_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let mut ping_payload = Vec::with_capacity(8);
    ping_payload.extend_from_slice(&payload_ts.to_be_bytes());
    let t0 = Instant::now();
    write_packet(&mut stream, 0x01, &ping_payload)?;
    let (pong_id, pong_body) = read_packet(&mut stream)?;
    let latency_ms = t0.elapsed().as_secs_f64() * 1000.0;
    if pong_id != 0x01 {
        bail!("expected pong 0x01, got {pong_id:#x}");
    }
    let _ = pong_body; // servers usually echo the timestamp

    let version = status
        .version
        .as_ref()
        .and_then(|v| v.name.clone())
        .unwrap_or_else(|| "?".into());
    let protocol = status.version.as_ref().and_then(|v| v.protocol);
    let players_online = status.players.as_ref().and_then(|p| p.online);
    let players_max = status.players.as_ref().and_then(|p| p.max);
    let motd = extract_motd(&status.description);
    let favicon = status.favicon.as_deref().and_then(decode_favicon);

    Ok(PingResult {
        edition: crate::Edition::Java,
        address: addr,
        latency_ms,
        players_online,
        players_max,
        version,
        protocol,
        motd,
        motd2: None,
        gamemode: None,
        edition_name: None,
        favicon,
    })
}
