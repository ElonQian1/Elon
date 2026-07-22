#!/usr/bin/env bash
# ================================================================
#  elon Android APK 发布脚本（Linux/macOS 版）
#  等效于 publish-apk.ps1，支持 Ubuntu/Linux 开发机和 Codex CLI 服务器环境
#
#  依赖：
#    - JDK（构建 Android APK 必须）
#    - Android SDK（ANDROID_HOME 环境变量）
#    - curl, python3, ssh, scp（系统自带）
#    - ~/.elon/signing/elon-release.jks（签名密钥）
#    - ~/.gradle/gradle.properties（签名密码配置）
#
#  签名配置（一次性，存入 ~/.gradle/gradle.properties，不进 git）：
#    ELON_RELEASE_KEYSTORE=/root/.elon/signing/elon-release.jks
#    ELON_RELEASE_STORE_PASSWORD=<密码>
#    ELON_RELEASE_KEY_ALIAS=elon
#    ELON_RELEASE_KEY_PASSWORD=<密码>
#
#  用法：
#    bash publish-apk.sh --changelog="描述"
#    bash publish-apk.sh --changelog="描述" --skip-build
#    bash publish-apk.sh --changelog="描述" --force
# ================================================================
set -euo pipefail

# ── ANSI 颜色 ──────────────────────────────────────────────────
RED='\033[0;31m'; YELLOW='\033[1;33m'; GREEN='\033[0;32m'
CYAN='\033[0;36m'; GRAY='\033[0;37m'; NC='\033[0m'

# ── 参数解析 ──────────────────────────────────────────────────
SKIP_BUILD=0; FORCE=0; CHANGELOG=""
for arg in "$@"; do
  case "$arg" in
    --skip-build)   SKIP_BUILD=1 ;;
    --force)        FORCE=1 ;;
    --changelog=*)  CHANGELOG="${arg#*=}" ;;
    *)
      if [[ -z "$CHANGELOG" && "$arg" != --* ]]; then
        CHANGELOG="$arg"
      else
        echo -e "${RED}未知参数: $arg${NC}" >&2; exit 1
      fi
      ;;
  esac
done

if [[ -z "$CHANGELOG" ]]; then
  echo -e "${RED}❌ 请提供 changelog：${NC}" >&2
  echo -e "${RED}   bash publish-apk.sh --changelog='描述'${NC}" >&2
  exit 1
fi

# ── 配置 ───────────────────────────────────────────────────────
RELEASE_API_BASE="http://43.139.149.158:8080/api/release"
SERVER_HOST="root@43.139.149.158"
SERVER_DIR="/opt/elon/data/app"
SERVER_URL="http://43.139.149.158:8080"
APK_SHA_FILE="$SERVER_DIR/.apk-deployed-sha"
SSH_OPTS="-o ProxyCommand=none -o ConnectTimeout=15 -o ServerAliveInterval=10 -o ServerAliveCountMax=3 -o BatchMode=yes"

is_local_apk_deploy() {
  case "${ELON_DEPLOY_LOCAL:-auto}" in
    1|true|TRUE|local|LOCAL) return 0 ;;
    0|false|FALSE|remote|REMOTE) return 1 ;;
  esac
  [[ -d "$SERVER_DIR" && -w "$SERVER_DIR" ]]
}

# ── 路径推导 ──────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
. "$SCRIPT_DIR/release-publish-lease.sh"
ANDROID_DIR="$REPO_ROOT/android"
GRADLE_PATH="$ANDROID_DIR/app/build.gradle"
APK_DIR="$ANDROID_DIR/app/build/outputs/apk/release"

DEFAULT_KEYSTORE="$HOME/.elon/signing/elon-release.jks"
USER_GRADLE_PROPS="$HOME/.gradle/gradle.properties"

# ── 状态变量 ──────────────────────────────────────────────────
RELEASE_TOKEN=""
RELEASE_FINISHED=0
ORIGINAL_GRADLE_CONTENT=""
BUILD_BASE_SHA=""

# ── JSON 辅助 ─────────────────────────────────────────────────
json_get() {
  # json_get <json_string> <key>
  echo "$1" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('$2',''))" 2>/dev/null || echo ""
}

json_get_int() {
  echo "$1" | python3 -c "import sys,json; d=json.load(sys.stdin); print(int(d.get('$2',0)))" 2>/dev/null || echo "0"
}

http_get_json() {
  local url="$1"
  curl -sS --fail --noproxy '*' --max-time 10 "$url"
}

git_fetch_hint() {
  local output="${1:-}"
  if [[ "$output" =~ (Could\ not\ resolve\ host|Name\ or\ service\ not\ known|Temporary\ failure\ in\ name\ resolution) ]]; then
    printf '%s\n' "网络/DNS 无法解析 GitHub，请检查网络、DNS 或代理后重试。"
  elif [[ "$output" =~ (Failed\ to\ connect|Connection\ timed\ out|Connection\ reset|Connection\ refused|Operation\ timed\ out|HTTP/2\ stream|early\ EOF|The\ remote\ end\ hung\ up\ unexpectedly) ]]; then
    printf '%s\n' "网络连接到 GitHub 不稳定或超时，通常是临时抖动；脚本已短重试但仍失败。"
  elif [[ "$output" =~ (Permission\ denied|Authentication\ failed|Repository\ not\ found|Could\ not\ read\ from\ remote\ repository|Host\ key\ verification\ failed|publickey) ]]; then
    printf '%s\n' "Git 远端认证或仓库权限异常，请检查 SSH key、GitHub 权限和 origin 地址。"
  else
    printf '%s\n' "Git fetch 失败，原因未能自动分类；请查看原始输出。"
  fi
}

git_fetch_with_retry() {
  local attempts=3 delay=2 quiet="${1:-0}" i output hint
  for ((i=1; i<=attempts; i++)); do
    if output=$(git -C "$REPO_ROOT" fetch origin main 2>&1); then
      if [[ "$quiet" != "1" && "$i" -gt 1 ]]; then
        echo -e "${GREEN}   ✅ git fetch 重试成功（第 $i 次）${NC}"
      fi
      return 0
    fi
    if [[ "$quiet" != "1" ]]; then
      hint="$(git_fetch_hint "$output")"
      echo -e "${YELLOW}   ⚠️  git fetch 失败（第 $i/$attempts 次）：$hint${NC}" >&2
    fi
    if [[ "$i" -lt "$attempts" ]]; then
      sleep "$delay"
    fi
  done
  hint="$(git_fetch_hint "$output")"
  if [[ "$quiet" != "1" ]]; then
    echo "CODE_SYNC_STATUS=unknown_fetch_failed"
    echo "APK_RELEASE_STATUS=not_attempted"
    echo "SERVER_RELEASE_STATUS=not_attempted"
    echo -e "${RED}❌ APK 发布未开始：git fetch origin main 连续失败 $attempts 次。$hint${NC}" >&2
    echo -e "${YELLOW}   原始输出：$output${NC}" >&2
  fi
  return 1
}

print_publish_status() {
  local apk_status="$1"
  local code_status="${2:-synced}"
  local server_status="${3:-not_attempted}"
  local message="${4:-}"
  [[ -n "$message" ]] && echo -e "${CYAN}   $message${NC}"
  echo -e "${GRAY}   CODE_SYNC_STATUS=$code_status${NC}"
  echo -e "${GRAY}   APK_RELEASE_STATUS=$apk_status${NC}"
  echo -e "${GRAY}   SERVER_RELEASE_STATUS=$server_status${NC}"
}

# ── Release API ───────────────────────────────────────────────
call_release_api() {
  local endpoint="$1"
  local body="${2:-}"
  local url="$RELEASE_API_BASE/$endpoint"
  local raw status body_text

  if [[ -n "$body" ]]; then
    raw=$(curl -s --noproxy '*' --max-time 20 \
      -X POST \
      -H 'Content-Type: application/json; charset=utf-8' \
      -d "$body" \
      -w '\n__HTTP_STATUS__:%{http_code}' \
      "$url" 2>&1) || { echo "curl 调用失败 ($endpoint)"; return 1; }
  else
    raw=$(curl -s --noproxy '*' --max-time 20 \
      -X POST \
      -w '\n__HTTP_STATUS__:%{http_code}' \
      "$url" 2>&1) || { echo "curl 调用失败 ($endpoint)"; return 1; }
  fi

  status=$(echo "$raw" | grep -oP '__HTTP_STATUS__:\K\d+' || echo "0")
  body_text=$(echo "$raw" | sed 's/\n\?__HTTP_STATUS__:[0-9]*$//' | head -c -1)

  if [[ "$status" -lt 200 || "$status" -ge 300 ]]; then
    echo "release/$endpoint HTTP $status: $body_text" >&2
    return 1
  fi
  echo "$body_text"
}

complete_release() {
  local success="$1"
  local version_name="${2:-}"
  local version_code="${3:-0}"
  local sha="${4:-}"
  local error_msg="${5:-}"

  [[ -z "$RELEASE_TOKEN" || "$RELEASE_FINISHED" == "1" ]] && return 0

  local payload
  if [[ "$success" == "true" ]]; then
    payload=$(python3 -c "import json; print(json.dumps({'kind':'apk','token':'$RELEASE_TOKEN','success':True,'versionName':'$version_name','versionCode':$version_code,'sha':'$sha'}))")
  else
    payload=$(python3 -c "import json; print(json.dumps({'kind':'apk','token':'$RELEASE_TOKEN','success':False,'errorMessage':'$error_msg'}))")
  fi

  call_release_api "finish" "$payload" > /dev/null 2>&1 || true
  RELEASE_FINISHED=1
}

resolve_apk_version_baseline() {
  local best_code=0 best_name="" best_source="" found=0
  local status_json status_code status_name deployed_json deployed_code deployed_name

  if status_json=$(http_get_json "$RELEASE_API_BASE/status?kind=apk" 2>/dev/null); then
    status_code=$(json_get_int "$status_json" "lastPublishedVersionCode")
    status_name=$(json_get "$status_json" "lastPublishedVersionName")
    if [[ "$status_code" -gt 0 && -n "$status_name" ]]; then
      best_code="$status_code"
      best_name="$status_name"
      best_source="/api/release/status"
      found=1
    fi
  else
    echo -e "${YELLOW}   ⚠️  APK 版本基线读取失败：/api/release/status?kind=apk${NC}" >&2
  fi

  if deployed_json=$(http_get_json "$SERVER_URL/app/version.json" 2>/dev/null); then
    deployed_code=$(json_get_int "$deployed_json" "versionCode")
    deployed_name=$(json_get "$deployed_json" "versionName")
    if [[ "$deployed_code" -gt 0 && -n "$deployed_name" ]]; then
      if [[ "$found" -eq 1 && ( "$deployed_code" -ne "$best_code" || "$deployed_name" != "$best_name" ) ]]; then
        echo -e "${YELLOW}   ⚠️  服务器 APK 版本来源不一致：/app/version.json=v${deployed_name} build ${deployed_code}，release/status=v${best_name} build ${best_code}，采用最高 build${NC}" >&2
      fi
      if [[ "$deployed_code" -gt "$best_code" ]]; then
        best_code="$deployed_code"
        best_name="$deployed_name"
        best_source="/app/version.json"
      fi
      found=1
    fi
  else
    echo -e "${YELLOW}   ⚠️  APK 版本基线读取失败：/app/version.json${NC}" >&2
  fi

  if [[ "$found" -eq 0 ]]; then
    echo -e "${RED}❌ 无法读取服务器 APK 版本基线；发布已停止，避免用 build.gradle 兜底版本发布。${NC}" >&2
    return 1
  fi

  printf '%s|%s|%s\n' "$best_code" "$best_name" "$best_source"
}

find_aapt() {
  local sdk_root candidate
  for sdk_root in "${ANDROID_HOME:-}" "${ANDROID_SDK_ROOT:-}" "$HOME/Android/Sdk"; do
    [[ -z "$sdk_root" || ! -d "$sdk_root/build-tools" ]] && continue
    candidate=$(find "$sdk_root/build-tools" -type f -name aapt 2>/dev/null | sort -V | tail -1)
    if [[ -n "$candidate" && -x "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  if command -v aapt >/dev/null 2>&1; then
    command -v aapt
    return 0
  fi

  return 1
}

apk_manifest_version() {
  local apk_path="$1"
  local aapt_bin badging package_line actual_code actual_name

  [[ -f "$apk_path" ]] || {
    echo "APK 文件不存在，无法校验版本: $apk_path" >&2
    return 1
  }

  aapt_bin=$(find_aapt) || {
    echo "未找到 aapt，无法校验 APK manifest 版本。请确认 Android SDK build-tools 已安装。" >&2
    return 1
  }

  badging=$("$aapt_bin" dump badging "$apk_path" 2>&1) || {
    echo "aapt dump badging 失败: $badging" >&2
    return 1
  }

  package_line=$(printf '%s\n' "$badging" | grep "^package:" | head -1)
  actual_code=$(printf '%s\n' "$package_line" | sed -n "s/.*versionCode='\([^']*\)'.*/\1/p")
  actual_name=$(printf '%s\n' "$package_line" | sed -n "s/.*versionName='\([^']*\)'.*/\1/p")

  if [[ -z "$actual_code" || -z "$actual_name" ]]; then
    echo "aapt package 行缺少 versionCode/versionName: $package_line" >&2
    return 1
  fi

  printf '%s\t%s\n' "$actual_code" "$actual_name"
}

assert_apk_manifest_version() {
  local apk_path="$1"
  local expected_code="$2"
  local expected_name="$3"
  local label="${4:-APK}"
  local actual actual_code actual_name

  actual=$(apk_manifest_version "$apk_path") || {
    complete_release "false" "" 0 "" "failed to read apk manifest version"
    exit 1
  }
  actual_code="${actual%%$'\t'*}"
  actual_name="${actual#*$'\t'}"

  if [[ "$actual_code" != "$expected_code" || "$actual_name" != "$expected_name" ]]; then
    echo -e "${RED}❌ ${label} manifest 版本不匹配：期望 v${expected_name} (build ${expected_code})，实际 v${actual_name} (build ${actual_code})。已停止发布，避免手机端重复更新。${NC}" >&2
    complete_release "false" "" 0 "" "${label} manifest mismatch: expected ${expected_name}/${expected_code}, actual ${actual_name}/${actual_code}"
    exit 1
  fi

  echo -e "${GREEN}   ✅ ${label} manifest: v${actual_name} (build ${actual_code})${NC}"
}

assert_remote_apk_manifest_version() {
  local expected_code="$1"
  local expected_name="$2"
  local tmp_apk

  tmp_apk=$(mktemp /tmp/elon-remote-apk.XXXXXX.apk)
  if ! curl -sS --noproxy '*' -f -L --max-time 120 -o "$tmp_apk" "$SERVER_URL/app/ElonSpeed-latest.apk"; then
    rm -f "$tmp_apk"
    complete_release "false" "" 0 "" "failed to download remote apk for manifest validation"
    echo -e "${RED}❌ 下载线上 APK 校验包体失败${NC}" >&2
    exit 1
  fi

  assert_apk_manifest_version "$tmp_apk" "$expected_code" "$expected_name" "线上 APK"
  rm -f "$tmp_apk"
}

# ── Trap 清理 ────────────────────────────────────────────────
restore_gradle() {
  [[ -z "$ORIGINAL_GRADLE_CONTENT" ]] && return 0
  echo "$ORIGINAL_GRADLE_CONTENT" > "$GRADLE_PATH"
  ORIGINAL_GRADLE_CONTENT=""
}

cleanup() {
  local exit_code=$?
  restore_gradle
  if [[ $exit_code -ne 0 && "$RELEASE_FINISHED" == "0" && -n "$RELEASE_TOKEN" ]]; then
    complete_release "false" "" 0 "" "script exited with code $exit_code" || true
  fi
}
trap cleanup EXIT

# ── 签名配置 ──────────────────────────────────────────────────
get_gradle_property() {
  local name="$1"
  [[ -f "$USER_GRADLE_PROPS" ]] || return 0
  grep "^${name}=" "$USER_GRADLE_PROPS" 2>/dev/null | head -1 | cut -d'=' -f2- | tr -d '"' | xargs || true
}

setup_signing_config() {
  [[ -z "${ELON_RELEASE_KEYSTORE:-}" ]] && {
    val=$(get_gradle_property "ELON_RELEASE_KEYSTORE")
    [[ -n "$val" ]] && export ELON_RELEASE_KEYSTORE="$val"
  }
  [[ -z "${ELON_RELEASE_STORE_PASSWORD:-}" ]] && {
    val=$(get_gradle_property "ELON_RELEASE_STORE_PASSWORD")
    [[ -n "$val" ]] && export ELON_RELEASE_STORE_PASSWORD="$val"
  }
  [[ -z "${ELON_RELEASE_KEY_ALIAS:-}" ]] && {
    val=$(get_gradle_property "ELON_RELEASE_KEY_ALIAS")
    [[ -n "$val" ]] && export ELON_RELEASE_KEY_ALIAS="$val"
  }
  [[ -z "${ELON_RELEASE_KEY_PASSWORD:-}" ]] && {
    val=$(get_gradle_property "ELON_RELEASE_KEY_PASSWORD")
    [[ -n "$val" ]] && export ELON_RELEASE_KEY_PASSWORD="$val"
  }
  # 默认路径
  [[ -z "${ELON_RELEASE_KEYSTORE:-}" && -f "$DEFAULT_KEYSTORE" ]] && \
    export ELON_RELEASE_KEYSTORE="$DEFAULT_KEYSTORE"
  # 默认别名
  [[ -z "${ELON_RELEASE_KEY_ALIAS:-}" ]] && export ELON_RELEASE_KEY_ALIAS="elon"
  return 0
}

assert_signing_config() {
  setup_signing_config
  local missing=()

  [[ -z "${ELON_RELEASE_KEYSTORE:-}" ]] && \
    missing+=("ELON_RELEASE_KEYSTORE（默认: $DEFAULT_KEYSTORE）")
  [[ -n "${ELON_RELEASE_KEYSTORE:-}" && ! -f "${ELON_RELEASE_KEYSTORE}" ]] && \
    missing+=("ELON_RELEASE_KEYSTORE 文件不存在: ${ELON_RELEASE_KEYSTORE}")
  [[ -z "${ELON_RELEASE_STORE_PASSWORD:-}" ]] && \
    missing+=("ELON_RELEASE_STORE_PASSWORD")
  [[ -z "${ELON_RELEASE_KEY_ALIAS:-}" ]] && \
    missing+=("ELON_RELEASE_KEY_ALIAS")
  [[ -z "${ELON_RELEASE_KEY_PASSWORD:-}" ]] && \
    missing+=("ELON_RELEASE_KEY_PASSWORD")

  if [[ ${#missing[@]} -gt 0 ]]; then
    echo -e "${YELLOW}缺少 APK 签名配置：${NC}" >&2
    for m in "${missing[@]}"; do echo -e "  - $m" >&2; done
    echo "" >&2
    echo -e "${CYAN}一次性推荐设置（存入 ~/.gradle/gradle.properties，不进 git）：${NC}" >&2
    echo -e "${CYAN}  1. 将 elon-release.jks 放到 $DEFAULT_KEYSTORE${NC}" >&2
    echo -e "${CYAN}     mkdir -p ~/.elon/signing && scp <本机路径>/elon-release.jks \$(hostname):$DEFAULT_KEYSTORE${NC}" >&2
    echo -e "${CYAN}  2. 配置 ~/.gradle/gradle.properties：${NC}" >&2
    echo -e "${CYAN}     ELON_RELEASE_KEYSTORE=$DEFAULT_KEYSTORE${NC}" >&2
    echo -e "${CYAN}     ELON_RELEASE_STORE_PASSWORD=<密码>${NC}" >&2
    echo -e "${CYAN}     ELON_RELEASE_KEY_ALIAS=elon${NC}" >&2
    echo -e "${CYAN}     ELON_RELEASE_KEY_PASSWORD=<密码>${NC}" >&2
    exit 1
  fi
}

# ── SHA 辅助 ──────────────────────────────────────────────────
get_deployed_apk_sha() {
  local sha
  sha=$(curl -s --noproxy '*' --max-time 10 "$SERVER_URL/app/version.json" 2>/dev/null | \
    python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('gitSha',''))" 2>/dev/null || true)
  if [[ "$sha" =~ ^[0-9a-f]{40}$ ]]; then
    echo "$sha"; return
  fi
  sha=$(ssh $SSH_OPTS "$SERVER_HOST" "cat $APK_SHA_FILE 2>/dev/null || true" 2>/dev/null | \
    tr -d '[:space:]' || true)
  [[ "$sha" =~ ^[0-9a-f]{40}$ ]] && echo "$sha" || echo ""
}

apk_runtime_unchanged_since() {
  local base_sha="$1"
  [[ "$base_sha" =~ ^[0-9a-f]{40}$ ]] || return 1
  git -C "$REPO_ROOT" merge-base --is-ancestor "$base_sha" "$BUILD_BASE_SHA" 2>/dev/null || return 1
  git -C "$REPO_ROOT" diff --quiet "$base_sha" "$BUILD_BASE_SHA" -- \
    android/app/src/main android/app/build.gradle android/build.gradle android/settings.gradle
}

is_git_ancestor() {
  local ancestor="$1" descendant="$2"
  [[ "$ancestor" =~ ^[0-9a-f]{40}$ && "$descendant" =~ ^[0-9a-f]{40}$ ]] || return 1
  git -C "$REPO_ROOT" merge-base --is-ancestor "$ancestor" "$descendant" 2>/dev/null
}

live_apk_includes_build_base() {
  local live_json live_sha
  git_fetch_with_retry 1 || return 1
  live_json=$(curl -s --noproxy '*' --max-time 10 "$SERVER_URL/app/version.json" 2>/dev/null || true)
  [[ -n "$live_json" ]] || return 1
  live_sha=$(json_get "$live_json" "sourceSha")
  [[ -z "$live_sha" ]] && live_sha=$(json_get "$live_json" "gitSha")
  is_git_ancestor "$BUILD_BASE_SHA" "$live_sha"
}

remote_advance_safe_for_apk() {
  local base_sha="$1" changed p
  git_fetch_with_retry 1 || return 1
  changed=$(git -C "$REPO_ROOT" diff --name-only "$base_sha..origin/main" 2>/dev/null || true)
  [[ -n "$changed" ]] || return 0
  while IFS= read -r p; do
    [[ -n "$p" ]] || continue
    if [[ "$p" == android/* || "$p" == scripts/publish-apk* ]]; then
      return 1
    fi
  done <<< "$changed"
  return 0
}

# ═══════════════════════════════════════════════════════════════
# Step 0: Git fetch + fast-forward
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}🔄 同步最新代码...${NC}"
git_fetch_with_retry

DIRTY=$(git -C "$REPO_ROOT" status --porcelain)
if [[ -n "$DIRTY" ]]; then
  echo -e "${RED}❌ 工作区不干净，请先 commit + push 业务改动再运行 APK 发布脚本：${NC}" >&2
  echo "$DIRTY" >&2
  exit 1
fi

LOCAL_HEAD=$(git -C "$REPO_ROOT" rev-parse HEAD)
ORIGIN_MAIN_SHA=$(git -C "$REPO_ROOT" rev-parse origin/main)
if [[ "$LOCAL_HEAD" != "$ORIGIN_MAIN_SHA" ]]; then
  if git -C "$REPO_ROOT" merge-base --is-ancestor "$LOCAL_HEAD" "$ORIGIN_MAIN_SHA" 2>/dev/null; then
    echo -e "${CYAN}   ℹ️  本地 HEAD 已包含在 origin/main 中，快进到最新 main：${ORIGIN_MAIN_SHA:0:7}${NC}"
    git -C "$REPO_ROOT" merge --ff-only origin/main
  elif git -C "$REPO_ROOT" merge-base --is-ancestor "$ORIGIN_MAIN_SHA" "$LOCAL_HEAD" 2>/dev/null; then
    echo -e "${RED}❌ 当前 HEAD 尚未进入 origin/main，禁止基于未推送提交发布 APK。请先执行：git push origin HEAD:main${NC}" >&2
    exit 1
  else
    echo -e "${RED}❌ 当前 HEAD 与 origin/main 已分叉，APK 发布脚本不会自动 rebase。请先完成代码合并并 push 后再运行。${NC}" >&2
    exit 1
  fi
fi

BUILD_BASE_SHA=$(git -C "$REPO_ROOT" rev-parse HEAD)
ORIGIN_MAIN_SHA=$(git -C "$REPO_ROOT" rev-parse origin/main)

if [[ "$BUILD_BASE_SHA" != "$ORIGIN_MAIN_SHA" ]]; then
  if git -C "$REPO_ROOT" merge-base --is-ancestor "$ORIGIN_MAIN_SHA" "$BUILD_BASE_SHA" 2>/dev/null; then
    echo -e "${YELLOW}   ℹ️  检测到本地存在待发布业务提交，APK freshness 基线改为 origin/main${NC}"
    BUILD_BASE_SHA="$ORIGIN_MAIN_SHA"
  fi
fi
echo -e "${GREEN}   基线 SHA: ${BUILD_BASE_SHA:0:7}${NC}"

DEPLOYED_APK_SHA=$(get_deployed_apk_sha)
if [[ "$FORCE" == "0" && -n "$DEPLOYED_APK_SHA" ]]; then
  if [[ "$DEPLOYED_APK_SHA" == "$BUILD_BASE_SHA" ]] || apk_runtime_unchanged_since "$DEPLOYED_APK_SHA"; then
    LIVE_VERSION=$(curl -s --noproxy '*' --max-time 10 "$SERVER_URL/app/version.json" 2>/dev/null || true)
    LIVE_NAME=$(json_get "$LIVE_VERSION" "versionName")
    LIVE_CODE=$(json_get_int "$LIVE_VERSION" "versionCode")
    echo -e "${GREEN}   ✅ APK 运行代码未变化，复用线上发布版 v${LIVE_NAME} (build ${LIVE_CODE})${NC}"
    echo -e "${GREEN}   下载: $SERVER_URL/app/ElonSpeed-latest.apk${NC}"
    print_publish_status "published" "synced" "not_attempted" "APK 已发布；当前 Android 运行代码未变化，复用线上版本。"
    echo -e "${GRAY}      如需强制重新打包：bash scripts/publish-apk.sh --force --changelog=\"$CHANGELOG\"${NC}"
    exit 0
  fi
fi

# ═══════════════════════════════════════════════════════════════
# Step 1: Claim 版本号
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}📝 向服务器申请新版本号...${NC}"

ORIGINAL_GRADLE_CONTENT=$(cat "$GRADLE_PATH")
OLD_CODE=$(grep 'versionCode ' "$GRADLE_PATH" | grep -oE '[0-9]+' | head -1)
OLD_NAME=$(grep 'versionName' "$GRADLE_PATH" | grep -oP '"[^"]+"' | tr -d '"' | head -1)

echo -e "${GRAY}   build.gradle 兜底: v${OLD_NAME} (build ${OLD_CODE}) — 不会被本次脚本提交${NC}"

BASELINE=$(resolve_apk_version_baseline) || exit 1
CLAIM_BASE_CODE="${BASELINE%%|*}"
BASELINE_REST="${BASELINE#*|}"
CLAIM_BASE_NAME="${BASELINE_REST%%|*}"
CLAIM_BASE_SOURCE="${BASELINE_REST#*|}"
echo -e "${GRAY}   服务器 APK 版本基线: v${CLAIM_BASE_NAME} (build ${CLAIM_BASE_CODE}) [${CLAIM_BASE_SOURCE}]${NC}"

BUILDER_ID="${HOSTNAME:-unknown}-${USER:-unknown}"
BUILDER_LABEL="publish-apk.sh @ $BUILDER_ID"

CLAIM_BODY=$(python3 -c "import json; print(json.dumps({'kind':'apk','sha':'$BUILD_BASE_SHA',
  'builderId':'$BUILDER_ID','builderLabel':'$BUILDER_LABEL','bump':'patch','currentVersionName':'$CLAIM_BASE_NAME','currentVersionCode':$CLAIM_BASE_CODE}))")
CLAIM_RESP=$(call_release_api "claim" "$CLAIM_BODY") || {
  echo -e "${RED}❌ /api/release/claim 失败${NC}" >&2; exit 1
}
CLAIM_RESP=$(enter_global_publish_lease "$CLAIM_RESP" apk "$RELEASE_API_BASE") || exit 1
[[ -n "$CLAIM_RESP" ]] || exit 0
CLAIM_ACTION=$(json_get "$CLAIM_RESP" action)
if [[ "$CLAIM_ACTION" == "coalesced" || "$CLAIM_ACTION" == "finished" ]]; then
  echo -e "${GREEN}   同一 SHA 已发布完成，本次请求已合并，无需重复构建。${NC}"
  exit 0
fi

RELEASE_TOKEN=$(json_get "$CLAIM_RESP" "token")
NEW_CODE=$(json_get_int "$CLAIM_RESP" "assignedVersionCode")
NEW_NAME=$(json_get "$CLAIM_RESP" "assignedVersionName")
if [[ -z "$RELEASE_TOKEN" || -z "$NEW_NAME" || "$NEW_CODE" -le 0 ]]; then
  echo -e "${RED}❌ release/claim 未返回有效的版本号: $CLAIM_RESP${NC}" >&2; exit 1
fi

if [[ "$NEW_CODE" -le "$CLAIM_BASE_CODE" ]]; then
  complete_release "false" "" 0 "" "claim returned non-incrementing apk build $NEW_CODE from baseline $CLAIM_BASE_CODE"
  echo -e "${RED}❌ release/claim 分配的 build $NEW_CODE 未高于服务器基线 build $CLAIM_BASE_CODE，已停止发布。${NC}" >&2
  exit 1
fi

echo -e "${GREEN}   ✅ 已分配版本号: v${NEW_NAME} (build ${NEW_CODE})${NC}"

if [[ "$FORCE" == "0" ]] && live_apk_includes_build_base; then
  LIVE_VERSION=$(curl -s --noproxy '*' --max-time 10 "$SERVER_URL/app/version.json" 2>/dev/null || true)
  LIVE_NAME=$(json_get "$LIVE_VERSION" "versionName")
  LIVE_CODE=$(json_get_int "$LIVE_VERSION" "versionCode")
  echo -e "${GREEN}   ✅ 线上 APK 已包含本次源码，复用 v${LIVE_NAME} (build ${LIVE_CODE})${NC}"
  echo -e "${GREEN}   下载: $SERVER_URL/app/ElonSpeed-latest.apk${NC}"
  complete_release "false" "" 0 "" "live apk already includes build base"
  print_publish_status "published" "synced" "not_attempted" "APK 已发布；线上 APK 已包含本次源码。"
  exit 0
fi

# 临时写入 build.gradle（编译后自动还原，不进 git）
sed -i "s/versionCode ${OLD_CODE}/versionCode ${NEW_CODE}/" "$GRADLE_PATH"
sed -i "s/versionName \"${OLD_NAME}\"/versionName \"${NEW_NAME}\"/" "$GRADLE_PATH"
echo -e "${GREEN}   versionCode: ${OLD_CODE} → ${NEW_CODE} (临时，编译后自动还原)${NC}"

# ═══════════════════════════════════════════════════════════════
# Step 2: 编译 Release APK
# ═══════════════════════════════════════════════════════════════
if [[ "$SKIP_BUILD" == "0" ]]; then
  assert_signing_config
  echo -e "${CYAN}🔨 编译 Release APK...${NC}"
  cd "$ANDROID_DIR"
  if [[ -f "$ANDROID_DIR/gradlew" ]]; then
    chmod +x "$ANDROID_DIR/gradlew"
    ./gradlew assembleRelease
  elif [[ -n "${GRADLE_BIN:-}" && -x "$GRADLE_BIN" ]]; then
    "$GRADLE_BIN" assembleRelease --no-daemon
  elif command -v gradle >/dev/null 2>&1; then
    gradle assembleRelease --no-daemon
  elif [[ -x "$HOME/.gradle/wrapper/dists/gradle-8.6-bin/afr5mpiioh2wthjmwnkmdsd5w/gradle-8.6/bin/gradle" ]]; then
    "$HOME/.gradle/wrapper/dists/gradle-8.6-bin/afr5mpiioh2wthjmwnkmdsd5w/gradle-8.6/bin/gradle" assembleRelease --no-daemon
  elif [[ -x "$HOME/.gradle/wrapper/dists/gradle-8.6-bin/9kfl6m3v6ux5ki4g2exnpl853/gradle-8.6/bin/gradle" ]]; then
    "$HOME/.gradle/wrapper/dists/gradle-8.6-bin/9kfl6m3v6ux5ki4g2exnpl853/gradle-8.6/bin/gradle" assembleRelease --no-daemon
  else
    java -classpath "$ANDROID_DIR/gradle/wrapper/gradle-wrapper.jar" \
      org.gradle.wrapper.GradleWrapperMain assembleRelease
  fi
  cd "$REPO_ROOT"
else
  echo -e "${YELLOW}⏭️  跳过编译（--skip-build）${NC}"
fi

# ═══════════════════════════════════════════════════════════════
# Step 3: 找到 APK 文件
# ═══════════════════════════════════════════════════════════════
APK_PATH=$(find "$APK_DIR" -name "*.apk" 2>/dev/null | sort | tail -1 || true)
if [[ -z "$APK_PATH" ]]; then
  echo -e "${RED}❌ 未找到 APK 文件: $APK_DIR${NC}" >&2; exit 1
fi
FILE_SIZE=$(stat -c%s "$APK_PATH" 2>/dev/null || stat -f%z "$APK_PATH")
assert_apk_manifest_version "$APK_PATH" "$NEW_CODE" "$NEW_NAME" "本地 release APK"
echo -e "${GREEN}📦 APK: $(basename "$APK_PATH") ($(python3 -c "print(round($FILE_SIZE/1024/1024,2))") MB)${NC}"

# ═══════════════════════════════════════════════════════════════
# Step 4: 还原 build.gradle（版本号不进 git）
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}🧹 还原 build.gradle 到 git 兜底版本 (v${OLD_NAME} / build ${OLD_CODE})...${NC}"
restore_gradle

SHA_FULL="$BUILD_BASE_SHA"
SHA_SHORT="${SHA_FULL:0:7}"
echo -e "${GREEN}   本次发布对应源 SHA: ${SHA_SHORT} (无新增版本号提交)${NC}"

# ═══════════════════════════════════════════════════════════════
# Step 5: 上传前检查（防慢构建覆盖）
# ═══════════════════════════════════════════════════════════════
git_fetch_with_retry 1
REMOTE_HEAD_NOW=$(git -C "$REPO_ROOT" rev-parse origin/main)
if [[ "$REMOTE_HEAD_NOW" != "$SHA_FULL" ]]; then
  if remote_advance_safe_for_apk "$SHA_FULL"; then
    echo -e "${CYAN}   ℹ️  origin/main 已前进到 ${REMOTE_HEAD_NOW:0:7}，但新提交不影响 Android，继续发布。${NC}"
  else
    echo -e "${CYAN}⏭️  origin/main 已从本次基础 ${SHA_SHORT} 前进到 ${REMOTE_HEAD_NOW:0:7}，且包含 Android 改动。为避免上传过期 APK，已停止；代码已合并，发布交给最新主线。${NC}"
    complete_release "false" "" 0 "" "origin/main moved to $REMOTE_HEAD_NOW and changed android files"
    print_publish_status "superseded_by_newer_main" "synced" "not_attempted" "代码已合并，发布交给最新主线。"
    exit 0
  fi
fi

SERVER_SHA_BEFORE=$(get_deployed_apk_sha)
[[ -z "$SERVER_SHA_BEFORE" ]] && SERVER_SHA_BEFORE=""

if [[ "$FORCE" == "0" ]]; then
  SERVER_NOW=$(curl -s --noproxy '*' --max-time 10 "$SERVER_URL/app/version.json" 2>/dev/null || true)
  if [[ -z "$SERVER_NOW" ]]; then
    complete_release "false" "" 0 "" "could not read server apk version before upload"
    echo -e "${RED}❌ 上传前无法读取服务器 version.json，已停止发布，避免覆盖服务器上的未知新版本。${NC}" >&2
    exit 1
  fi
  if [[ -n "$SERVER_NOW" ]]; then
    SERVER_NOW_CODE=$(json_get_int "$SERVER_NOW" "versionCode")
    if [[ "$SERVER_NOW_CODE" -le 0 ]]; then
      complete_release "false" "" 0 "" "invalid server apk version before upload"
      echo -e "${RED}❌ 上传前读取到的服务器 version.json 无有效 versionCode，已停止发布。${NC}" >&2
      exit 1
    fi
    if [[ "$SERVER_NOW_CODE" -ge "$NEW_CODE" ]]; then
      if live_apk_includes_build_base; then
        SERVER_NOW_NAME=$(json_get "$SERVER_NOW" "versionName")
        echo -e "${GREEN}   ✅ 服务器已有更新 APK 且包含本次源码，复用 v${SERVER_NOW_NAME} (build ${SERVER_NOW_CODE})${NC}"
        complete_release "false" "" 0 "" "superseded by live apk that includes build base"
        print_publish_status "published" "synced" "not_attempted" "APK 已发布；服务器已有更新 APK 且包含本次源码。"
      else
        echo -e "${YELLOW}⚠️  APK 发布已中止：服务器已有更新版本 (build $SERVER_NOW_CODE >= $NEW_CODE)${NC}"
        echo -e "${YELLOW}   处理：代码已合并，发布交给最新主线；如确要覆盖（不推荐）：重跑加 --force${NC}"
        complete_release "false" "" 0 "" "server already has newer apk: build $SERVER_NOW_CODE"
        print_publish_status "superseded_by_newer_main" "synced" "not_attempted" "代码已合并，发布交给最新主线。"
      fi
      exit 0
    fi
    echo -e "${GREEN}   ✅ 服务器版本检查通过 (服务器 $SERVER_NOW_CODE < 本次 $NEW_CODE)${NC}"
  fi
fi

# ═══════════════════════════════════════════════════════════════
# Step 5: 生成 version.json
# ═══════════════════════════════════════════════════════════════
TMP_JSON=$(mktemp /tmp/elon-version.XXXXXX.json)
trap "rm -f '$TMP_JSON'; $(trap -p EXIT | sed 's/trap -- //' | sed "s/ EXIT//")" EXIT

python3 -c "
import json
print(json.dumps({
  'versionCode': $NEW_CODE,
  'versionName': '$NEW_NAME',
  'downloadUrl': '$SERVER_URL/app/ElonSpeed-latest.apk',
  'changelog': $(python3 -c "import json; print(json.dumps('$CHANGELOG'))"),
  'forceUpdate': False,
  'fileSize': $FILE_SIZE,
  'gitSha': '$SHA_FULL',
  'sourceSha': '$BUILD_BASE_SHA'
}, indent=2))
" > "$TMP_JSON"
echo -e "${GREEN}📋 version.json 已生成${NC}"

# ═══════════════════════════════════════════════════════════════
# Step 6: SCP 上传（staged 原子替换，flock+CAS）
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}🚀 上传到服务器...${NC}"
if is_local_apk_deploy; then
  echo -e "${GRAY}   部署模式: 本机上传（跳过 SSH/SCP）${NC}"
else
  echo -e "${GRAY}   部署模式: 远程 SSH/SCP${NC}"
fi

APK_STAGE="$SERVER_DIR/ElonSpeed-latest.apk.${SHA_FULL}.tmp"
JSON_STAGE="$SERVER_DIR/version.json.${SHA_FULL}.tmp"

if is_local_apk_deploy; then
  mkdir -p "$SERVER_DIR"
  cp "$APK_PATH" "$APK_STAGE"
else
  # shellcheck disable=SC2086
  ssh $SSH_OPTS "$SERVER_HOST" "mkdir -p $SERVER_DIR"
  # shellcheck disable=SC2086
  scp -o ProxyCommand=none "$APK_PATH" "${SERVER_HOST}:${APK_STAGE}"
fi
echo -e "${GREEN}   ✅ APK staging 上传完成${NC}"
if is_local_apk_deploy; then
  cp "$TMP_JSON" "$JSON_STAGE"
else
  # shellcheck disable=SC2086
  scp -o ProxyCommand=none "$TMP_JSON" "${SERVER_HOST}:${JSON_STAGE}"
fi
echo -e "${GREEN}   ✅ version.json staging 上传完成${NC}"

REMOTE_SCRIPT=$(cat <<'BASH_EOF'
set -eu
APP_DIR='__APP_DIR__'
EXPECTED='__EXPECTED__'
NEW_SHA='__NEW_SHA__'
APK_STAGE='__APK_STAGE__'
JSON_STAGE='__JSON_STAGE__'
LOCK_FILE="$APP_DIR/.apk-deploy.lock"
SHA_FILE="$APP_DIR/.apk-deployed-sha"
(
  flock -x 9
  CURRENT=""
  if [ -f "$SHA_FILE" ]; then
    CURRENT="$(cat "$SHA_FILE" 2>/dev/null || true)"
  fi
  if [ "$CURRENT" != "$EXPECTED" ]; then
    echo "APK_DEPLOY_CAS_MISMATCH current=$CURRENT expected=$EXPECTED" >&2
    exit 42
  fi
  mv "$APK_STAGE" "$APP_DIR/ElonSpeed-latest.apk"
  mv "$JSON_STAGE" "$APP_DIR/version.json"
  printf '%s\n' "$NEW_SHA" > "$SHA_FILE"
) 9>"$LOCK_FILE"
BASH_EOF
)

REMOTE_SCRIPT="${REMOTE_SCRIPT//__APP_DIR__/$SERVER_DIR}"
REMOTE_SCRIPT="${REMOTE_SCRIPT//__EXPECTED__/$SERVER_SHA_BEFORE}"
REMOTE_SCRIPT="${REMOTE_SCRIPT//__NEW_SHA__/$SHA_FULL}"
REMOTE_SCRIPT="${REMOTE_SCRIPT//__APK_STAGE__/$APK_STAGE}"
REMOTE_SCRIPT="${REMOTE_SCRIPT//__JSON_STAGE__/$JSON_STAGE}"

set +e
if is_local_apk_deploy; then
  echo "$REMOTE_SCRIPT" | bash -s
else
  # shellcheck disable=SC2086
  echo "$REMOTE_SCRIPT" | ssh $SSH_OPTS "$SERVER_HOST" "bash -s"
fi
DEPLOY_EXIT=$?
set -e

if [[ $DEPLOY_EXIT -eq 42 ]]; then
  DEPLOYED_SHA=$(get_deployed_apk_sha || true)
  if [[ -n "$DEPLOYED_SHA" ]] && is_git_ancestor "$SHA_FULL" "$DEPLOYED_SHA" 2>/dev/null; then
    echo -e "${CYAN}⏭️  另一台机器已部署更新 APK，本次 staging 不覆盖${NC}"
    if is_local_apk_deploy; then
      rm -f "$APK_STAGE" "$JSON_STAGE" 2>/dev/null || true
    else
      # shellcheck disable=SC2086
      ssh $SSH_OPTS "$SERVER_HOST" "rm -f '$APK_STAGE' '$JSON_STAGE'" > /dev/null 2>&1 || true
    fi
    complete_release "false" "" 0 "" "superseded by deployed apk $DEPLOYED_SHA"
    print_publish_status "published" "synced" "not_attempted" "APK 已由更新主线发布，当前 staging 不覆盖。"
    exit 0
  fi
  echo -e "${CYAN}⏭️  APK 上传 CAS 失败（并发冲突）。本次 staging 不覆盖；代码已合并，发布交给最新主线。${NC}"
  if is_local_apk_deploy; then
    rm -f "$APK_STAGE" "$JSON_STAGE" 2>/dev/null || true
  else
    # shellcheck disable=SC2086
    ssh $SSH_OPTS "$SERVER_HOST" "rm -f '$APK_STAGE' '$JSON_STAGE'" > /dev/null 2>&1 || true
  fi
  complete_release "false" "" 0 "" "cas mismatch in apk deploy"
  print_publish_status "superseded_by_newer_main" "synced" "not_attempted" "代码已合并，发布交给最新主线。"
  exit 0
fi

if [[ $DEPLOY_EXIT -ne 0 ]]; then
  echo -e "${RED}❌ 服务器 APK 原子发布失败，退出码 $DEPLOY_EXIT${NC}" >&2
  exit 1
fi

echo -e "${GREEN}   ✅ APK 原子发布完成，.apk-deployed-sha = ${SHA_SHORT}${NC}"

# ═══════════════════════════════════════════════════════════════
# Step 7: 验证
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}🔍 验证服务器响应...${NC}"
sleep 1

RESP=$(curl -s --noproxy '*' --max-time 10 "$SERVER_URL/app/version.json" 2>/dev/null || true)
if [[ -n "$RESP" ]]; then
  RESP_CODE=$(json_get_int "$RESP" "versionCode")
  RESP_NAME=$(json_get "$RESP" "versionName")
  echo -e "${GREEN}   服务器返回: v${RESP_NAME} (build ${RESP_CODE})${NC}"
  if [[ "$RESP_CODE" == "$NEW_CODE" ]]; then
    echo -e "${GREEN}   ✅ versionCode 一致，发布成功！${NC}"
  else
    echo -e "${YELLOW}   ⚠️  服务器 versionCode=$RESP_CODE，期望 $NEW_CODE${NC}"
  fi
fi
assert_remote_apk_manifest_version "$NEW_CODE" "$NEW_NAME"

# 广播在线客户端更新
echo -e "${CYAN}📣 广播在线客户端更新提醒...${NC}"
curl -s --noproxy '*' --max-time 10 -X POST "$SERVER_URL/api/app/update/broadcast" > /dev/null 2>&1 || true

# ── finish ───────────────────────────────────────────────────
complete_release "true" "$NEW_NAME" "$NEW_CODE" "$SHA_FULL"

SEP=$(printf '=%.0s' {1..60})
echo ""
echo -e "${CYAN}${SEP}${NC}"
echo -e "${GREEN}✅ 发布完成！${NC}"
echo -e "   版本: v${NEW_NAME} (build ${NEW_CODE}) — 服务器分配，未写入 git"
echo -e "   SHA:  ${SHA_SHORT} (源代码提交，无新增版本号提交)"
echo -e "   下载: $SERVER_URL/app/ElonSpeed-latest.apk"
print_publish_status "published"
echo -e "${CYAN}${SEP}${NC}"

# 自动清理已合并、工作树干净的孤儿 task worktree
. "$SCRIPT_DIR/apk-publish-postflight.sh"
invoke_elon_apk_worktree_cleanup "$REPO_ROOT"
