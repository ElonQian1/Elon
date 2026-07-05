#!/usr/bin/env bash
set -euo pipefail

apply=false
files=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --apply)
      apply=true
      shift
      ;;
    --files)
      shift
      files=("$@")
      break
      ;;
    *)
      echo "Usage: scripts/format-rust.sh [--apply] [--files <file>...]" >&2
      exit 2
      ;;
  esac
done

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(git -C "$script_dir" rev-parse --show-toplevel)"
cd "$repo_root"

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

for manifest in "${manifests[@]}"; do
  if [[ "$apply" == true ]]; then
    echo "Formatting $manifest"
    cargo fmt --manifest-path "$manifest" --all
  else
    echo "Checking $manifest"
    cargo fmt --manifest-path "$manifest" --all -- --check
  fi
done
