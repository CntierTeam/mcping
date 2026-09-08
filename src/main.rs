mod bedrock;
mod dns;
mod icon;
mod java;
mod motd;

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use std::io::{self, Write};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Edition {
    Java,
    Bedrock,
}

impl std::fmt::Display for Edition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Edition::Java => write!(f, "java"),
            Edition::Bedrock => write!(f, "bedrock"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PingResult {
    pub edition: Edition,
    pub address: SocketAddr,
    pub latency_ms: f64,
    pub players_online: Option<i64>,
    pub players_max: Option<i64>,
    pub version: String,
    pub protocol: Option<i32>,
    pub motd: String,
    pub motd2: Option<String>,
    pub gamemode: Option<String>,
    pub edition_name: Option<String>,
    /// Java favicon as `data:image/png;base64,...` (undecoded until --show icon render).
    pub favicon: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum Mode {
    #[default]
    Auto,
    Java,
    Bedrock,
}

#[derive(Parser, Debug)]
#[command(
    name = "mcping",
    about = "Minecraft Java/Bedrock server ping (SLP + RakNet Unconnected Ping)",
    version
)]
struct Cli {
    /// Target host, host:port, or IP
    target: String,

    /// Number of pings (like ping -c)
    #[arg(short = 'c', long = "count", default_value_t = 4)]
    count: u32,

    /// Show colored MOTD (+ Java favicon ASCII) after first success
    #[arg(short = 's', long = "show")]
    show: bool,

    /// Force Java Edition SLP
    #[arg(long = "java", conflicts_with = "bedrock")]
    java: bool,

    /// Force Bedrock RakNet ping
    #[arg(long = "bedrock", conflicts_with = "java")]
    bedrock: bool,
}

#[derive(Debug)]
struct ParsedTarget {
    host: String,
    port: Option<u16>,
}

fn parse_target(raw: &str) -> Result<ParsedTarget> {
    // [ipv6]:port
    if let Some(rest) = raw.strip_prefix('[') {
        let (host, after) = rest
            .split_once(']')
            .context("invalid IPv6 target, expected [addr] or [addr]:port")?;
        if after.is_empty() {
            return Ok(ParsedTarget {
                host: host.to_string(),
                port: None,
            });
        }
        let port_str = after
            .strip_prefix(':')
            .context("invalid IPv6 target, expected [addr]:port")?;
        let port: u16 = port_str.parse().context("invalid port")?;
        return Ok(ParsedTarget {
            host: host.to_string(),
            port: Some(port),
        });
    }

    // host:port — only split on last ':' and require numeric port (avoid IPv6 ambiguity)
    if let Some((host, port_str)) = raw.rsplit_once(':') {
        if !host.is_empty() && port_str.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(port) = port_str.parse::<u16>() {
                // Heuristic: bare IPv6 has multiple colons and no brackets — leave as host.
                if host.contains(':') {
                    // Likely raw IPv6 without brackets and without port.
                    return Ok(ParsedTarget {
                        host: raw.to_string(),
                        port: None,
                    });
                }
                return Ok(ParsedTarget {
                    host: host.to_string(),
                    port: Some(port),
                });
            }
        }
    }

    Ok(ParsedTarget {
        host: raw.to_string(),
        port: None,
    })
}

fn mode_from_cli(cli: &Cli, port: Option<u16>) -> Mode {
    if cli.java {
        return Mode::Java;
    }
    if cli.bedrock {
        return Mode::Bedrock;
    }
    match port {
        Some(19132) | Some(19133) => Mode::Bedrock,
        Some(_) => Mode::Java,
        None => Mode::Auto,
    }
}

struct ProbeOk {
    edition: Edition,
    result: PingResult,
    /// Hostname sent in Java Handshake (SRV target when redirected).
    handshake_host: String,
    /// Original user host (for re-resolve fallback).
    query_host: String,
}

fn try_java(host: &str, port: Option<u16>, keep_favicon: bool) -> Result<ProbeOk> {
    let resolved = dns::resolve_java(host, port)?;
    let preview = *resolved
        .addrs
        .first()
        .ok_or_else(|| anyhow::anyhow!("no Java addresses"))?;
    print_header(host, preview, Edition::Java);
    ping_java_addrs(&resolved, host, keep_favicon)
}

fn ping_java_addrs(
    resolved: &dns::ResolvedHost,
    query_host: &str,
    keep_favicon: bool,
) -> Result<ProbeOk> {
    let mut last_err = None;
    for addr in &resolved.addrs {
        match java::ping(&resolved.host_for_handshake, *addr, keep_favicon) {
            Ok(r) => {
                return Ok(ProbeOk {
                    edition: Edition::Java,
                    result: r,
                    handshake_host: resolved.host_for_handshake.clone(),
                    query_host: query_host.to_string(),
                });
            }
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no Java addresses")))
}

fn try_bedrock(host: &str, port: Option<u16>) -> Result<ProbeOk> {
    let resolved = dns::resolve_bedrock(host, port)?;
    let preview = *resolved
        .addrs
        .first()
        .ok_or_else(|| anyhow::anyhow!("no Bedrock addresses"))?;
    print_header(host, preview, Edition::Bedrock);
    ping_bedrock_addrs(&resolved, host)
}

fn ping_bedrock_addrs(resolved: &dns::ResolvedHost, query_host: &str) -> Result<ProbeOk> {
    let mut last_err = None;
    for addr in &resolved.addrs {
        match bedrock::ping(*addr) {
            Ok(r) => {
                return Ok(ProbeOk {
                    edition: Edition::Bedrock,
                    result: r,
                    handshake_host: query_host.to_string(),
                    query_host: query_host.to_string(),
                });
            }
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no Bedrock addresses")))
}

fn first_ping(mode: Mode, host: &str, port: Option<u16>, keep_favicon: bool) -> Result<ProbeOk> {
    match mode {
        Mode::Java => try_java(host, port, keep_favicon),
        Mode::Bedrock => try_bedrock(host, port),
        Mode::Auto => match try_java(host, port, keep_favicon) {
            Ok(r) => Ok(r),
            Err(java_err) => match try_bedrock(host, None) {
                Ok(r) => Ok(r),
                Err(bedrock_err) => {
                    bail!("auto failed: java={java_err:#}; bedrock={bedrock_err:#}")
                }
            },
        },
    }
}

fn ping_once(probe: &ProbeOk, port: Option<u16>) -> Result<PingResult> {
    let prefer_addr = probe.result.address;
    match probe.edition {
        // Later pings never need favicon — --show caches from the first success.
        Edition::Java => match java::ping(&probe.handshake_host, prefer_addr, false) {
            Ok(r) => Ok(r),
            Err(_) => try_java_quiet(&probe.query_host, port.or(Some(prefer_addr.port())))
                .map(|p| p.result),
        },
        Edition::Bedrock => match bedrock::ping(prefer_addr) {
            Ok(r) => Ok(r),
            Err(_) => try_bedrock_quiet(&probe.query_host, port.or(Some(prefer_addr.port())))
                .map(|p| p.result),
        },
    }
}

/// Re-resolve + ping without reprinting the MCPING header (fallback path).
fn try_java_quiet(host: &str, port: Option<u16>) -> Result<ProbeOk> {
    let resolved = dns::resolve_java(host, port)?;
    ping_java_addrs(&resolved, host, false)
}

fn try_bedrock_quiet(host: &str, port: Option<u16>) -> Result<ProbeOk> {
    let resolved = dns::resolve_bedrock(host, port)?;
    ping_bedrock_addrs(&resolved, host)
}

fn format_reply_line(seq: u32, r: &PingResult) -> String {
    let players = match (r.players_online, r.players_max) {
        (Some(o), Some(m)) => format!("players={o}/{m}"),
        (Some(o), None) => format!("players={o}/?"),
        _ => "players=?/?".into(),
    };
    let mut line = format!(
        "64 bytes from {}: seq={seq} time={:.1} ms {players} ver={}",
        r.address, r.latency_ms, r.version
    );
    if let Some(ref gm) = r.gamemode {
        line.push_str(&format!(" gamemode={gm}"));
    }
    if let Some(ref ed) = r.edition_name {
        line.push_str(&format!(" edition={ed}"));
    }
    line
}

fn print_header(host: &str, addr: SocketAddr, edition: Edition) {
    println!(
        "MCPING {} ({}:{}) [{}]: Minecraft status",
        host,
        addr.ip(),
        addr.port(),
        edition
    );
    let _ = io::stdout().flush();
}

/// MOTD only — flush so the user sees it before favicon ASCII work.
fn print_motd(r: &PingResult) {
    println!();
    println!("--- MOTD ---");
    println!("{}", motd::to_ansi(&r.motd));
    if let Some(ref m2) = r.motd2 {
        println!("{}", motd::to_ansi(m2));
    }
    let _ = io::stdout().flush();
}

fn print_icon(favicon_data_url: &str) {
    println!("--- ICON ---");
    icon::print_ascii(favicon_data_url);
    let _ = io::stdout().flush();
}

fn mdev(samples: &[f64], avg: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let var = samples.iter().map(|x| {
        let d = x - avg;
        d * d
    }).sum::<f64>() / samples.len() as f64;
    var.sqrt()
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.count == 0 {
        bail!("count must be >= 1");
    }

    let target = parse_target(&cli.target)?;
    let mode = mode_from_cli(&cli, target.port);
    let keep_favicon = cli.show;

    // Show the target immediately so DNS/SRV does not look like a hung process.
    println!("MCPING {}:", target.host);
    let _ = io::stdout().flush();

    let wall_start = Instant::now();
    let probe = first_ping(mode, &target.host, target.port, keep_favicon)
        .with_context(|| format!("ping {}", cli.target))?;
    let first = &probe.result;
    let prefer_addr = first.address;
    let effective_port = Some(prefer_addr.port());

    // Header already printed after DNS resolve (before TCP/UDP ping).
    // Show MOTD immediately, then the first reply line, then ICON last.
    let cached_favicon = first.favicon.clone();
    let mut shown = false;
    if cli.show {
        print_motd(first);
        shown = true;
    }

    let mut transmitted = 0u32;
    let mut received = 0u32;
    let mut rtts: Vec<f64> = Vec::new();

    // First sample
    transmitted += 1;
    received += 1;
    rtts.push(first.latency_ms);
    println!("{}", format_reply_line(1, first));
    let _ = io::stdout().flush();

    if cli.show {
        if let Some(ref fav) = cached_favicon {
            print_icon(fav);
        }
        println!();
    }

    for seq in 2..=cli.count {
        std::thread::sleep(Duration::from_secs(1));
        transmitted += 1;
        match ping_once(&probe, effective_port) {
            Ok(r) => {
                received += 1;
                rtts.push(r.latency_ms);
                if cli.show && !shown {
                    print_motd(&r);
                    shown = true;
                    println!("{}", format_reply_line(seq, &r));
                    let _ = io::stdout().flush();
                    if let Some(ref fav) = r.favicon {
                        print_icon(fav);
                    }
                    println!();
                } else {
                    println!("{}", format_reply_line(seq, &r));
                }
            }
            Err(e) => {
                println!("Request timeout for seq={seq} ({e})");
            }
        }
    }

    let elapsed_ms = wall_start.elapsed().as_millis();
    let loss = if transmitted == 0 {
        0.0
    } else {
        100.0 * (transmitted - received) as f64 / transmitted as f64
    };

    println!();
    println!("--- {} mcping statistics ---", target.host);
    println!(
        "{transmitted} packets transmitted, {received} received, {loss:.0}% packet loss, time {elapsed_ms}ms"
    );
    if !rtts.is_empty() {
        let min = rtts.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = rtts.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg = rtts.iter().sum::<f64>() / rtts.len() as f64;
        let md = mdev(&rtts, avg);
        println!("rtt min/avg/max/mdev = {min:.1}/{avg:.1}/{max:.1}/{md:.1} ms");
    }

    if received == 0 {
        std::process::exit(1);
    }
    Ok(())
}
