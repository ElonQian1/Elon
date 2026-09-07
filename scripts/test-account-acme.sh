#!/usr/bin/env bash
# Pure-plan contract checks; no installation, certificate requests or production state.
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
export PYTHONDONTWRITEBYTECODE=1
if [[ -n "${ELON_TEST_PYTHON:-}" ]]; then
  python3() {
    local script="$1"
    shift
    if [[ "$script" == /* ]]; then script="$(cygpath -w "$script")"; fi
    MSYS2_ARG_CONV_EXCL='*' "$ELON_TEST_PYTHON" "$script" "$@" | sed 's/\r$//'
    return "${PIPESTATUS[0]}"
  }
  export -f python3
fi
bash -n "$SCRIPT_DIR/configure-account-acme.sh"
bash -n "$SCRIPT_DIR/install-account-acme.sh"
plan="$(bash "$SCRIPT_DIR/configure-account-acme.sh" plan 43.139.149.158)"
python3 -c 'import json,sys
p=json.load(sys.stdin)
assert p["ipv4"] == "43.139.149.158"
assert p["challenge"] == "tls-alpn-01" and p["challenge_port"] == 443
assert p["business_origin"] == "https://43.139.149.158:8443"
assert p["http_80_enabled"] is False
assert p["profile"] == "shortlived" and p["lego_version"] == "5.4.1"
assert p["archive_sha256"] == "ebb33f1bead5a7c99dd46f1c5734b44cf1eab5b5c12faf397cd14d50a5916419"
assert p["staging_state"] != p["production_state"]
assert p["renew_command"] == "run" and p["renew_policy"] == "default_ari_dynamic"
assert p["timer_hours"] == 6 and p["timer_jitter_seconds"] == 600
assert p["contact_email"] is None
assert p["deployment_mutated"] is False and p["certificate_issuance_performed"] is False
' <<<"$plan"
passed=1
for target in 127.0.0.1 10.0.0.1 0.0.0.0 169.254.169.254 224.0.0.1 \
  example.com https://43.139.149.158:8443 43.139.149.158:443 '43.139.149.158;echo' ''; do
  if bash "$SCRIPT_DIR/configure-account-acme.sh" plan "$target" >/dev/null 2>&1; then
    printf 'Invalid target unexpectedly accepted\n' >&2
    exit 1
  fi
  passed=$((passed + 1))
done
if bash "$SCRIPT_DIR/configure-account-acme.sh" plan 43.139.149.158 --http >/dev/null 2>&1; then
  printf 'Unexpected override accepted\n' >&2
  exit 1
fi
passed=$((passed + 1))
printf 'ACCOUNT_ACME_PLAN_CHECKS=%s passed; deployment_mutated=false\n' "$passed"
