#!/bin/bash
set -euo pipefail

export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
export PATH="$CARGO_HOME/bin:$PATH"

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source "$CARGO_HOME/env"
rustup target add wasm32-unknown-unknown

ARCH="$(uname -m)"
mkdir -p bin
TRUNK_TARBALL="/tmp/trunk.tar.gz"
candidates=()
case "$ARCH" in
  aarch64|arm64)
    candidates=("trunk-aarch64-unknown-linux-gnu.tar.gz" "trunk-aarch64-unknown-linux-musl.tar.gz") ;;
  *)
    candidates=("trunk-x86_64-unknown-linux-musl.tar.gz" "trunk-x86_64-unknown-linux-gnu.tar.gz") ;;
esac

downloaded=0
for asset in "${candidates[@]}"; do
  if curl -fsSL "https://github.com/thedodd/trunk/releases/latest/download/${asset}" -o "$TRUNK_TARBALL"; then
    downloaded=1
    break
  fi
done

if [ "$downloaded" -ne 1 ]; then
  echo "no prebuilt trunk binary could be downloaded" >&2
  exit 1
fi

tar -xzf "$TRUNK_TARBALL" -C bin
export PATH="$PWD/bin:$PATH"

trunk --version
trunk build --release
ls -la dist
