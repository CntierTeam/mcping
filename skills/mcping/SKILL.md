---
name: mcping
description: >-
  Operate mcping CLI by running it for the user to ping Minecraft Java/Bedrock
  servers, check MOTD, latency, player counts, and favicon. Covers install from
  GitHub Releases, -c/--count, -s/--show, --java/--bedrock, auto edition
  selection, and common examples. Prefer shell execution over pasting recipes.
  Trigger on: mcping, Minecraft MOTD ping, server list ping CLI, Bedrock ping,
  Java status, SLP, RakNet Unconnected Ping, favicon MOTD, mcping --show.
license: MIT
metadata:
  short-description: 代跑 mcping（Java/Bedrock 测活）
---

# mcping

产品：**`mcping`** — Minecraft **Java**（TCP Server List Ping）/ **Bedrock**（RakNet Unconnected Ping）测活 CLI，输出风格像系统 `ping`。

你是 **操作员**：用户说 ping 某服 / 看 MOTD / 测延迟 → **自己在 shell 执行 `mcping`**，不要只拼命令给用户。

本 skill 是 **execute-first**：代跑产品，不是开发协议实现。细节见 [references/cli.md](references/cli.md)。

Repo: https://github.com/CntierTeam/mcping

## Agent 硬规则

1. **执行优先**：能跑就跑。二进制：`mcping` 或 `~/.local/bin/mcping`；没有就先从 Releases 安装。
2. **禁止**用「组装指令 / SAMPLE / YOUR_CLI / 长篇教程 / 自己手写 SLP」代替执行。短句说明 → 立刻跑 → 根据输出继续。
3. 用户说「ping X」→ **马上** `mcping X`（要 MOTD 加 `--show`；要多次加 `-c`）。缺 host/port 或强制版本时只问缺的那一项，问完继续跑。
4. 命令名永远 **`mcping`**，禁止 `SAMPLE` / `YOUR_CLI`。
5. 默认 **auto** 选版本；不要发明协议字段，以 `mcping` 打印为准。
6. Mock/自测无意义（无 mock 模式）；网络不可达就如实报告输出。

## 标准代跑流

```bash
command -v mcping || ~/.local/bin/mcping --help
mcping <TARGET>                 # 默认 -c 4，auto edition
mcping --show <TARGET>          # 彩色 MOTD；Java 还有 favicon ASCII
mcping -c 10 <TARGET>           # 更多采样
```

## 意图 → 怎么跑

| 用户意图 | 执行 |
|----------|------|
| ping / 测活 / 延迟 | `mcping <host>` |
| 看 MOTD / favicon | `mcping --show <host>` |
| 多打几次 | `mcping -c 10 <host>` |
| 强制 Java | `mcping --java <host>` |
| 强制 Bedrock | `mcping --bedrock <host>` |
| 非默认端口 | `mcping host:25566` 或 `host:19132` |

## CLI map

```text
mcping <TARGET> [-c COUNT] [-s|--show] [--java|--bedrock]
```

| Flag | Meaning |
|------|---------|
| `TARGET` | `host`、`host:port`、IP、`[ipv6]:port` |
| `-c, --count` | 次数（默认 `4`） |
| `-s, --show` | 彩色 MOTD；Java 另渲染 favicon ASCII |
| `--java` | 强制 Java SLP |
| `--bedrock` | 强制 Bedrock RakNet |

`--java` 与 `--bedrock` 互斥。

## Auto edition

1. `--java` / `--bedrock` → 仅该版。
2. 显式端口 `19132` / `19133` → 优先 Bedrock，再回退。
3. 其它显式端口 → 优先 Java。
4. 无端口 → Java（SRV `_minecraft._tcp` / 25565），再 Bedrock `19132`。

## 读输出（汇报给用户时）

```text
MCPING <host> (<addr:port>) [java|bedrock]: Minecraft status
64 bytes from <addr:port>: seq=N time=X.Y ms players=A/B ver=...
--- <host> mcping statistics ---
N packets transmitted, M received, P% packet loss, time Tms
rtt min/avg/max/mdev = ...
```

摘要：**edition、version、players、RTT/loss、MOTD**（若用了 `--show`）。

## Install（仅当本机没有 mcping）

从 [Releases](https://github.com/CntierTeam/mcping/releases) 取最新：

```bash
curl -fsSL -o mcping \
  https://github.com/CntierTeam/mcping/releases/latest/download/mcping-x86_64-unknown-linux-gnu
chmod +x mcping
mkdir -p ~/.local/bin && mv mcping ~/.local/bin/mcping
# 或 deb：mcping_amd64.deb + dpkg -i
command -v mcping && mcping --help
```

https://github.com/CntierTeam/mcping
