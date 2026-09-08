use anyhow::{bail, Context, Result};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::PingResult;

const TIMEOUT: Duration = Duration::from_secs(5);
/// RakNet offline message magic (16 bytes).
const MAGIC: [u8; 16] = [
    0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];

#[derive(Debug, Clone)]
pub struct BedrockId {
    pub edition: String,
    pub motd1: String,
    pub protocol: String,
    pub version: String,
    pub online: i64,
    pub max: i64,
    pub motd2: String,
    pub gamemode: String,
}

/// Parse semicolon-separated Bedrock server ID string.
pub fn parse_server_id(s: &str) -> BedrockId {
    let parts: Vec<&str> = s.split(';').collect();
    let get = |i: usize| parts.get(i).copied().unwrap_or("").to_string();
    let parse_i = |i: usize| get(i).parse::<i64>().unwrap_or(0);
    BedrockId {
        edition: get(0),
        motd1: get(1),
        protocol: get(2),
        version: get(3),
        online: parse_i(4),
        max: parse_i(5),
        // field 6 = server UID (unused)
        motd2: get(7),
        gamemode: get(8),
    }
}

/// One Bedrock RakNet Unconnected Ping against `addr`.
pub fn ping(addr: SocketAddr) -> Result<PingResult> {
    let bind: SocketAddr = if addr.is_ipv4() {
        "0.0.0.0:0".parse().unwrap()
    } else {
        "[::]:0".parse().unwrap()
    };
    let sock = UdpSocket::bind(bind).context("bind UDP")?;
    sock.set_read_timeout(Some(TIMEOUT))?;
    sock.set_write_timeout(Some(TIMEOUT))?;
    sock.connect(addr)
        .with_context(|| format!("UDP connect {addr}"))?;

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    // Stable client GUID for offline ping packets.
    let client_guid: i64 = 0x0211_2233_4455_6677;

    // Unconnected Ping 0x01
    let mut pkt = Vec::with_capacity(1 + 8 + 16 + 8);
    pkt.push(0x01);
    pkt.extend_from_slice(&time.to_be_bytes());
    pkt.extend_from_slice(&MAGIC);
    pkt.extend_from_slice(&client_guid.to_be_bytes());

    let t0 = Instant::now();
    sock.send(&pkt).context("send unconnected ping")?;

    let mut buf = [0u8; 2048];
    let n = sock.recv(&mut buf).context("recv unconnected pong")?;
    let latency_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let data = &buf[..n];

    if data.is_empty() || data[0] != 0x1c {
        bail!(
            "expected Unconnected Pong 0x1c, got {:#x}",
            data.first().copied().unwrap_or(0)
        );
    }
    // 0x1c + Time(8) + ServerGuid(8) + MAGIC(16) + u16 len + string
    if data.len() < 1 + 8 + 8 + 16 + 2 {
        bail!("pong too short ({})", data.len());
    }
    let magic_start = 1 + 8 + 8;
    if data[magic_start..magic_start + 16] != MAGIC {
        bail!("pong magic mismatch");
    }
    let len_off = magic_start + 16;
    let str_len = u16::from_be_bytes([data[len_off], data[len_off + 1]]) as usize;
    let str_off = len_off + 2;
    if str_off + str_len > data.len() {
        bail!("pong string truncated");
    }
    let id_str = std::str::from_utf8(&data[str_off..str_off + str_len])
        .context("pong server id utf8")?;
    let id = parse_server_id(id_str);
    let protocol = id.protocol.parse::<i32>().ok();

    Ok(PingResult {
        edition: crate::Edition::Bedrock,
        address: addr,
        latency_ms,
        players_online: Some(id.online),
        players_max: Some(id.max),
        version: id.version,
        protocol,
        motd: id.motd1,
        motd2: if id.motd2.is_empty() {
            None
        } else {
            Some(id.motd2)
        },
        gamemode: if id.gamemode.is_empty() {
            None
        } else {
            Some(id.gamemode)
        },
        edition_name: if id.edition.is_empty() {
            None
        } else {
            Some(id.edition)
        },
        favicon: None,
    })
}
