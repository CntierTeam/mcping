# mcping CLI reference (operator)

## Synopsis

```text
mcping <TARGET> [-c COUNT] [-s|--show] [--java|--bedrock]
```

## Arguments

| Name | Default | Notes |
|------|---------|-------|
| `TARGET` | required | Host, `host:port`, IPv4/IPv6, or `[ipv6]:port` |
| `-c` / `--count` | `4` | Number of ping rounds |
| `-s` / `--show` | off | After first success: ANSI MOTD; Java also prints favicon half-block ASCII |
| `--java` | off | Force Java Edition Server List Ping |
| `--bedrock` | off | Force Bedrock RakNet Unconnected Ping |

`--java` and `--bedrock` conflict.

## Auto edition selection

1. If `--java` / `--bedrock` → that edition only.
2. Else if explicit port is `19132` or `19133` → prefer Bedrock, then fall back.
3. Else if explicit other port → prefer Java.
4. Else → try Java (SRV `_minecraft._tcp` / 25565), then Bedrock `19132`.

## Output shape

Mirrors classic `ping`:

```text
MCPING <host> (<addr:port>) [java|bedrock]: Minecraft status
64 bytes from <addr:port>: seq=N time=X.Y ms players=A/B ver=...
--- <host> mcping statistics ---
N packets transmitted, M received, P% packet loss, time Tms
rtt min/avg/max/mdev = ...
```

## What each edition reports

| Edition | Default port | Useful fields in status lines |
|---------|--------------|-------------------------------|
| Java | 25565 (+ `_minecraft._tcp` SRV) | players, version; `--show` → MOTD + favicon |
| Bedrock | 19132 | players, version/gamemode-ish MOTD fields; `--show` → MOTD |

## Install artifacts (Releases)

| File | Platform |
|------|----------|
| `mcping-x86_64-unknown-linux-gnu` | Linux glibc x86_64 |
| `mcping-x86_64-pc-windows-msvc.exe` | Windows MSVC x86_64 |
| `mcping_amd64.deb` | Debian/Ubuntu amd64 |

Download from https://github.com/CntierTeam/mcping/releases
