#!/usr/bin/env bash
set -euo pipefail

SERVICE_USER="yilong-edge"
SERVICE_GROUP="yilong-edge"
INSTALL_BINARY="/usr/local/bin/yilong-commerce-edge"
INSTALL_CONFIG="/etc/yilong-commerce-edge/edge.json"
INSTALL_ENV="/etc/yilong-commerce-edge/edge.env"
INSTALL_SERVICE="/etc/systemd/system/yilong-commerce-edge.service"
STATE_DIR="/var/lib/yilong-commerce-edge"
BACKUP_ROOT="/var/backups/yilong-commerce-edge"

binary_source=""
config_source=""
service_source="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/systemd/yilong-commerce-edge.service"
apply=false

usage() {
  cat <<'USAGE'
Usage: install-commerce-edge.sh --binary PATH --config PATH [--service PATH] [--apply]

Without --apply, validates source paths and prints the installation plan only.
USAGE
}

fail() {
  printf 'COMMERCE_EDGE_INSTALL_ERROR=%s\n' "$1" >&2
  exit 1
}

require_value() {
  [[ $# -ge 2 && -n "$2" ]] || fail "MISSING_ARGUMENT_VALUE"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)
      require_value "$@"
      binary_source="$2"
      shift 2
      ;;
    --config)
      require_value "$@"
      config_source="$2"
      shift 2
      ;;
    --service)
      require_value "$@"
      service_source="$2"
      shift 2
      ;;
    --apply)
      apply=true
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      fail "UNKNOWN_ARGUMENT"
      ;;
  esac
done

[[ -n "$binary_source" ]] || fail "BINARY_REQUIRED"
[[ -n "$config_source" ]] || fail "CONFIG_REQUIRED"

resolve_regular_file() {
  local source_path="$1"
  local label="$2"
  [[ ! -L "$source_path" ]] || fail "${label}_SYMLINK_FORBIDDEN"
  [[ -f "$source_path" ]] || fail "${label}_NOT_FOUND"
  realpath -- "$source_path"
}

binary_source="$(resolve_regular_file "$binary_source" BINARY)"
config_source="$(resolve_regular_file "$config_source" CONFIG)"
service_source="$(resolve_regular_file "$service_source" SERVICE)"

cat <<PLAN
COMMERCE_EDGE_INSTALL_MODE=$([[ "$apply" == true ]] && printf apply || printf preview)
COMMERCE_EDGE_INSTALL_BINARY=$INSTALL_BINARY
COMMERCE_EDGE_INSTALL_CONFIG=$INSTALL_CONFIG
COMMERCE_EDGE_INSTALL_SERVICE=$INSTALL_SERVICE
COMMERCE_EDGE_INSTALL_STATE_DIR=$STATE_DIR
COMMERCE_EDGE_INSTALL_WILL_RESTART=$apply
PLAN

if [[ "$apply" != true ]]; then
  printf 'COMMERCE_EDGE_INSTALL_RESULT=preview_only\n'
  exit 0
fi

[[ "$(uname -s)" == "Linux" ]] || fail "LINUX_REQUIRED"
[[ "${EUID:-$(id -u)}" -eq 0 ]] || fail "ROOT_REQUIRED"
for command_name in getent groupadd useradd install runuser systemctl realpath; do
  command -v "$command_name" >/dev/null 2>&1 || fail "COMMAND_MISSING_${command_name^^}"
done

if ! getent group "$SERVICE_GROUP" >/dev/null; then
  groupadd --system "$SERVICE_GROUP"
fi
if ! id "$SERVICE_USER" >/dev/null 2>&1; then
  useradd --system --gid "$SERVICE_GROUP" --home-dir "$STATE_DIR" --shell /usr/sbin/nologin "$SERVICE_USER"
fi

install -d -o root -g "$SERVICE_GROUP" -m 0750 /etc/yilong-commerce-edge
install -d -o "$SERVICE_USER" -g "$SERVICE_GROUP" -m 0700 "$STATE_DIR"
install -d -o root -g root -m 0700 "$BACKUP_ROOT"
install -d -o root -g root -m 0755 /usr/local/libexec

candidate_suffix="$(date -u +%Y%m%dT%H%M%SZ)-$$"
candidate_binary="/usr/local/libexec/yilong-commerce-edge.candidate.$candidate_suffix"
candidate_config="/etc/yilong-commerce-edge/edge.candidate.$candidate_suffix.json"
candidate_env="/etc/yilong-commerce-edge/edge.candidate.$candidate_suffix.env"
cleanup_candidates() {
  rm -f -- "$candidate_binary" "$candidate_config" "$candidate_env"
}
trap cleanup_candidates EXIT

install -o root -g root -m 0755 "$binary_source" "$candidate_binary"
install -o root -g "$SERVICE_GROUP" -m 0640 "$config_source" "$candidate_config"
printf 'YILONG_COMMERCE_EDGE_CONFIG_PATH=%s\nRUST_LOG=yilong_commerce_edge=info\n' \
  "$candidate_config" >"$candidate_env"
chown root:"$SERVICE_GROUP" "$candidate_env"
chmod 0640 "$candidate_env"

runuser -u "$SERVICE_USER" -- env \
  "YILONG_COMMERCE_EDGE_CONFIG_PATH=$candidate_config" \
  "$candidate_binary" --check-config

backup_dir="$BACKUP_ROOT/$candidate_suffix"
install -d -o root -g root -m 0700 "$backup_dir"
service_was_active=false
if systemctl is-active --quiet yilong-commerce-edge.service; then
  service_was_active=true
fi
service_was_enabled=false
if systemctl is-enabled --quiet yilong-commerce-edge.service; then
  service_was_enabled=true
fi
for existing in "$INSTALL_BINARY" "$INSTALL_CONFIG" "$INSTALL_ENV" "$INSTALL_SERVICE"; do
  if [[ -e "$existing" ]]; then
    cp -a -- "$existing" "$backup_dir/"
  fi
done

rollback_required=false
finish_install() {
  local status=$?
  trap - EXIT
  if [[ "$status" -ne 0 && "$rollback_required" == true ]]; then
    set +e
    if [[ "$service_was_enabled" != true ]]; then
      systemctl disable yilong-commerce-edge.service
    fi
    for installed in "$INSTALL_BINARY" "$INSTALL_CONFIG" "$INSTALL_ENV" "$INSTALL_SERVICE"; do
      backup_file="$backup_dir/$(basename -- "$installed")"
      if [[ -e "$backup_file" ]]; then
        cp -a -- "$backup_file" "$installed"
      else
        rm -f -- "$installed"
      fi
    done
    systemctl daemon-reload
    if [[ "$service_was_active" == true ]]; then
      systemctl restart yilong-commerce-edge.service
    else
      systemctl stop yilong-commerce-edge.service
    fi
    printf 'COMMERCE_EDGE_INSTALL_ROLLBACK=attempted\n' >&2
  fi
  cleanup_candidates
  exit "$status"
}
trap finish_install EXIT

rollback_required=true
install -o root -g root -m 0755 "$candidate_binary" "$INSTALL_BINARY"
install -o root -g "$SERVICE_GROUP" -m 0640 "$candidate_config" "$INSTALL_CONFIG"
printf 'YILONG_COMMERCE_EDGE_CONFIG_PATH=%s\nRUST_LOG=yilong_commerce_edge=info\n' \
  "$INSTALL_CONFIG" >"$INSTALL_ENV"
chown root:"$SERVICE_GROUP" "$INSTALL_ENV"
chmod 0640 "$INSTALL_ENV"
install -o root -g root -m 0644 "$service_source" "$INSTALL_SERVICE"

systemctl daemon-reload
systemctl enable yilong-commerce-edge.service
systemctl restart yilong-commerce-edge.service
systemctl is-active --quiet yilong-commerce-edge.service || fail "SERVICE_NOT_ACTIVE"
rollback_required=false

printf 'COMMERCE_EDGE_INSTALL_BACKUP=%s\n' "$backup_dir"
printf 'COMMERCE_EDGE_INSTALL_RESULT=applied\n'
