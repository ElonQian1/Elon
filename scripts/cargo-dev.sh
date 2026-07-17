#!/usr/bin/env bash
set -euo pipefail

target_dir=""
domain="dev-windows-msvc"
no_lock=0
disable_sccache=0
skip_cache_gc=0
lock_timeout_seconds=3600

usage() {
  cat >&2 <<'EOF'
Usage: scripts/cargo-dev.sh [platform-options] <cargo-args...>

Platform options:
  --target-dir <path>             Override the final-artifact directory.
  --domain <name>                 Select a compatibility domain.
  --no-lock                       Do not lock the managed build partition.
  --disable-sccache               Run without the compiler object cache.
  --skip-cache-gc                 Skip the preflight disk-watermark check.
  --lock-timeout-seconds <value>  Partition-lock timeout (default: 3600).

On Windows/Git Bash this is a thin adapter to cargo-dev.ps1 so every shell uses
the same machine-wide Rust cache policy. Other hosts keep Cargo's workspace-local
target directory and use sccache when it is available.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target-dir)
      [[ $# -ge 2 ]] || { usage; exit 2; }
      target_dir="$2"
      shift 2
      ;;
    --domain)
      [[ $# -ge 2 ]] || { usage; exit 2; }
      domain="$2"
      shift 2
      ;;
    --no-lock)
      no_lock=1
      shift
      ;;
    --disable-sccache)
      disable_sccache=1
      shift
      ;;
    --skip-cache-gc)
      skip_cache_gc=1
      shift
      ;;
    --lock-timeout-seconds)
      [[ $# -ge 2 ]] || { usage; exit 2; }
      lock_timeout_seconds="$2"
      shift 2
      ;;
    --)
      shift
      break
      ;;
    *)
      break
      ;;
  esac
done

[[ $# -gt 0 ]] || { usage; exit 2; }

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(git -C "$script_dir" rev-parse --show-toplevel 2>/dev/null || true)"
[[ -n "$repo_root" ]] || { echo "cargo-dev.sh is not inside a Git repository." >&2; exit 1; }

if command -v powershell.exe >/dev/null 2>&1 && command -v cygpath >/dev/null 2>&1; then
  ps_script="$(cygpath -w "$script_dir/cargo-dev.ps1")"
  ps_args=(-NoProfile -ExecutionPolicy Bypass -File "$ps_script" -Domain "$domain" -LockTimeoutSeconds "$lock_timeout_seconds")
  if [[ -n "$target_dir" ]]; then
    ps_args+=(-TargetDir "$(cygpath -w "$target_dir")")
  fi
  [[ "$no_lock" -eq 0 ]] || ps_args+=(-NoLock)
  [[ "$disable_sccache" -eq 0 ]] || ps_args+=(-DisableSccache)
  [[ "$skip_cache_gc" -eq 0 ]] || ps_args+=(-SkipCacheGc)
  powershell.exe "${ps_args[@]}" "$@"
  exit $?
fi

# Non-Windows fallback: do not create another machine-wide target pool. Cargo's
# normal workspace target remains the final-artifact owner, while sccache may
# still share cacheable compiler objects on that host.
if [[ -n "$target_dir" ]]; then
  case "$target_dir" in
    /*) export CARGO_TARGET_DIR="$target_dir" ;;
    *) echo "--target-dir must be absolute: $target_dir" >&2; exit 1 ;;
  esac
fi
if [[ "$disable_sccache" -eq 0 ]] && command -v sccache >/dev/null 2>&1; then
  export RUSTC_WRAPPER="$(command -v sccache)"
  export SCCACHE_BASEDIRS="$repo_root"
fi
for arg in "$@"; do
  if [[ "$arg" == "--release" || "$arg" == "--profile=release" ]]; then
    export CARGO_INCREMENTAL=0
    break
  fi
done

echo "cargo $*"
cargo "$@"
