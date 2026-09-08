use anyhow::{bail, Context, Result};
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::proto::rr::rdata::SRV;
use hickory_resolver::Resolver;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::Duration;

const RESOLVE_TIMEOUT: Duration = Duration::from_secs(5);

pub struct ResolvedHost {
    pub host_for_handshake: String,
    pub addrs: Vec<SocketAddr>,
}

fn resolver() -> Result<Resolver> {
    let mut opts = ResolverOpts::default();
    opts.timeout = RESOLVE_TIMEOUT;
    opts.attempts = 2;
    Resolver::new(ResolverConfig::default(), opts).context("create DNS resolver")
}

/// Resolve A/AAAA for `host` and attach `port`.
pub fn resolve_a(host: &str, port: u16) -> Result<Vec<SocketAddr>> {
    // Literal IP: skip DNS.
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(vec![SocketAddr::new(ip, port)]);
    }

    let mut addrs: Vec<SocketAddr> = Vec::new();
    if let Ok(resolver) = resolver() {
        if let Ok(response) = resolver.lookup_ip(host) {
            addrs.extend(response.iter().map(|ip| SocketAddr::new(ip, port)));
        }
    }
    if addrs.is_empty() {
        // Fallback to system resolver (covers partial AAAA failures / odd local setups).
        addrs = (host, port)
            .to_socket_addrs()
            .with_context(|| format!("resolve {host}:{port}"))?
            .collect();
    }
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

    let resolver = resolver()?;
    let srv_name = format!("_minecraft._tcp.{host}");
    match resolver.srv_lookup(&srv_name) {
        Ok(lookup) => {
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
        Err(_) => {
            // No SRV is normal; fall through to A/AAAA:25565.
        }
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
