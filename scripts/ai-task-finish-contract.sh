#!/usr/bin/env bash

ai_finish_contract_root() {
  local state_root
  if [[ -n "${XDG_STATE_HOME:-}" ]]; then
    state_root="$XDG_STATE_HOME"
  elif [[ -n "${HOME:-}" ]]; then
    state_root="$HOME/.local/state"
  else
    state_root="${TMPDIR:-/tmp}/elon-node-${UID:-unknown}"
  fi
  printf '%s/ElonNode/ai-finish-contracts-v1' "$state_root"
}

ai_finish_sha256_file() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 "$path" | awk '{print $NF}'
  else
    echo 'A SHA-256 utility is required for task finish contracts.' >&2
    return 1
  fi
}

ai_finish_reject_control_text() {
  local label="$1" value="$2"
  if [[ "$value" == *$'\n'* || "$value" == *$'\r'* || "$value" == *$'\t'* ]]; then
    echo "Task finish contract $label contains unsupported control characters." >&2
    return 1
  fi
}

ai_finish_worktree_lease_reason() {
  local repo_path="$1" target
  target="$(cd "$repo_path" && pwd -P)"
  git -C "$repo_path" worktree list --porcelain | awk -v target="$target" '
    /^worktree / { path=substr($0,10); current=(path == target); next }
    current && /^locked/ { sub(/^locked[[:space:]]*/, ""); print; exit }
  '
}

ai_finish_load_platform_identity() {
  local repo_path="$1" branch="$2" allow_released="${3:-0}" expected_root="${4:-}"
  local worktree project conversation marker lease expected_lease
  AI_FINISH_PLATFORM_ROOT=""
  AI_FINISH_PLATFORM_LEASE=""
  AI_FINISH_PLATFORM_COMMON_DIR=""
  [[ "$branch" == ai/session/* ]] || return 0
  IFS='/' read -r -a branch_parts <<< "$branch"
  [[ "${#branch_parts[@]}" -eq 4 && "${branch_parts[0]}" == ai && "${branch_parts[1]}" == session ]] || {
    echo 'Platform session branch shape is not trusted.' >&2; return 1;
  }
  worktree="$(cd "$repo_path" && pwd -P)"
  conversation="$(basename "$worktree")"
  project="$(basename "$(dirname "$worktree")")"
  marker="$(basename "$(dirname "$(dirname "$worktree")")")"
  [[ "$marker" == conversation-worktrees && "$project" == "${branch_parts[2]}" && "$conversation" == "${branch_parts[3]}" ]] || {
    echo 'Platform session path, project, conversation, and branch identity differ.' >&2; return 1;
  }
  lease="$(ai_finish_worktree_lease_reason "$repo_path")"
  if [[ -z "$expected_root" ]]; then
    [[ "$lease" =~ ^elon-supervision:([A-Za-z0-9._-]{1,160})$ ]] || {
      echo 'Platform session preflight requires an exact elon-supervision:<root> lease.' >&2; return 1;
    }
    expected_root="${BASH_REMATCH[1]}"
  else
    expected_lease="elon-supervision:$expected_root"
    [[ -z "$lease" || "$lease" == "$expected_lease" ]] || {
      echo "Platform session lease identity mismatch: $lease" >&2; return 1;
    }
    [[ -n "$lease" || "$allow_released" -eq 1 ]] || {
      echo 'Platform session root lease disappeared before validation.' >&2; return 1;
    }
  fi
  AI_FINISH_PLATFORM_ROOT="$expected_root"
  AI_FINISH_PLATFORM_LEASE="elon-supervision:$expected_root"
  AI_FINISH_PLATFORM_COMMON_DIR="$(git -C "$repo_path" rev-parse --path-format=absolute --git-common-dir)"
}

new_ai_task_finish_contract() {
  local repo_path="$1" worktree branch base_commit origin issued nonce root temp contract_id path
  worktree="$(cd "$repo_path" && pwd -P)"
  branch="$(git -C "$worktree" branch --show-current)"
  base_commit="$(git -C "$worktree" rev-parse 'HEAD^{commit}')"
  origin="$(git -C "$worktree" remote get-url origin)"
  ai_finish_load_platform_identity "$worktree" "$branch"
  ai_finish_reject_control_text worktree "$worktree"
  ai_finish_reject_control_text branch "$branch"
  ai_finish_reject_control_text origin "$origin"
  issued="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  nonce="$(printf '%s' "$worktree|$branch|$base_commit|$issued|$$|${RANDOM:-0}|${RANDOM:-0}" | git -C "$worktree" hash-object --stdin)"
  root="$(ai_finish_contract_root)"
  mkdir -p "$root"
  temp="$root/.contract-$nonce.tmp"
  printf 'schema\telon.ai_finish_contract.v1\nworktree\t%s\nbranch\t%s\nbaseCommit\t%s\norigin\t%s\nissuedAtUtc\t%s\nnonce\t%s\n' \
    "$worktree" "$branch" "$base_commit" "$origin" "$issued" "$nonce" > "$temp"
  if [[ "$branch" == ai/session/* ]]; then
    printf 'platformProvenance\telon.conversation_worktree.v1\nsupervisionRootTaskId\t%s\nleaseReason\t%s\ngitCommonDir\t%s\n' \
      "$AI_FINISH_PLATFORM_ROOT" "$AI_FINISH_PLATFORM_LEASE" "$AI_FINISH_PLATFORM_COMMON_DIR" >> "$temp"
  fi
  contract_id="$(ai_finish_sha256_file "$temp")"
  path="$root/$contract_id.contract"
  if ! command ln "$temp" "$path" 2>/dev/null; then
    rm -f -- "$temp"
    echo "Task finish contract already exists unexpectedly: $contract_id" >&2
    return 1
  fi
  rm -f -- "$temp"
  printf '%s' "$contract_id"
}

ai_finish_contract_value() {
  local path="$1" key="$2"
  awk -F '\t' -v key="$key" '$1 == key { sub(/^[^\t]*\t/, ""); print; found=1; exit } END { if (!found) exit 1 }' "$path"
}

assert_ai_task_finish_contract() {
  local repo_path="$1" contract_id="$2" root path actual worktree branch origin schema expected_worktree expected_branch expected_origin base_commit provenance expected_root expected_lease expected_common
  [[ "$contract_id" =~ ^[0-9a-f]{64}$ ]] || { echo 'TaskContract must be a SHA-256 id.' >&2; return 1; }
  root="$(ai_finish_contract_root)"
  path="$root/$contract_id.contract"
  [[ -f "$path" ]] || { echo "Task finish contract not found: $contract_id" >&2; return 1; }
  actual="$(ai_finish_sha256_file "$path")"
  [[ "$actual" == "$contract_id" ]] || { echo 'Task finish contract digest mismatch.' >&2; return 1; }
  schema="$(ai_finish_contract_value "$path" schema)"
  [[ "$schema" == 'elon.ai_finish_contract.v1' ]] || { echo 'Unsupported task finish contract schema.' >&2; return 1; }
  worktree="$(cd "$repo_path" && pwd -P)"
  branch="$(git -C "$worktree" branch --show-current)"
  origin="$(git -C "$worktree" remote get-url origin)"
  expected_worktree="$(ai_finish_contract_value "$path" worktree)"
  expected_branch="$(ai_finish_contract_value "$path" branch)"
  expected_origin="$(ai_finish_contract_value "$path" origin)"
  base_commit="$(ai_finish_contract_value "$path" baseCommit)"
  [[ "$worktree" == "$expected_worktree" ]] || { echo 'Task finish contract worktree identity mismatch.' >&2; return 1; }
  [[ "$branch" == "$expected_branch" && "$origin" == "$expected_origin" ]] || { echo 'Task finish contract branch or repository identity mismatch.' >&2; return 1; }
  if [[ "$branch" == ai/session/* ]]; then
    provenance="$(ai_finish_contract_value "$path" platformProvenance)"
    expected_root="$(ai_finish_contract_value "$path" supervisionRootTaskId)"
    expected_lease="$(ai_finish_contract_value "$path" leaseReason)"
    expected_common="$(ai_finish_contract_value "$path" gitCommonDir)"
    [[ "$provenance" == elon.conversation_worktree.v1 && -n "$expected_root" && "$expected_lease" == "elon-supervision:$expected_root" ]] || {
      echo 'Platform task finish contract lacks immutable provenance/root/lease identity.' >&2; return 1;
    }
    ai_finish_load_platform_identity "$worktree" "$branch" 0 "$expected_root"
    [[ "$AI_FINISH_PLATFORM_COMMON_DIR" == "$expected_common" ]] || {
      echo 'Platform task finish contract Git common-dir identity mismatch.' >&2; return 1;
    }
    AI_FINISH_CONTRACT_ROOT="$expected_root"
  fi
  git -C "$worktree" merge-base --is-ancestor "$base_commit" HEAD || { echo 'Task HEAD is not descended from the preflight contract base.' >&2; return 1; }
}
