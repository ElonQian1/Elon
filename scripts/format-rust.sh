#!/usr/bin/env bash
set -euo pipefail

apply=false
all=false
files=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --apply)
      apply=true
      shift
      ;;
    --all)
      all=true
      shift
      ;;
    --files)
      shift
      files=("$@")
      break
      ;;
    *)
      echo "Usage: scripts/format-rust.sh [--apply --all] [--apply --files <file>...]" >&2
      exit 2
      ;;
  esac
done

if [[ "$all" == true && ${#files[@]} -gt 0 ]]; then
  echo "Choose either --all or --files; they cannot be combined." >&2
  exit 2
fi

if [[ "$apply" == true && "$all" != true && ${#files[@]} -eq 0 ]]; then
  echo "Refusing an implicit repository-wide write. Use --apply --files <changed.rs...> for daily work, or --apply --all in a dedicated format-only task." >&2
  exit 2
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
cd "$repo_root"

if [[ "$apply" == true && "$all" == true ]]; then
  worktree_status="$(git status --porcelain=v1 --untracked-files=all)"
  if [[ -n "$worktree_status" ]]; then
    echo "Refusing a repository-wide write in a dirty worktree. Commit or isolate existing changes, then run --apply --all from a clean dedicated task." >&2
    exit 2
  fi
fi

if [[ ! -f ".rustfmt-version" ]]; then
  echo "Rust formatter version lock is missing: .rustfmt-version" >&2
  exit 1
fi
expected_rustfmt_version="$(tr -d '\r\n' < .rustfmt-version)"
actual_rustfmt_version="$(rustfmt --version)"
if [[ "$actual_rustfmt_version" != "$expected_rustfmt_version" ]]; then
  echo "rustfmt version mismatch. Expected '$expected_rustfmt_version', got '$actual_rustfmt_version'. Use the baseline toolchain or create a dedicated format-baseline migration." >&2
  exit 1
fi

manifests=(
  "server/Cargo.toml"
  "server/pc-dev-runtime/Cargo.toml"
  "server/homecli-proto/Cargo.toml"
)

roots=(
  "server"
  "server/pc-dev-runtime"
  "server/homecli-proto"
)

manifest_edition() {
  local manifest="$1"
  local edition
  edition="$(sed -nE 's/^edition[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$manifest" | head -n 1)"
  if [[ -z "$edition" ]]; then
    echo "Rust manifest is missing an explicit edition: $manifest" >&2
    exit 1
  fi
  printf '%s\n' "$edition"
}

repo_relative_path() {
  local file="${1//\\//}"
  local repo="${repo_root//\\//}"
  if [[ "$file" == "$repo/"* ]]; then
    file="${file#"$repo/"}"
  elif [[ "$file" == /* ]]; then
    echo "Rust file is outside repository: $1" >&2
    exit 1
  fi
  printf '%s\n' "$file"
}

for manifest in "${manifests[@]}"; do
  if [[ ! -f "$manifest" ]]; then
    echo "Rust manifest not found: $manifest" >&2
    exit 1
  fi

  manifest_edition "$manifest" >/dev/null
done

if [[ ${#files[@]} -gt 0 ]]; then
  declare -A grouped_files=()
  editions=()

  for file in "${files[@]}"; do
    relative="$(repo_relative_path "$file")"
    [[ "$relative" == *.rs ]] || continue
    if [[ ! -f "$relative" ]]; then
      echo "Rust file not found: $relative" >&2
      exit 1
    fi

    manifest=""
    for index in "${!roots[@]}"; do
      root="${roots[$index]}"
      if [[ "$relative" == "$root/"* ]]; then
        manifest="${manifests[$index]}"
      fi
    done
    if [[ -z "$manifest" ]]; then
      echo "Rust file is not under a known crate: $relative" >&2
      exit 1
    fi

    edition="$(manifest_edition "$manifest")"
    if [[ -z "${grouped_files[$edition]+set}" ]]; then
      grouped_files["$edition"]=""
      editions+=("$edition")
    fi
    grouped_files["$edition"]+="$relative"$'\n'
  done

  if [[ ${#editions[@]} -eq 0 ]]; then
    echo "No Rust files to format"
    exit 0
  fi

  for edition in "${editions[@]}"; do
    mapfile -t edition_files <<<"${grouped_files[$edition]}"
    nonempty_files=()
    for edition_file in "${edition_files[@]}"; do
      [[ -n "$edition_file" ]] && nonempty_files+=("$edition_file")
    done
    edition_files=("${nonempty_files[@]}")
    args=(--edition "$edition" --config skip_children=true)
    if [[ "$apply" != true ]]; then
      args+=(--check)
    fi
    args+=("${edition_files[@]}")
    if [[ "$apply" == true ]]; then
      echo "Formatting ${#edition_files[@]} Rust file(s) with edition $edition"
    else
      echo "Checking ${#edition_files[@]} Rust file(s) with edition $edition"
    fi
    rustfmt "${args[@]}"
  done
  exit 0
fi

if [[ "$apply" != true ]]; then
  for manifest in "${manifests[@]}"; do
    echo "Checking $manifest"
    cargo fmt --manifest-path "$manifest" --all -- --check
  done
  exit 0
fi

full_format_clean() {
  for manifest in "${manifests[@]}"; do
    cargo fmt --manifest-path "$manifest" --all -- --check >/dev/null 2>&1 || return 1
  done
}

converged=false
for pass in 1 2 3; do
  for manifest in "${manifests[@]}"; do
    echo "Formatting $manifest (pass $pass/3)"
    cargo fmt --manifest-path "$manifest" --all
  done
  if full_format_clean; then
    echo "Full Rust format converged after $pass pass(es)"
    converged=true
    break
  fi
done

if [[ "$converged" != true ]]; then
  echo "Full Rust format did not converge after 3 passes. Running a visible check for diagnostics." >&2
  for manifest in "${manifests[@]}"; do
    cargo fmt --manifest-path "$manifest" --all -- --check
  done
  exit 1
fi
