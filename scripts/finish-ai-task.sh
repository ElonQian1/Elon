#!/usr/bin/env bash
set -euo pipefail

kind="CodePushed"
task_worktree=""
skip_artifact_cleanup=0
skip_worktree_cleanup=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --kind) kind="${2:?missing value for --kind}"; shift 2 ;;
    --task-worktree) task_worktree="${2:?missing value for --task-worktree}"; shift 2 ;;
    --skip-artifact-cleanup) skip_artifact_cleanup=1; shift ;;
    --skip-worktree-cleanup) skip_worktree_cleanup=1; shift ;;
    -h|--help)
      echo "Usage: bash scripts/finish-ai-task.sh [--kind CodePushed] [--task-worktree PATH]"
      exit 0
      ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

business_status="not_checked"
local_main_status="not_checked"
task_worktree_status="not_checked"

finish_error() {
  echo "BUSINESS_STATUS=$business_status"
  echo "LOCAL_MAIN_STATUS=$local_main_status"
  echo "TASK_WORKTREE_STATUS=$task_worktree_status"
  echo "FINALIZABLE=false"
  echo "FINISH_ERROR=$1" >&2
  exit 1
}

start_path="${task_worktree:-$PWD}"
[[ -e "$start_path" ]] || finish_error "Task worktree does not exist: $start_path"
task_root="$(git -C "$start_path" rev-parse --show-toplevel)"
policy_path="$task_root/.ai/workspace-policy.txt"
[[ -f "$policy_path" ]] || finish_error "Workspace policy is missing: $policy_path"

temporary_roots=()
source_extensions=()
generated_extensions=()
while read -r rule value rest; do
  [[ -z "${rule:-}" || "$rule" == \#* ]] && continue
  [[ -n "${value:-}" && -z "${rest:-}" ]] || finish_error "Invalid workspace policy line: $rule $value $rest"
  case "$rule" in
    temporary-root) temporary_roots+=("$value") ;;
    source-extension) source_extensions+=("$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')") ;;
    generated-extension) generated_extensions+=("$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')") ;;
    *) finish_error "Unknown workspace policy rule: $rule" ;;
  esac
done < "$policy_path"

contains_value() {
  local needle="$1"; shift
  local item
  for item in "$@"; do [[ "$item" == "$needle" ]] && return 0; done
  return 1
}

artifact_disposition() {
  local path="$1" base extension
  base="${path##*/}"
  if [[ "$base" == *.* ]]; then extension=".${base##*.}"; else extension=""; fi
  extension="$(printf '%s' "$extension" | tr '[:upper:]' '[:lower:]')"
  if contains_value "$extension" "${source_extensions[@]}"; then
    echo "candidate_track"
  elif contains_value "$extension" "${generated_extensions[@]}"; then
    echo "candidate_temporary_or_precise_ignore"
  else
    echo "owner_decision_required"
  fi
}

clear_temporary_roots() {
  local repo="$1" label="$2" relative candidate tracked
  [[ "$skip_artifact_cleanup" -eq 1 ]] && return 0
  for relative in "${temporary_roots[@]}"; do
    relative="${relative%/}"
    [[ -n "$relative" && "$relative" != /* && "$relative" != *"../"* && "$relative" != ".." ]] || \
      finish_error "Unsafe temporary-root policy value: $relative"
    candidate="$repo/$relative"
    case "$candidate" in "$repo"/*) ;; *) finish_error "Temporary root escaped repository boundary: $candidate" ;; esac
    tracked="$(git -C "$repo" ls-files -- "$relative")"
    [[ -z "$tracked" ]] || finish_error "Refusing to clean tracked files from declared temporary root '$relative'."
    if [[ -e "$candidate" ]]; then
      rm -rf -- "$candidate"
      echo "ARTIFACT_CLEANUP=$label:$relative"
    fi
  done
}

audit_untracked() {
  local repo="$1" prefix="$2" line path disposition count=0
  while IFS= read -r line; do
    [[ "$line" == "?? "* ]] || continue
    path="${line:3}"
    disposition="$(artifact_disposition "$path")"
    echo "${prefix}_PATH=$path|$disposition"
    count=$((count + 1))
  done < <(git -C "$repo" -c core.quotePath=false status --porcelain=v1 --untracked-files=all)
  if [[ "$count" -eq 0 ]]; then echo "$prefix=clean"; else echo "$prefix=warning:$count"; fi
}

clear_temporary_roots "$task_root" "task"
task_status="$(git -C "$task_root" -c core.quotePath=false status --porcelain=v1 --untracked-files=all)"
if [[ -n "$task_status" ]]; then
  task_worktree_status="dirty"
  while IFS= read -r line; do
    if [[ "$line" == "?? "* ]]; then
      path="${line:3}"
      echo "TASK_UNRESOLVED_PATH=$path|$(artifact_disposition "$path")"
    else
      echo "TASK_UNRESOLVED_GIT=$line"
    fi
  done <<< "$task_status"
  finish_error "Task worktree is not clean. Track intentional source/tests, move disposable output under .ai-tmp/, or add a precise ignore rule for stable generated output."
fi
task_worktree_status="clean"

case "$kind" in
  CodePushed|CodeSync|DocsOnly)
    git -C "$task_root" fetch origin main
    task_head="$(git -C "$task_root" rev-parse HEAD)"
    origin_head="$(git -C "$task_root" rev-parse origin/main)"
    git -C "$task_root" merge-base --is-ancestor "$task_head" "$origin_head" || \
      finish_error "Code push is incomplete: task HEAD is not contained in origin/main."
    ;;
  AndroidFeature|NodeAgent|Server|PcFrontend)
    if command -v pwsh >/dev/null 2>&1; then
      pwsh -NoProfile -File "$task_root/scripts/check-task-complete.ps1" -Kind "$kind" || \
        finish_error "Completion check failed for kind $kind."
    else
      finish_error "Kind $kind requires pwsh for the release provenance check; install PowerShell 7 or run the check on a configured release machine."
    fi
    ;;
  *) finish_error "Unsupported completion kind: $kind" ;;
esac
business_status="complete"

main_path="$(git -C "$task_root" worktree list --porcelain | awk '
  /^worktree / { path=substr($0,10) }
  /^branch refs\/heads\/main$/ { print path; exit }
')"
[[ -n "$main_path" ]] || finish_error "No checked-out main worktree was found."

clear_temporary_roots "$main_path" "main"
main_tracked="$(git -C "$main_path" status --porcelain=v1 --untracked-files=no)"
if [[ -n "$main_tracked" ]]; then
  local_main_status="blocked_tracked_changes"
  printf 'MAIN_TRACKED_CHANGE=%s\n' "$main_tracked"
  finish_error "The main baseline has tracked changes and cannot be fast-forwarded safely."
fi

git -C "$main_path" fetch origin main || finish_error "Unable to fetch origin/main while finalizing."
if ! merge_output="$(git -C "$main_path" merge --ff-only origin/main 2>&1)"; then
  local_main_status="sync_failed"
  finish_error "The main baseline could not fast-forward. Git may be protecting an untracked same-path collision: $merge_output"
fi
main_head="$(git -C "$main_path" rev-parse HEAD)"
origin_head="$(git -C "$main_path" rev-parse origin/main)"
[[ "$main_head" == "$origin_head" ]] || finish_error "Local main is not the current fetched origin/main."
local_main_status="current:${main_head:0:7}"
audit_untracked "$main_path" "MAIN_UNTRACKED_STATUS"

task_branch="$(git -C "$task_root" branch --show-current)"
task_leaf="$(basename "$task_root")"
if [[ "$(cd "$task_root" && pwd -P)" == "$(cd "$main_path" && pwd -P)" ]]; then
  task_worktree_status="main_baseline_not_applicable"
elif [[ "$task_branch" == ai/session/* ]]; then
  task_worktree_status="platform_managed"
elif [[ "$skip_worktree_cleanup" -eq 1 ]]; then
  task_worktree_status="skipped_by_option"
elif [[ "$task_branch" == codex/* || "$task_leaf" =~ -task-[0-9]{8}-[0-9]{6} ]]; then
  cd "$main_path"
  bash "$main_path/scripts/cleanup-task-worktrees.sh" --apply || {
    task_worktree_status="cleanup_failed"
    finish_error "Task worktree cleanup command failed."
  }
  if git -C "$main_path" worktree list --porcelain | grep -Fq "worktree $task_root"; then
    task_worktree_status="cleanup_failed"
    finish_error "Task worktree is still registered after cleanup: $task_root"
  fi
  task_worktree_status="cleaned"
else
  task_worktree_status="user_managed"
fi

echo "BUSINESS_STATUS=$business_status"
echo "LOCAL_MAIN_STATUS=$local_main_status"
echo "TASK_WORKTREE_STATUS=$task_worktree_status"
echo "FINALIZABLE=true"
