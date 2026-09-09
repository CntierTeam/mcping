---
name: mcping
description: >-
  Use the mcping CLI to ping Minecraft Java/Bedrock servers, check MOTD, latency,
  player counts, and favicon. Covers install from GitHub Releases, -c/--count,
  -s/--show, --java/--bedrock, auto edition selection, and common examples.
  Trigger on: mcping, Minecraft MOTD ping, server list ping CLI, Bedrock ping,
  Java status, SLP, RakNet Unconnected Ping, favicon MOTD, mcping --show.
license: MIT
metadata:
  short-description: Ping Minecraft servers with mcping
---

# mcping

Operator skill for the **mcping** CLI: ping Minecraft **Java** (TCP Server List
Ping) and **Bedrock** (RakNet Unconnected Ping/Pong). Output style mirrors
system `ping`.

Binary: `mcping`. Repo / releases: https://github.com/CntierTeam/mcping

## Hard rules

1. Prefer the **installed binary** (`mcping` on `PATH`) over re-implementing SLP/RakNet.
2. This skill is for **using** mcping. Do **not** implement, refactor, or release
   mcping unless the user explicitly asks to develop it.
3. Default mode is **auto**: try Java (SRV / 25565), then Bedrock `19132`.
   Explicit `19132`/`19133` prefers Bedrock; other explicit ports prefer Java.
4. Do not invent protocol fields; trust what `mcping` prints (and `--show` MOTD).
5. User-facing replies follow the user's language.

## Install the binary

From [GitHub Releases](https://github.com/CntierTeam/mcping/releases) (pick latest):

```bash
# Linux x86_64 — place on PATH
curl -fsSL -o mcping \
  https://github.com/CntierTeam/mcping/releases/latest/download/mcping-x86_64-unknown-linux-gnu
chmod +x mcping
sudo mv mcping /usr/local/bin/mcping   # or: mkdir -p ~/bin && mv mcping ~/bin/

# Debian/Ubuntu amd64
curl -fsSL -O https://github.com/CntierTeam/mcping/releases/latest/download/mcping_amd64.deb
sudo dpkg -i mcping_amd64.deb

# Windows: download mcping-x86_64-pc-windows-msvc.exe from the same Releases page
```

Verify:

```bash
command -v mcping && mcping --help
```

## CLI map

```text
mcping <TARGET> [-c COUNT] [-s|--show] [--java|--bedrock]
```

| Flag | Meaning |
|------|---------|
| `TARGET` | `host`, `host:port`, IP, `[ipv6]:port` |
| `-c, --count` | Ping count (default `4`) |
| `-s, --show` | Colored MOTD; Java also renders favicon ASCII |
| `--java` | Force Java SLP |
| `--bedrock` | Force Bedrock RakNet |

`--java` and `--bedrock` conflict.

## Common examples

```bash
mcping cntier.club
mcping --java --show cntier.club
mcping -c 10 hypixel.net
mcping --bedrock play.example.com
mcping host:25566
mcping host:19132
```

| Goal | Command |
|------|---------|
| Quick reachability / RTT | `mcping <host>` |
| Colored MOTD (+ Java favicon) | `mcping --show <host>` |
| More samples | `mcping -c 10 <host>` |
| Force Java | `mcping --java <host>` |
| Force Bedrock | `mcping --bedrock <host>` |
| Non-default port | `mcping host:25566` |

## Auto edition selection

1. `--java` / `--bedrock` → that edition only.
2. Explicit port `19132` or `19133` → prefer Bedrock, then fall back.
3. Explicit other port → prefer Java.
4. No port → try Java (SRV `_minecraft._tcp` / 25565), then Bedrock `19132`.

## Reading the output

```text
MCPING <host> (<addr:port>) [java|bedrock]: Minecraft status
64 bytes from <addr:port>: seq=N time=X.Y ms players=A/B ver=...
--- <host> mcping statistics ---
N packets transmitted, M received, P% packet loss, time Tms
rtt min/avg/max/mdev = ...
```

With `--show`, after the first success mcping also prints ANSI MOTD (and for Java,
a half-block ASCII favicon). Summarize edition, version, players, RTT, and MOTD
when reporting to the user.

## Typical agent workflow

1. Ensure `mcping` is on `PATH` (install from Releases if missing).
2. Confirm target host/port and whether to force `--java` / `--bedrock`.
3. Run `mcping` (add `--show` for MOTD/favicon; `-c` for more samples).
4. Report latency, loss, players/version, and MOTD if requested.

## Developing mcping

Only if the user asks to change the tool itself: see the repo `README.md`.
Do not treat protocol/CI/release work as part of normal ping usage.

## References

- CLI details: [references/cli.md](references/cli.md)
- Releases: https://github.com/CntierTeam/mcping/releases
