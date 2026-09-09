# mcping

Minecraft **Java** / **Bedrock** server ping CLI. Output style mirrors system `ping`.

- **Java**: Server List Ping (TCP 1.7+) with `_minecraft._tcp` SRV lookup
- **Bedrock**: RakNet Unconnected Ping/Pong (UDP, default port `19132`)

Repo: https://github.com/CntierTeam/mcping

## Build

```bash
cargo build --release
```

Binary: `target/release/mcping`

Release profile (see `Cargo.toml`): `lto`, `codegen-units = 1`, `panic = "abort"`, `strip`, `opt-level = "s"` — keeps the CLI small without UPX.

### Release artifacts (local)

```bash
# Linux binary + .deb (requires: cargo install cargo-deb)
./scripts/build-release.sh

# Optional Windows cross-build (needs MSVC target toolchain):
./scripts/build-release.sh --windows
```

Outputs under `dist/`:

| Artifact | Description |
|----------|-------------|
| `mcping-x86_64-unknown-linux-gnu` | Linux x86_64 binary |
| `mcping-x86_64-pc-windows-msvc.exe` | Windows x86_64 (CI / `--windows`) |
| `mcping_amd64.deb` | Debian package (amd64) |

Manual equivalents:

```bash
cargo build --release --locked
cp target/release/mcping dist/mcping-x86_64-unknown-linux-gnu

cargo install cargo-deb
cargo deb --no-build --output dist/
```

### GitHub Releases

Push a version tag to trigger [`.github/workflows/release.yml`](.github/workflows/release.yml):

```bash
git tag v0.1.0
git push origin v0.1.0
```

Or run the workflow manually (`workflow_dispatch`) and pass a tag like `v0.1.0`.

CI builds Linux + Windows binaries and the `.deb`, then uploads them to the GitHub Release.

## Usage

```text
mcping <TARGET> [-c COUNT] [-s|--show] [--java|--bedrock]
```

| Argument | Meaning |
|----------|---------|
| `TARGET` | `host`, `host:port`, IP, or `[ipv6]:port` |
| `-c, --count` | Number of pings (default `4`) |
| `-s, --show` | Colored MOTD; Java also renders favicon ASCII |
| `--java` | Force Java SLP |
| `--bedrock` | Force Bedrock RakNet |
| *(default)* | **auto**: try Java (SRV/25565), then Bedrock `19132`. Explicit `19132`/`19133` prefers Bedrock; other explicit ports prefer Java |

Examples:

```bash
mcping cntier.club
mcping --java --show cntier.club
mcping -c 10 hypixel.net
mcping --bedrock play.example.com
mcping host:25566
```

## Sample output

```text
MCPING cntier.club (103.91.210.150:25565) [java]: Minecraft status
64 bytes from 103.91.210.150:25565: seq=1 time=23.4 ms players=1/100 ver=1.21.8
...
--- cntier.club mcping statistics ---
4 packets transmitted, 4 received, 0% packet loss, time 3012ms
rtt min/avg/max/mdev = 20.1/22.8/25.0/1.7 ms
```

## Codex skill

**操作员代跑 / execute-first**：用户说 ping 某服时，在 shell 直接跑 `mcping`，不是只拼命令。

```bash
./scripts/install-codex-skill.sh         # copy into ~/.codex/skills/mcping
./scripts/install-codex-skill.sh link    # symlink from this checkout
./scripts/install-codex-skill.sh release # pull from GitHub
```

Skill sources live in `skills/mcping/`.

## Modules

| File | Role |
|------|------|
| `main.rs` | CLI, edition selection, ping loop, stats |
| `dns.rs` | Java SRV + shared A/AAAA |
| `java.rs` | VarInt / SLP handshake + status + ping/pong |
| `bedrock.rs` | RakNet unconnected ping/pong + MOTD fields |
| `motd.rs` | `§` codes → ANSI |
| `icon.rs` | Favicon PNG → half-block ASCII |

Sync I/O only (no async runtime).
