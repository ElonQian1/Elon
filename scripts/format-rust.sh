#!/usr/bin/env bash
set -euo pipefail

apply=false
if [[ "${1:-}" == "--apply" ]]; then
  apply=true
elif [[ $# -gt 0 ]]; then
  echo "Usage: scripts/format-rust.sh [--apply]" >&2
  exit 2
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(git -C "$script_dir" rev-parse --show-toplevel)"
cd "$repo_root"

manifests=(
  "server/Cargo.toml"
  "server/pc-dev-runtime/Cargo.toml"
  "server/homecli-proto/Cargo.toml"
)

for manifest in "${manifests[@]}"; do
  if [[ ! -f "$manifest" ]]; then
    echo "Rust manifest not found: $manifest" >&2
    exit 1
  fi

  if ! grep -Eq '^edition[[:space:]]*=[[:space:]]*"[^"]+"' "$manifest"; then
    echo "Rust manifest is missing an explicit edition: $manifest" >&2
    exit 1
  fi

  if [[ "$apply" == true ]]; then
    echo "Formatting $manifest"
    cargo fmt --manifest-path "$manifest" --all
  else
    echo "Checking $manifest"
    cargo fmt --manifest-path "$manifest" --all -- --check
  fi
done
