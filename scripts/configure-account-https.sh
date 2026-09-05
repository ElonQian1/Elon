#!/usr/bin/env bash
# Account-only Rust TLS, independently switchable from legacy HTTP ingress.
set -euo pipefail
# Certificate operations must not inherit unrelated local proxy configuration.
unset HTTP_PROXY HTTPS_PROXY ALL_PROXY http_proxy https_proxy all_proxy PIP_PROXY
MODE="${1:-}"
IP="${2:-}"
ENV_FILE="/root/Elon/server/.env"
SERVICE="elon-server"
CERTBOT="/opt/elon-account-certbot/bin/certbot"

fail() { printf 'ACCOUNT_HTTPS_ERROR=%s\n' "$*" >&2; exit 1; }
[[ "$MODE" =~ ^(prepare|activate|verify)$ ]] || fail "usage: $0 prepare|activate|verify public-ipv4"
[[ $EUID -eq 0 ]] || fail "root required"
python3 - "$IP" <<'PY'
import ipaddress, sys
address = ipaddress.ip_address(sys.argv[1])
assert address.version == 4 and address.is_global, 'public IPv4 required'
PY
CERT="/etc/letsencrypt/live/$IP/fullchain.pem"
KEY="/etc/letsencrypt/live/$IP/privkey.pem"

verify() {
  local code
  curl --silent --show-error --fail --max-time 10 "https://$IP/health" |
    python3 -c 'import json,sys; assert json.load(sys.stdin)["service"] == "elon-account-https"'
  for path in /api/me /api/auth/security; do
    code="$(curl --silent --show-error --max-time 10 -o /dev/null -w '%{http_code}' "https://$IP$path")"
    [[ "$code" == 401 ]] || fail "$path expected 401, received $code"
  done
  for path in /api/admin/users /mcp /api/nodes; do
    code="$(curl --silent --show-error --max-time 10 -o /dev/null -w '%{http_code}' "https://$IP$path")"
    [[ "$code" == 404 ]] || fail "$path unexpectedly published: $code"
  done
  curl --silent --show-error --fail --max-time 10 http://127.0.0.1:8080/health >/dev/null
  printf 'ACCOUNT_HTTPS_STATUS=verified\n'
}

if [[ "$MODE" == verify ]]; then verify; exit; fi

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
    --ip-address "$IP" --cert-name "$IP" --keep-until-expiring
  openssl x509 -in "$CERT" -checkend 86400 -noout
  cat >/etc/systemd/system/elon-account-cert-renew.service <<EOF
[Unit]
Description=Renew Elon account IP certificate
After=network-online.target
Wants=network-online.target
[Service]
Type=oneshot
ExecStart=$CERTBOT renew --cert-name $IP --quiet
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
openssl x509 -in "$CERT" -checkend 86400 -noout
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
python3 - "$ENV_FILE" "$CERT" "$KEY" <<'PY'
import os, pathlib, sys, tempfile
path = pathlib.Path(sys.argv[1])
updates = {
    'ACCOUNT_HTTPS_ENABLED': 'true',
    'ACCOUNT_HTTPS_LISTEN_ADDR': '0.0.0.0:443',
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
  if curl --silent --fail --max-time 2 "https://$IP/health" >/dev/null; then break; fi
  sleep 1
done
verify
