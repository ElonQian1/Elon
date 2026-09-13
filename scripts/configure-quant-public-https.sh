#!/usr/bin/env bash
# Enable only after the matching server commit has been published normally.
set -euo pipefail
unset HTTP_PROXY HTTPS_PROXY ALL_PROXY http_proxy https_proxy all_proxy
MODE="${1:-}"
TARGET="${2:-}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE=/root/Elon/server/.env
SERVICE=elon-server
fail() { printf 'QUANT_HTTPS_ERROR=%s\n' "$1" >&2; exit 1; }
[[ $# == 2 && "$MODE" =~ ^(plan|verify|enable|disable)$ ]] || fail invalid_arguments
target_json="$(python3 "$SCRIPT_DIR/account_https_target.py" "$TARGET" --format json)" || fail invalid_origin
ORIGIN="$(printf '%s' "$target_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["origin"])')"
if [[ "$MODE" == plan ]]; then
  printf 'QUANT_HTTPS_PLAN=existing_native_tls; new_listener=false; new_certificate=false; configuration_key=QUANT_PUBLIC_HTTPS_ENABLED\n'
  exit
fi
code() { curl --silent --show-error --max-time 10 --output /dev/null --write-out '%{http_code}' "$ORIGIN$1"; }
verify_account() {
  curl --silent --show-error --fail --max-time 10 "$ORIGIN/health" |
    python3 -c 'import json,sys; assert json.load(sys.stdin)["service"] == "elon-account-https"' || return 1
  [[ "$(code /api/me)" == 401 ]] || return 1
  [[ "$(code '/api/me?token=probe')" == 404 ]] || return 1
  for route in /api/nodes /mcp /quant/api/v1/grid-center/robots /quant/api/v1/paper/orders; do
    [[ "$(code "$route")" == 404 ]] || return 1
  done
}
verify_public() {
  local headers
  headers="$(curl --silent --show-error --fail --max-time 10 -D - -o /dev/null "$ORIGIN/quant/?app_section=market")" || return 1
  printf '%s' "$headers" | grep -qi '^x-yilong-quant-transport: https-public-v1' || return 1
  printf '%s' "$headers" | grep -qi '^content-security-policy:' || return 1
  [[ "$(code /quant/api/v1/runtime)" == 200 ]] || return 1
}
verify_account || fail account_tls_verification_failed
if [[ "$MODE" == verify ]]; then
  verify_public || fail public_surface_verification_failed
  printf 'QUANT_HTTPS_STATUS=verified; market_data_and_apk_acceptance=separate\n'
  exit
fi
[[ $EUID == 0 ]] || fail root_required
# Share the official server swap lock; never race a binary publication.
exec 9>/tmp/elon-deploy.lock
flock -x -w 120 9 || fail deployment_busy
before="$(python3 "$SCRIPT_DIR/quant_public_https_config.py" inspect "$ENV_FILE")" || fail invalid_environment
desired=true
[[ "$MODE" == disable ]] && desired=false
if [[ "$before" == "$desired" ]]; then
  if [[ "$desired" == true ]]; then verify_public || fail enabled_but_unavailable
  else [[ "$(code /quant/)" == 404 ]] || fail disabled_but_available; fi
  printf 'QUANT_HTTPS_STATUS=already_configured; service_restarted=false\n'
  exit
fi
changed=false
rollback() {
  local status=$?
  trap - EXIT
  if [[ $status != 0 && "$changed" == true ]]; then
    if python3 "$SCRIPT_DIR/quant_public_https_config.py" set "$ENV_FILE" "$desired" "$before"; then
      systemctl restart "$SERVICE" || true
      printf 'QUANT_HTTPS_STATUS=activation_failed_flag_restored\n' >&2
    else
      printf 'QUANT_HTTPS_STATUS=rollback_requires_operator_configuration_changed\n' >&2
    fi
  fi
  exit "$status"
}
trap rollback EXIT
python3 "$SCRIPT_DIR/quant_public_https_config.py" set "$ENV_FILE" "$before" "$desired"
changed=true
systemctl restart "$SERVICE"
for attempt in {1..20}; do
  if curl --silent --fail --max-time 2 "$ORIGIN/health" >/dev/null; then break; fi
  sleep 1
done
verify_account || fail account_verification_failed
if [[ "$desired" == true ]]; then verify_public || fail public_verification_failed
else [[ "$(code /quant/)" == 404 ]] || fail public_route_still_enabled; fi
printf 'QUANT_HTTPS_STATUS=configured_verified; account_policy_preserved=true; certificate_configuration_changed=false\n'
