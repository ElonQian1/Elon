#!/usr/bin/env bash
# Deploy the matching committed server through publish-server before enabling this flag.
set -euo pipefail
unset HTTP_PROXY HTTPS_PROXY ALL_PROXY http_proxy https_proxy all_proxy
MODE="${1:-}"
TARGET="${2:-}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE=/root/Elon/server/.env
SERVICE=elon-server
fail() { printf 'PWA_HTTPS_ERROR=%s\n' "$1" >&2; exit 1; }
[[ $# == 2 && "$MODE" =~ ^(plan|verify|enable|disable)$ ]] || fail invalid_arguments
target_json="$(python3 "$SCRIPT_DIR/account_https_target.py" "$TARGET" --format json)" || fail invalid_origin
ORIGIN="$(printf '%s' "$target_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["origin"])')"
if [[ "$MODE" == plan ]]; then
  printf 'PWA_HTTPS_PLAN=existing_browser_router_on_native_tls; new_listener=false; new_certificate=false; flag=MOBILE_PWA_HTTPS_ENABLED\n'
  exit
fi
code() { curl --silent --show-error --max-time 10 --output /dev/null --write-out '%{http_code}' "$ORIGIN$1"; }
verify_account() {
  curl --silent --show-error --fail --max-time 10 "$ORIGIN/health" |
    python3 -c 'import json,sys; assert json.load(sys.stdin)["service"] == "elon-account-https"' || return 1
  [[ "$(code /api/me)" == 401 ]] || return 1
  [[ "$(code '/api/me?token=probe')" == 404 ]] || return 1
  [[ "$(curl --silent --show-error --fail --max-time 5 http://127.0.0.1:8080/health)" == OK ]] || return 1
}
verify_pwa() {
  local headers
  headers="$(curl --silent --show-error --fail --max-time 15 -D - -o /dev/null "$ORIGIN/web")" || return 1
  printf '%s' "$headers" | grep -qi '^x-elon-pwa-transport: https-v1' || return 1
  printf '%s' "$headers" | grep -qi '^x-elon-mobile-pwa-source: runtime' || return 1
  [[ "$(code /manifest.json)" == 200 ]] || return 1
  [[ "$(code /sw.js)" == 200 ]] || return 1
  [[ "$(code /api/me/groups)" == 401 ]] || return 1
  [[ "$(code /api/me/friends)" == 401 ]] || return 1
}
verify_account || fail existing_ingress_verification_failed
if [[ "$MODE" == verify ]]; then
  verify_pwa || fail pwa_verification_failed
  printf 'PWA_HTTPS_STATUS=verified; url=%s/web; physical_ios_acceptance=separate\n' "$ORIGIN"
  exit
fi
[[ $EUID == 0 ]] || fail root_required
exec 9>/tmp/elon-deploy.lock
flock -x -w 120 9 || fail deployment_busy
before="$(python3 "$SCRIPT_DIR/mobile_pwa_https_config.py" inspect "$ENV_FILE")" || fail invalid_environment
desired=true
[[ "$MODE" == disable ]] && desired=false
if [[ "$before" == "$desired" ]]; then
  if [[ "$desired" == true ]]; then verify_pwa || fail enabled_but_unavailable
  else [[ "$(code /web)" == 404 ]] || fail disabled_but_available; fi
  printf 'PWA_HTTPS_STATUS=already_configured; service_restarted=false\n'
  exit
fi
changed=false
rollback() {
  local status=$?
  trap - EXIT
  if [[ $status != 0 && "$changed" == true ]]; then
    if python3 "$SCRIPT_DIR/mobile_pwa_https_config.py" set "$ENV_FILE" "$desired" "$before"; then
      systemctl restart "$SERVICE" || true
      printf 'PWA_HTTPS_STATUS=activation_failed_flag_restored\n' >&2
    else
      printf 'PWA_HTTPS_STATUS=rollback_requires_operator_configuration_changed\n' >&2
    fi
  fi
  exit "$status"
}
trap rollback EXIT
python3 "$SCRIPT_DIR/mobile_pwa_https_config.py" set "$ENV_FILE" "$before" "$desired"
changed=true
systemctl restart "$SERVICE"
for attempt in {1..20}; do
  if curl --silent --fail --max-time 2 "$ORIGIN/health" >/dev/null; then break; fi
  sleep 1
done
verify_account || fail account_verification_failed
if [[ "$desired" == true ]]; then verify_pwa || fail pwa_verification_failed
else [[ "$(code /web)" == 404 ]] || fail disabled_but_available; fi
printf 'PWA_HTTPS_STATUS=configured_verified; url=%s/web; legacy_http_preserved=true; certificate_configuration_changed=false\n' "$ORIGIN"
