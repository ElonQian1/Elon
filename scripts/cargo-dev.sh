#!/usr/bin/env bash
set -euo pipefail

target_dir_arg=""
no_lock=0
lock_timeout_seconds=3600

usage() {
  cat >&2 <<'EOF'
Usage: scripts/cargo-dev.sh [--target-dir <path>] [--no-lock] [--lock-timeout-seconds <seconds>] <cargo-args...>

Examples:
  bash scripts/cargo-dev.sh check --manifest-path server/Cargo.toml
  bash scripts/cargo-dev.sh test --manifest-path server/Cargo.toml pc_lightweight
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target-dir)
      shift
      if [[ $# -eq 0 ]]; then
        usage
        exit 2
      fi
      target_dir_arg="$1"
      shift
      ;;
    --no-lock)
      no_lock=1
      shift
      ;;
    --lock-timeout-seconds)
      shift
      if [[ $# -eq 0 ]]; then
        usage
        exit 2
      fi
      lock_timeout_seconds="$1"
      shift
      ;;
    --)
      shift
      break
      ;;
    -*)
      break
      ;;
    *)
      break
      ;;
  esac
done

if [[ $# -eq 0 ]]; then
  usage
  exit 2
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(git -C "$script_dir" rev-parse --show-toplevel 2>/dev/null || git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$repo_root" ]]; then
  echo "Current directory is not inside a Git repository." >&2
  exit 1
fi

import_local_env_file() {
  local env_file="$1"
  [[ -f "$env_file" ]] || return 0

  local line name value first last
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line#"${line%%[![:space:]]*}"}"
    line="${line%"${line##*[![:space:]]}"}"
    [[ -z "$line" || "$line" == \#* ]] && continue
    [[ "$line" =~ ^([A-Za-z_][A-Za-z0-9_]*)[[:space:]]*=[[:space:]]*(.*)$ ]] || continue
    name="${BASH_REMATCH[1]}"
    value="${BASH_REMATCH[2]}"
    value="${value#"${value%%[![:space:]]*}"}"
    value="${value%"${value##*[![:space:]]}"}"
    if [[ ${#value} -ge 2 ]]; then
      first="${value:0:1}"
      last="${value: -1}"
      if [[ ("$first" == '"' && "$last" == '"') || ("$first" == "'" && "$last" == "'") ]]; then
        value="${value:1:${#value}-2}"
      fi
    fi
    if [[ -z "${!name:-}" ]]; then
      export "$name=$value"
    fi
  done < "$env_file"
}

import_local_env_file "$repo_root/.env.local"

normalize_target_dir() {
  local path="$1"
  if [[ "$path" =~ ^[A-Za-z]:[\\/].* ]] && command -v cygpath >/dev/null 2>&1; then
    cygpath -u "$path"
    return
  fi
  printf '%s\n' "$path"
}

if [[ -n "$target_dir_arg" ]]; then
  target_dir="$target_dir_arg"
  target_source="--target-dir"
elif [[ -n "${ELON_DEV_CARGO_TARGET_DIR:-}" ]]; then
  target_dir="$ELON_DEV_CARGO_TARGET_DIR"
  target_source="ELON_DEV_CARGO_TARGET_DIR"
elif [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  target_dir="$CARGO_TARGET_DIR"
  target_source="CARGO_TARGET_DIR"
elif [[ -n "${LOCALAPPDATA:-}" ]] && command -v cygpath >/dev/null 2>&1; then
  target_dir="$(cygpath -u "$LOCALAPPDATA")/Elon/build-target/elon-dev-cargo"
  target_source="default LOCALAPPDATA"
else
  target_dir="${XDG_CACHE_HOME:-$HOME/.cache}/elon/build/elon-dev-cargo"
  target_source="default XDG cache"
fi

target_dir="$(normalize_target_dir "$target_dir")"

case "$target_dir" in
  /*) ;;
  *)
    echo "$target_source must be an absolute path, current value: $target_dir" >&2
    exit 1
    ;;
esac

mkdir -p "$target_dir"

lock_dir="$target_dir/.cargo-dev.lockdir"
release_lock() {
  if [[ "$no_lock" -eq 0 && -d "$lock_dir" && -f "$lock_dir/owner" ]]; then
    local owner_pid
    owner_pid="$(sed -n 's/^pid=//p' "$lock_dir/owner" 2>/dev/null | head -n 1 || true)"
    if [[ "$owner_pid" == "$$" ]]; then
      rm -rf "$lock_dir"
    fi
  fi
}
trap release_lock EXIT INT TERM

if [[ "$no_lock" -eq 0 ]]; then
  echo "Waiting for Cargo dev target lock: $lock_dir"
  deadline=$((SECONDS + lock_timeout_seconds))
  while ! mkdir "$lock_dir" 2>/dev/null; do
    if (( SECONDS >= deadline )); then
      echo "Timed out waiting for Cargo dev target lock: $lock_dir" >&2
      if [[ -f "$lock_dir/owner" ]]; then
        cat "$lock_dir/owner" >&2 || true
      fi
      exit 1
    fi
    sleep 2
  done
  {
    echo "pid=$$"
    date -u '+started_utc=%Y-%m-%dT%H:%M:%SZ'
  } > "$lock_dir/owner"
fi

export CARGO_TARGET_DIR="$target_dir"
echo "CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
echo "cargo $*"
cargo "$@"
exit $?
