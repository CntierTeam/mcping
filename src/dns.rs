use anyhow::{bail, Context, Result};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

const SRV_TIMEOUT: Duration = Duration::from_secs(5);
const DNS_PORT: u16 = 53;
const QTYPE_SRV: u16 = 33;
const QCLASS_IN: u16 = 1;

pub struct ResolvedHost {
    pub host_for_handshake: String,
    pub addrs: Vec<SocketAddr>,
}

/// Resolve A/AAAA for `host` and attach `port` via `getaddrinfo` (`ToSocketAddrs`).
pub fn resolve_a(host: &str, port: u16) -> Result<Vec<SocketAddr>> {
    // Literal IP: skip DNS.
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(vec![SocketAddr::new(ip, port)]);
    }

    let addrs: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .with_context(|| format!("resolve {host}:{port}"))?
        .collect();
    if addrs.is_empty() {
        bail!("no addresses for {host}");
    }
    Ok(addrs)
}

/// Java resolution: optional SRV `_minecraft._tcp.<host>`, else A/AAAA + default/explicit port.
pub fn resolve_java(host: &str, explicit_port: Option<u16>) -> Result<ResolvedHost> {
    if let Some(port) = explicit_port {
        let addrs = resolve_a(host, port)?;
        return Ok(ResolvedHost {
            host_for_handshake: host.to_string(),
            addrs,
        });
    }

    // Minimal sync UDP SRV — no hickory/tokio. Failures fall through to A/AAAA:25565.
    let srv_name = format!("_minecraft._tcp.{host}");
    if let Some((target, port)) = lookup_srv(&srv_name) {
        let addrs = resolve_a(&target, port)?;
        return Ok(ResolvedHost {
            host_for_handshake: target,
            addrs,
        });
    }

    let addrs = resolve_a(host, 25565)?;
    Ok(ResolvedHost {
        host_for_handshake: host.to_string(),
        addrs,
    })
}

/// Bedrock: A/AAAA only (no Java-style SRV). Default port 19132.
pub fn resolve_bedrock(host: &str, explicit_port: Option<u16>) -> Result<ResolvedHost> {
    let port = explicit_port.unwrap_or(19132);
    let addrs = resolve_a(host, port)?;
    Ok(ResolvedHost {
        host_for_handshake: host.to_string(),
        addrs,
    })
}

/// Best-effort `_minecraft._tcp` SRV via system DNS servers (UDP/53).
fn lookup_srv(name: &str) -> Option<(String, u16)> {
    let mut best: Option<(u16, u16, String, u16)> = None; // priority, weight, target, port
    for ns in nameservers() {
        if let Some(records) = query_srv(ns, name) {
            for (prio, weight, port, target) in records {
                match best {
                    None => best = Some((prio, weight, target, port)),
                    Some((bp, bw, _, _)) if (prio, weight) < (bp, bw) => {
                        best = Some((prio, weight, target, port));
                    }
                    _ => {}
                }
            }
            if best.is_some() {
                break;
            }
        }
    }
    best.map(|(_, _, target, port)| (target, port))
}

fn nameservers() -> Vec<SocketAddr> {
    let mut out = Vec::new();

    #[cfg(unix)]
    if let Ok(text) = std::fs::read_to_string("/etc/resolv.conf") {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            if parts.next() != Some("nameserver") {
                continue;
            }
            if let Some(ip) = parts.next() {
                if let Ok(addr) = ip.parse::<IpAddr>() {
                    out.push(SocketAddr::new(addr, DNS_PORT));
                }
            }
        }
    }

    #[cfg(windows)]
    if let Ok(adapters) = ipconfig::get_adapters() {
        for adapter in adapters {
            for dns in adapter.dns_servers() {
                out.push(SocketAddr::new(*dns, DNS_PORT));
            }
        }
    }

    // Public fallback when system config is missing (containers, minimal images).
    if out.is_empty() {
        out.push(SocketAddr::new(IpAddr::from([1, 1, 1, 1]), DNS_PORT));
        out.push(SocketAddr::new(IpAddr::from([8, 8, 8, 8]), DNS_PORT));
    }
    out
}

fn query_srv(server: SocketAddr, name: &str) -> Option<Vec<(u16, u16, u16, String)>> {
    let id = 0x4d43u16; // 'MC'
    let mut packet = Vec::with_capacity(64);
    packet.extend_from_slice(&id.to_be_bytes());
    packet.extend_from_slice(&0x0100u16.to_be_bytes()); // RD
    packet.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT
    packet.extend_from_slice(&0u16.to_be_bytes()); // ANCOUNT
    packet.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT
    packet.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT
    write_qname(&mut packet, name)?;
    packet.extend_from_slice(&QTYPE_SRV.to_be_bytes());
    packet.extend_from_slice(&QCLASS_IN.to_be_bytes());

    let sock = UdpSocket::bind(match server {
        SocketAddr::V4(_) => "0.0.0.0:0",
        SocketAddr::V6(_) => "[::]:0",
    })
    .ok()?;
    sock.set_read_timeout(Some(SRV_TIMEOUT)).ok()?;
    sock.set_write_timeout(Some(SRV_TIMEOUT)).ok()?;
    sock.send_to(&packet, server).ok()?;

    let mut buf = [0u8; 1500];
    let (n, _) = sock.recv_from(&mut buf).ok()?;
    parse_srv_response(&buf[..n], id)
}

fn write_qname(buf: &mut Vec<u8>, name: &str) -> Option<()> {
    for label in name.trim_end_matches('.').split('.') {
        let bytes = label.as_bytes();
        if bytes.is_empty() || bytes.len() > 63 {
            return None;
        }
        buf.push(bytes.len() as u8);
        buf.extend_from_slice(bytes);
    }
    buf.push(0);
    Some(())
}

fn parse_srv_response(msg: &[u8], expect_id: u16) -> Option<Vec<(u16, u16, u16, String)>> {
    if msg.len() < 12 {
        return None;
    }
    let id = u16::from_be_bytes([msg[0], msg[1]]);
    if id != expect_id {
        return None;
    }
    let flags = u16::from_be_bytes([msg[2], msg[3]]);
    if flags & 0x000f != 0 {
        // RCODE != NOERROR
        return None;
    }
    let qd = u16::from_be_bytes([msg[4], msg[5]]) as usize;
    let an = u16::from_be_bytes([msg[6], msg[7]]) as usize;
    let mut i = 12usize;
    for _ in 0..qd {
        i = skip_name(msg, i)?;
        i = i.checked_add(4)?; // qtype + qclass
    }

    let mut out = Vec::new();
    for _ in 0..an {
        i = skip_name(msg, i)?;
        if i + 10 > msg.len() {
            return None;
        }
        let rtype = u16::from_be_bytes([msg[i], msg[i + 1]]);
        let _class = u16::from_be_bytes([msg[i + 2], msg[i + 3]]);
        // TTL at i+4..i+8
        let rdlen = u16::from_be_bytes([msg[i + 8], msg[i + 9]]) as usize;
        i += 10;
        if i + rdlen > msg.len() {
            return None;
        }
        if rtype == QTYPE_SRV && rdlen >= 6 {
            let prio = u16::from_be_bytes([msg[i], msg[i + 1]]);
            let weight = u16::from_be_bytes([msg[i + 2], msg[i + 3]]);
            let port = u16::from_be_bytes([msg[i + 4], msg[i + 5]]);
            let (target, _) = read_name(msg, i + 6)?;
            let target = target.trim_end_matches('.').to_string();
            if !target.is_empty() {
                out.push((prio, weight, port, target));
            }
        }
        i += rdlen;
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn skip_name(msg: &[u8], mut i: usize) -> Option<usize> {
    let mut guard = 0;
    loop {
        if guard > 64 || i >= msg.len() {
            return None;
        }
        guard += 1;
        let len = msg[i];
        if len == 0 {
            return Some(i + 1);
        }
        if len & 0xc0 == 0xc0 {
            // compression pointer — name ends after these 2 bytes on the wire
            return Some(i + 2);
        }
        i = i.checked_add(1 + len as usize)?;
    }
}

fn read_name(msg: &[u8], mut i: usize) -> Option<(String, usize)> {
    let mut labels = Vec::new();
    let mut end = i;
    let mut jumped = false;
    let mut guard = 0;
    loop {
        if guard > 64 || i >= msg.len() {
            return None;
        }
        guard += 1;
        let len = msg[i];
        if len == 0 {
            if !jumped {
                end = i + 1;
            }
            break;
        }
        if len & 0xc0 == 0xc0 {
            if i + 1 >= msg.len() {
                return None;
            }
            let ptr = (((len as usize) & 0x3f) << 8) | (msg[i + 1] as usize);
            if !jumped {
                end = i + 2;
            }
            i = ptr;
            jumped = true;
            continue;
        }
        i += 1;
        if i + len as usize > msg.len() {
            return None;
        }
        labels.push(
            std::str::from_utf8(&msg[i..i + len as usize])
                .ok()?
                .to_string(),
        );
        i += len as usize;
        if !jumped {
            end = i;
        }
    }
    Some((labels.join("."), end))
}
