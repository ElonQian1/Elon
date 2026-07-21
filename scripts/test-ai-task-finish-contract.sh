#!/usr/bin/env bash
set -euo pipefail
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/ai-task-finish-contract.sh"

test_root="$(mktemp -d)"
trap 'rm -rf -- "$test_root"' EXIT
origin="$test_root/origin.git"
repo_root="$test_root/repo"
git init --bare --initial-branch=main "$origin" >/dev/null
git init -b main "$repo_root" >/dev/null
git -C "$repo_root" config user.email finish-contract-test@example.invalid
git -C "$repo_root" config user.name finish-contract-test
git -C "$repo_root" remote add origin "$origin"
printf 'finish contract fixture\n' > "$repo_root/README.md"
git -C "$repo_root" add README.md
git -C "$repo_root" commit -m 'seed finish contract fixture' >/dev/null

contract_id="$(new_ai_task_finish_contract "$repo_root")"
assert_ai_task_finish_contract "$repo_root" "$contract_id"
printf 'BASH_FINISH_CONTRACT=validated:%s\n' "$contract_id"
