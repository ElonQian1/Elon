#!/usr/bin/env bash
# Account-only Rust TLS, independently switchable from legacy HTTP ingress.
set -euo pipefail
# Certificate operations must not inherit unrelated local proxy configuration.
unset HTTP_PROXY HTTPS_PROXY ALL_PROXY http_proxy https_proxy all_proxy PIP_PROXY
MODE="${1:-}"
TARGET="${2:-}"
ENV_FILE="/root/Elon/server/.env"
SERVICE="elon-server"
CERTBOT="/opt/elon-account-certbot/bin/certbot"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

fail() { printf 'ACCOUNT_HTTPS_ERROR=%s\n' "$*" >&2; exit 1; }
[[ "$MODE" =~ ^(plan|check|prepare|activate|verify)$ && $# -ge 2 ]] ||
  fail "usage: $0 plan|check|prepare|activate|verify public-ipv4-or-https-origin [--certificate-path PEM --private-key-path PEM]"
shift 2
if [[ "$MODE" == plan ]]; then
  python3 "$SCRIPT_DIR/account_https_target.py" "$TARGET" "$@" --format json
  exit
fi
target_lines="$(python3 "$SCRIPT_DIR/account_https_target.py" "$TARGET" "$@" --format lines)" ||
  fail "invalid HTTPS target"
mapfile -t target_config <<<"$target_lines"
[[ ${#target_config[@]} -eq 7 ]] || fail "invalid target configuration"
ORIGIN="${target_config[0]}"
HOST="${target_config[1]}"
PORT="${target_config[2]}"
LISTEN="${target_config[3]}"
CERT="${target_config[4]}"
KEY="${target_config[5]}"
CERT_SOURCE="${target_config[6]}"

check_certificate() {
  [[ -r "$CERT" && -r "$KEY" ]] || fail "certificate chain and private key must be readable"
  # rustls-pemfile does not decrypt PEM, including a PKCS#8 key encrypted with an empty password.
  if ! grep -qE '^-----BEGIN (RSA |EC )?PRIVATE KEY-----[[:space:]]*$' "$KEY" ||
    grep -qiE '^-----BEGIN ENCRYPTED PRIVATE KEY-----|^Proc-Type:.*ENCRYPTED' "$KEY"; then
    fail "the native TLS service requires an unencrypted private key PEM"
  fi
  openssl x509 -in "$CERT" -checkend 86400 -noout >/dev/null ||
    fail "certificate expires within one day or cannot be parsed"
  local identity_flag="-verify_hostname" cert_public key_public
  [[ "$HOST" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]] && identity_flag="-verify_ip"
  openssl verify -purpose sslserver "$identity_flag" "$HOST" \
    -untrusted "$CERT" "$CERT" >/dev/null || fail "certificate must be trusted and match the target host"
  cert_public="$(openssl x509 -in "$CERT" -pubkey -noout)" || fail "cannot read certificate public key"
  key_public="$(openssl pkey -in "$KEY" -passin pass: -pubout 2>/dev/null)" ||
    fail "private key cannot be parsed"
  [[ "$cert_public" == "$key_public" ]] || fail "certificate and private key do not match"
}

if [[ "$MODE" == check ]]; then
  check_certificate
  printf 'ACCOUNT_HTTPS_STATUS=certificate_checked_runtime_unchanged\n'
  exit
fi

verify() {
  local code
  curl --silent --show-error --fail --max-time 10 "$ORIGIN/health" |
    python3 -c 'import json,sys; assert json.load(sys.stdin)["service"] == "elon-account-https"'
  for path in /api/me /api/auth/security /api/asset-access/grids; do
    code="$(curl --silent --show-error --max-time 10 -o /dev/null -w '%{http_code}' "$ORIGIN$path")"
    [[ "$code" == 401 ]] || fail "$path expected 401, received $code"
  done
  for path in /api/admin/users /mcp /api/nodes; do
    code="$(curl --silent --show-error --max-time 10 -o /dev/null -w '%{http_code}' "$ORIGIN$path")"
    [[ "$code" == 404 ]] || fail "$path unexpectedly published: $code"
  done
  curl --silent --show-error --fail --max-time 10 http://127.0.0.1:8080/health >/dev/null
  printf 'ACCOUNT_HTTPS_STATUS=verified\n'
}

if [[ "$MODE" == verify ]]; then verify; exit; fi
if [[ "$MODE" == prepare && "$CERT_SOURCE" != managed_ip ]]; then
  fail "external certificate issuance and renewal belong to its provider; use check, then activate"
fi
if [[ "$MODE" == activate && "$PORT" == 8080 ]]; then
  fail "port 8080 is reserved for existing HTTP and WebSocket clients"
fi
[[ $EUID -eq 0 ]] || fail "root required for prepare or activate"

if [[ "$MODE" == prepare ]]; then
  # Standalone challenges temporarily bind port 80; no reverse proxy is installed.
  [[ -z "$(ss -H -ltn 'sport = :80')" ]] || fail "port 80 must be free for standalone ACME"
  if [[ ! -x "$CERTBOT" ]]; then
    python3 -m venv /opt/elon-account-certbot
    timeout 240 /opt/elon-account-certbot/bin/pip --isolated install \
      --index-url https://pypi.org/simple --disable-pip-version-check \
      --no-cache-dir --timeout 30 --retries 1 'certbot==5.4.0'
  fi
  "$CERTBOT" --version
  timeout 300 "$CERTBOT" certonly --standalone --non-interactive --agree-tos \
    --register-unsafely-without-email --preferred-profile shortlived \
    --ip-address "$HOST" --cert-name "$HOST" --keep-until-expiring
  check_certificate
  cat >/etc/systemd/system/elon-account-cert-renew.service <<EOF
[Unit]
Description=Renew Elon account IP certificate
After=network-online.target
Wants=network-online.target
[Service]
Type=oneshot
ExecStart=$CERTBOT renew --cert-name $HOST --quiet
TimeoutStartSec=300
EOF
  cat >/etc/systemd/system/elon-account-cert-renew.timer <<'EOF'
[Unit]
Description=Check short-lived Elon account certificate every six hours
[Timer]
OnCalendar=*-*-* 00/6:17:00
RandomizedDelaySec=600
Persistent=true
[Install]
WantedBy=timers.target
EOF
  systemctl daemon-reload
  systemctl enable --now elon-account-cert-renew.timer
  printf 'ACCOUNT_HTTPS_STATUS=certificate_prepared_runtime_unchanged\n'
  exit
fi

[[ -f "$ENV_FILE" && -f "$CERT" && -f "$KEY" ]] || fail "run prepare and deploy backend first"
[[ "$(systemctl show "$SERVICE" -p User --value)" == root ]] || fail "review certificate permissions for non-root service first"
check_certificate
python3 - "$ENV_FILE" "$PORT" <<'PY'
import pathlib, sys
from urllib.parse import urlsplit
for raw in pathlib.Path(sys.argv[1]).read_text().splitlines():
    name, separator, value = raw.partition('=')
    if separator and name.strip() in ('LISTEN_ADDR', 'NODE_ENDPOINT_DIRECT_TLS_LISTEN_ADDR'):
        address = value.strip().strip('"\'')
        if address and urlsplit('//' + address).port == int(sys.argv[2]):
            raise SystemExit('ACCOUNT_HTTPS_ERROR=port conflicts with an existing server listener')
PY
listener="$(ss -H -ltnp "sport = :$PORT")"
if [[ -n "$listener" ]]; then
  service_pid="$(systemctl show "$SERVICE" -p MainPID --value)"
  [[ "$service_pid" =~ ^[1-9][0-9]*$ && "$listener" == *"pid=$service_pid,"* ]] ||
    fail "target port belongs to another service"
  # Existing account TLS can be reconfigured; never mistake its legacy HTTP listener for TLS.
  curl --silent --show-error --fail --max-time 10 "$ORIGIN/health" |
    python3 -c 'import json,sys; assert json.load(sys.stdin)["service"] == "elon-account-https"'
fi
BACKUP="$(mktemp /root/Elon/server/.account-https-env-backup.XXXXXXXX)"
cp -p "$ENV_FILE" "$BACKUP"
chmod 0600 "$BACKUP"
rollback() {
  local status=$?
  trap - EXIT
  if [[ $status -ne 0 ]]; then
    cp -p "$BACKUP" "$ENV_FILE"
    systemctl restart "$SERVICE" || true
    printf 'ACCOUNT_HTTPS_STATUS=activation_failed_environment_restored\n' >&2
  fi
  rm -f -- "$BACKUP"
  exit "$status"
}
trap rollback EXIT
python3 - "$ENV_FILE" "$CERT" "$KEY" "$LISTEN" <<'PY'
import os, pathlib, sys, tempfile
path = pathlib.Path(sys.argv[1])
updates = {
    'ACCOUNT_HTTPS_ENABLED': 'true',
    'ACCOUNT_HTTPS_LISTEN_ADDR': sys.argv[4],
    'ACCOUNT_HTTPS_CERTIFICATE_PATH': sys.argv[2],
    'ACCOUNT_HTTPS_PRIVATE_KEY_PATH': sys.argv[3],
}
lines = [line for line in path.read_text().splitlines()
         if line.split('=', 1)[0].strip() not in updates]
lines.extend(f'{key}={value}' for key, value in updates.items())
fd, temporary = tempfile.mkstemp(prefix='.account-https-env-', dir=path.parent)
try:
    with os.fdopen(fd, 'w') as stream:
        stream.write('\n'.join(lines) + '\n')
        stream.flush()
        os.fsync(stream.fileno())
    os.chmod(temporary, 0o600)
    os.replace(temporary, path)
finally:
    if os.path.exists(temporary): os.unlink(temporary)
PY
systemctl restart "$SERVICE"
for attempt in {1..20}; do
  if curl --silent --fail --max-time 2 "$ORIGIN/health" >/dev/null; then break; fi
  sleep 1
done
verify
