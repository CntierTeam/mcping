use anyhow::{bail, Context, Result};
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::proto::rr::rdata::SRV;
use hickory_resolver::Resolver;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::Duration;

const SRV_TIMEOUT: Duration = Duration::from_secs(5);

pub struct ResolvedHost {
    pub host_for_handshake: String,
    pub addrs: Vec<SocketAddr>,
}

/// Hickory is used **only** for `_minecraft._tcp` SRV. A/AAAA always go through the
/// system resolver so we hit nscd / systemd-resolved / OS DNS cache.
fn srv_resolver() -> Result<Resolver> {
    let mut opts = ResolverOpts::default();
    opts.timeout = SRV_TIMEOUT;
    opts.attempts = 2;
    Resolver::new(ResolverConfig::default(), opts).context("create SRV resolver")
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

    // SRV needs a real DNS query; hickory does that. Target A/AAAA still uses the OS.
    let srv_name = format!("_minecraft._tcp.{host}");
    if let Ok(resolver) = srv_resolver() {
        if let Ok(lookup) = resolver.srv_lookup(&srv_name) {
            let mut records: Vec<&SRV> = lookup.iter().collect();
            records.sort_by_key(|r| (r.priority(), r.weight()));
            if let Some(srv) = records.first() {
                let target = srv.target().to_string().trim_end_matches('.').to_string();
                let port = srv.port();
                let addrs = resolve_a(&target, port)?;
                return Ok(ResolvedHost {
                    host_for_handshake: target,
                    addrs,
                });
            }
        }
    }
    // No SRV (or SRV lookup failed) is normal; fall through to A/AAAA:25565.

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
