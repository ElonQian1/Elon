#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

test -f server/Cargo.lock
if grep -Eq '^(/?server/)?Cargo\.lock$' .gitignore server/.gitignore; then
  echo "Cargo.lock must be tracked, not ignored" >&2
  exit 1
fi

source_count="$(grep -c '"id":' scripts/validation/cargo-sources.json)"
test "$source_count" -eq 3
test "$(grep -c '"index": "sparse+https://' scripts/validation/cargo-sources.json)" -eq 3
grep -q 'crates-io-official' scripts/validation/cargo-sources.json
grep -q 'rsproxy-official' scripts/validation/cargo-sources.json
grep -q 'ustc-official' scripts/validation/cargo-sources.json

grep -q 'prepare-push.ps1' .githooks/pre-push
grep -q 'ELON_ENABLE_RUST_PUSH_RECEIPT' .githooks/pre-push
grep -q 'RUST_PUSH_RECEIPT_GATE=disabled' .githooks/pre-push
grep -q 'ELON_ENABLE_RUST_PUSH_RECEIPT' scripts/push.ps1
grep -q 'RUST_PUSH_RECEIPT_GATE=disabled' scripts/push.ps1
grep -q -- '--locked' scripts/prepare-push.ps1
grep -q 'CARGO_SOURCE_REPAIR_REQUIRED' scripts/validation/Cargo.Network.psm1
grep -q 'elon.ai.cargo_source_repair.v1' scripts/validation/Cargo.Network.psm1
echo "PASS: Cargo source Bash contract"
