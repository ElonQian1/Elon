#!/usr/bin/env bash
# Independent IP certificate lifecycle: TLS-ALPN-01 on 443, account HTTPS on 8443.
set -euo pipefail

readonly VERSION="5.4.1"
# The separate installer must verify this official linux_amd64 archive before extraction.
readonly ARCHIVE_SHA256="ebb33f1bead5a7c99dd46f1c5734b44cf1eab5b5c12faf397cd14d50a5916419"
readonly LEGO="/opt/elon-account-acme/lego-v${VERSION}/lego"
readonly STATE_ROOT="/var/lib/elon-account-acme"
readonly PERMANENT_DIR="/opt/elon-account-acme/scripts"
readonly TIMER="elon-account-acme-renew"
readonly MODE="${1:-}"
readonly IP="${2:-}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

fail() { printf 'ACCOUNT_ACME_ERROR=%s\n' "$*" >&2; exit 1; }
[[ $# -eq 2 && "$MODE" =~ ^(plan|staging|issue|renew|install-timer)$ ]] ||
  fail "usage: $0 plan|staging|issue|renew|install-timer public-ipv4"

# Pure validation and plan: do not open certificate files, create state or contact a CA.
plan="$(python3 - "$IP" "$VERSION" "$ARCHIVE_SHA256" <<'PY'
import ipaddress, json, sys
try:
    ip = ipaddress.IPv4Address(sys.argv[1])
    if not ip.is_global or any((ip.is_private, ip.is_loopback, ip.is_link_local,
                               ip.is_multicast, ip.is_reserved, ip.is_unspecified)):
        raise ValueError()
except ValueError:
    raise SystemExit('ACCOUNT_ACME_ERROR=public IPv4 required')
print(json.dumps({
    'schema': 'elon.account-acme.plan.v1', 'operation': 'plan',
    'ipv4': str(ip), 'lego_version': sys.argv[2], 'archive_sha256': sys.argv[3],
    'challenge': 'tls-alpn-01', 'challenge_port': 443, 'business_port': 8443,
    'business_origin': f'https://{ip}:8443', 'http_80_enabled': False,
    'profile': 'shortlived', 'default_operation': 'issue', 'staging': False,
    'staging_state': '/var/lib/elon-account-acme/staging',
    'production_state': '/var/lib/elon-account-acme/production',
    'contact_email': None, 'renew_command': 'run', 'renew_policy': 'default_ari_dynamic',
    'timeout_seconds': 300, 'timer_hours': 6, 'timer_jitter_seconds': 600,
    'deployment_mutated': False, 'certificate_issuance_performed': False,
}, separators=(',', ':')))
PY
)" || fail "invalid target"
if [[ "$MODE" == plan ]]; then printf '%s\n' "$plan"; exit; fi

[[ $EUID -eq 0 ]] || fail "root required"
umask 077
# No inherited proxy, alternate trust store, or LEGO_HTTP/LEGO_TLS_SKIP_VERIFY authority.
unset HTTP_PROXY HTTPS_PROXY ALL_PROXY http_proxy https_proxy all_proxy
unset SSL_CERT_FILE SSL_CERT_DIR CURL_CA_BUNDLE REQUESTS_CA_BUNDLE
while IFS= read -r name; do unset "$name"; done < <(compgen -e | grep '^LEGO_' || true)
for tool in flock timeout openssl stat install cmp; do
  command -v "$tool" >/dev/null || fail "required command unavailable: $tool"
done

private_directory() {
  local path="$1"
  [[ ! -L "$path" ]] || fail "state directory cannot be a symlink"
  if [[ -e "$path" ]]; then
    [[ -d "$path" && "$(stat -c %u "$path")" == 0 ]] || fail "state must be a root-owned directory"
  fi
  install -d -m 0700 -o root -g root -- "$path"
}
private_directory "$STATE_ROOT"
lock="$STATE_ROOT/operation.lock"
[[ ! -L "$lock" ]] || fail "lock cannot be a symlink"
if [[ -e "$lock" ]]; then
  [[ -f "$lock" && "$(stat -c %u "$lock")" == 0 ]] || fail "invalid lock ownership"
fi
exec 9>>"$lock"
chmod 0600 "$lock"
flock -w 10 9 || fail "another account certificate operation is running"

[[ -f "$LEGO" && -x "$LEGO" && ! -L "$LEGO" ]] || fail "install the pinned lego binary first"
[[ "$(stat -c %u "$LEGO")" == 0 ]] || fail "lego must be root-owned"
binary_mode="$(stat -c %a "$LEGO")"
(( (8#$binary_mode & 0022) == 0 )) || fail "lego cannot be writable by group or others"
version_output="$(timeout --kill-after=2s 10s "$LEGO" --version)" || fail "lego version check failed"
[[ "$version_output" =~ ^lego\ version\ 5\.4\.1([[:space:]]|$) ]] || fail "unexpected lego version"

environment=production
directory="https://acme-v02.api.letsencrypt.org/directory"
if [[ "$MODE" == staging ]]; then
  environment=staging
  directory="https://acme-staging-v02.api.letsencrypt.org/directory"
fi
state="$STATE_ROOT/$environment"
private_directory "$state"
certificate="$state/certificates/$IP.crt"
private_key="$state/certificates/$IP.key"
resource="$state/certificates/$IP.json"

check_production() {
  [[ -f "$SCRIPT_DIR/configure-account-https.sh" && -f "$SCRIPT_DIR/account_https_target.py" ]] ||
    fail "deploy the matching HTTPS check scripts first"
  timeout --kill-after=2s 30s bash "$SCRIPT_DIR/configure-account-https.sh" \
    check "https://$IP:8443" --certificate-path "$certificate" --private-key-path "$private_key"
}

install_timer() {
  check_production
  command -v systemctl >/dev/null || fail "systemd is required"
  # The timer must never retain a worktree, temporary directory or mismatched script revision.
  for name in configure-account-acme.sh configure-account-https.sh account_https_target.py; do
    [[ -f "$PERMANENT_DIR/$name" && ! -L "$PERMANENT_DIR/$name" ]] &&
      cmp -s -- "$SCRIPT_DIR/$name" "$PERMANENT_DIR/$name" || fail "deploy the matching permanent scripts first"
    [[ "$(stat -c %u "$PERMANENT_DIR/$name")" == 0 ]] || fail "permanent scripts must be root-owned"
    local mode
    mode="$(stat -c %a "$PERMANENT_DIR/$name")"
    (( (8#$mode & 0022) == 0 )) || fail "permanent scripts cannot be writable by group or others"
  done
  local service_path="/etc/systemd/system/$TIMER.service"
  local timer_path="/etc/systemd/system/$TIMER.timer"
  [[ ! -L "$service_path" && ! -L "$timer_path" ]] || fail "timer units cannot be symlinks"
  timer_temporary="$(mktemp -d /etc/systemd/system/.elon-account-acme.XXXXXXXX)"
  trap 'rm -f -- "$timer_temporary/service" "$timer_temporary/timer"; rmdir -- "$timer_temporary"' EXIT
  cat >"$timer_temporary/service" <<EOF
[Unit]
Description=Renew the Elon account IP certificate with TLS-ALPN-01
After=network-online.target
Wants=network-online.target
[Service]
Type=oneshot
User=root
Group=root
UMask=0077
ExecStart=/bin/bash $PERMANENT_DIR/configure-account-acme.sh renew $IP
TimeoutStartSec=360
KillMode=control-group
EOF
  cat >"$timer_temporary/timer" <<'EOF'
[Unit]
Description=Check the Elon account IP certificate every six hours
[Timer]
OnCalendar=*-*-* 00/6:17:00
RandomizedDelaySec=600
Persistent=true
[Install]
WantedBy=timers.target
EOF
  install -m 0644 -o root -g root "$timer_temporary/service" "$service_path"
  install -m 0644 -o root -g root "$timer_temporary/timer" "$timer_path"
  systemctl daemon-reload
  systemctl enable --now "$TIMER.timer"
  printf 'ACCOUNT_ACME_STATUS=timer_installed; main_service_restarted=false\n'
}

if [[ "$MODE" == install-timer ]]; then install_timer; exit; fi
if [[ "$MODE" == renew ]]; then
  [[ -s "$certificate" && -s "$private_key" && -s "$resource" ]] ||
    fail "production certificate state missing; renewal cannot silently become first issuance"
fi

# v5 run obtains once, then renews according to ARI/default dynamic lifetime checks.
# The outer timer supplies jitter; no internal random sleep competes with the 300s limit.
(
  cd -- "$state"
  timeout --kill-after=10s 300s "$LEGO" run --path "$state" --server "$directory" \
    --account-id elon-account-ip --email '' --accept-tos --domains "$IP" \
    --profile shortlived --tls --tls.address 0.0.0.0:443 --http=false \
    --env-file /dev/null --no-random-sleep
) || fail "lego operation failed or timed out; production activation was not performed"

if [[ "$MODE" == staging ]]; then
  openssl x509 -in "$certificate" -checkip "$IP" -noout >/dev/null || fail "staging IP identity check failed"
  openssl x509 -in "$certificate" -checkend 86400 -noout >/dev/null || fail "staging certificate lifetime check failed"
  printf 'ACCOUNT_ACME_STATUS=staging_certificate_checked; production_trust_verified=false\n'
else
  check_production
  printf 'ACCOUNT_ACME_STATUS=production_certificate_checked; main_service_restarted=false\n'
fi
