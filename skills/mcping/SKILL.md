---
name: mcping
description: >-
  Develop and operate the mcping Rust CLI (Minecraft Java SLP + Bedrock RakNet
  Unconnected Ping). Covers CLI usage (-c, --show, --java, --bedrock), protocol
  overview, building/releasing (Linux/Windows binaries + .deb), and Codex skill
  install. Trigger on: mcping, Minecraft ping, SLP, Server List Ping, RakNet
  Unconnected Ping, Bedrock ping, Java status, favicon MOTD.
license: MIT
metadata:
  short-description: Minecraft Java/Bedrock ping CLI
---

# mcping

Rust CLI that pings Minecraft **Java** (TCP Server List Ping) and **Bedrock**
(RakNet Unconnected Ping/Pong). Output style mirrors system `ping`.

Binary: `mcping`. Repo: https://github.com/CntierTeam/mcping

## Hard rules

1. Prefer the **installed binary** (`mcping` on `PATH`, or `dist/` / `target/release/mcping`) over re-implementing SLP/RakNet.
2. Default mode is **auto**: try Java (SRV / 25565), then Bedrock `19132`. Explicit `19132`/`19133` prefers Bedrock; other explicit ports prefer Java.
3. Sync I/O only — no async runtime. Keep it that way unless the user explicitly asks otherwise.
4. Code/comments in English; user-facing replies follow the user's language.
5. Do not invent protocol fields; Java status JSON and Bedrock `;`-separated MOTD are the sources of truth.

## Resolve the binary

```bash
command -v mcping
./target/release/mcping --help
./dist/mcping-x86_64-unknown-linux-gnu --help
```

Build:

```bash
cargo build --release
# or package locally:
./scripts/build-release.sh          # Linux binary + .deb (needs cargo-deb)
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

Examples:

```bash
mcping cntier.club
mcping --java --show cntier.club
mcping -c 10 hypixel.net
mcping --bedrock play.example.com
mcping host:25566
```

## Protocol overview

| Edition | Transport | Default port | Notes |
|---------|-----------|--------------|-------|
| Java | TCP SLP 1.7+ | 25565 (+ `_minecraft._tcp` SRV) | Handshake → status → ping/pong VarInt framing |
| Bedrock | UDP RakNet | 19132 | Unconnected Ping → Unconnected Pong; MOTD fields `;`-split |

Modules: `main.rs` (CLI/loop/stats), `dns.rs`, `java.rs`, `bedrock.rs`, `motd.rs`, `icon.rs`.

## Build / release

| Artifact | How |
|----------|-----|
| Linux `mcping-x86_64-unknown-linux-gnu` | `cargo build --release` (CI host) |
| Windows `mcping-x86_64-pc-windows-msvc.exe` | GHA `windows-latest` + MSVC target |
| `mcping_amd64.deb` | `cargo deb` (`[package.metadata.deb]` in `Cargo.toml`) |

Tag `v*` (or `workflow_dispatch` with a tag) → `.github/workflows/release.yml` builds all three and uploads a GitHub Release.

Local:

```bash
cargo install cargo-deb   # once
./scripts/build-release.sh
```

## Install this skill into Codex

```bash
./scripts/install-codex-skill.sh              # copy from checkout
./scripts/install-codex-skill.sh link         # symlink
./scripts/install-codex-skill.sh release      # pull from CntierTeam/mcping
```

## Typical agent workflow

1. Confirm target host/port and edition (`--java` / `--bedrock` / auto).
2. Run `mcping` (or build first if binary missing).
3. For MOTD/favicon inspection use `--show`.
4. When changing protocol code, keep sync I/O and verify with a real public server when network is available.

## References

- CLI details: [references/cli.md](references/cli.md)
- Project README: `README.md`
