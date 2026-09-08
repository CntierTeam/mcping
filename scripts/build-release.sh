#!/usr/bin/env bash
# Build local release artifacts into ./dist/
# - Linux host binary (renamed with triple)
# - .deb via cargo-deb (if installed)
# - Optional: Windows via --windows (needs mingw or wine+msvc tooling; prefer CI)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
mkdir -p dist

echo "==> cargo build --release --locked"
cargo build --release --locked
cp -f target/release/mcping dist/mcping-x86_64-unknown-linux-gnu
chmod +x dist/mcping-x86_64-unknown-linux-gnu
echo "wrote dist/mcping-x86_64-unknown-linux-gnu"

if command -v cargo-deb >/dev/null 2>&1; then
  echo "==> cargo deb --no-build"
  cargo deb --no-build --output dist/
  DEB="$(find dist -maxdepth 1 -name '*.deb' -type f | head -1)"
  if [[ -n "$DEB" ]]; then
    cp -f "$DEB" dist/mcping_amd64.deb
    echo "wrote dist/mcping_amd64.deb (from $(basename "$DEB"))"
  fi
else
  echo "skip .deb: install with: cargo install cargo-deb"
fi

if [[ "${1:-}" == "--windows" ]]; then
  echo "==> cargo build --release --target x86_64-pc-windows-msvc"
  rustup target add x86_64-pc-windows-msvc
  cargo build --release --locked --target x86_64-pc-windows-msvc
  cp -f target/x86_64-pc-windows-msvc/release/mcping.exe \
    dist/mcping-x86_64-pc-windows-msvc.exe
  echo "wrote dist/mcping-x86_64-pc-windows-msvc.exe"
fi

(cd dist && sha256sum mcping-* mcping_*.deb 2>/dev/null > SHA256SUMS || true)
ls -lh dist/
echo "OK"
