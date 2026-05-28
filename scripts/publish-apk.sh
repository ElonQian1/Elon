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

# ── 路径推导 ──────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
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

is_git_ancestor() {
  local ancestor="$1" descendant="$2"
  [[ "$ancestor" =~ ^[0-9a-f]{40}$ && "$descendant" =~ ^[0-9a-f]{40}$ ]] || return 1
  git -C "$REPO_ROOT" merge-base --is-ancestor "$ancestor" "$descendant" 2>/dev/null
}

# ═══════════════════════════════════════════════════════════════
# Step 0: Git sync
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}🔄 同步最新代码...${NC}"
git -C "$REPO_ROOT" pull --rebase origin main

BUILD_BASE_SHA=$(git -C "$REPO_ROOT" rev-parse HEAD)
ORIGIN_MAIN_SHA=$(git -C "$REPO_ROOT" rev-parse origin/main)

if [[ "$BUILD_BASE_SHA" != "$ORIGIN_MAIN_SHA" ]]; then
  if git -C "$REPO_ROOT" merge-base --is-ancestor "$ORIGIN_MAIN_SHA" "$BUILD_BASE_SHA" 2>/dev/null; then
    echo -e "${YELLOW}   ℹ️  检测到本地存在待发布业务提交，APK freshness 基线改为 origin/main${NC}"
    BUILD_BASE_SHA="$ORIGIN_MAIN_SHA"
  fi
fi
echo -e "${GREEN}   基线 SHA: ${BUILD_BASE_SHA:0:7}${NC}"

# ═══════════════════════════════════════════════════════════════
# Step 1: Claim 版本号
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}📝 向服务器申请新版本号...${NC}"

ORIGINAL_GRADLE_CONTENT=$(cat "$GRADLE_PATH")
OLD_CODE=$(grep 'versionCode ' "$GRADLE_PATH" | grep -oE '[0-9]+' | head -1)
OLD_NAME=$(grep 'versionName' "$GRADLE_PATH" | grep -oP '"[^"]+"' | tr -d '"' | head -1)

echo -e "${GRAY}   build.gradle 兜底: v${OLD_NAME} (build ${OLD_CODE}) — 不会被本次脚本提交${NC}"

BUILDER_ID="${HOSTNAME:-unknown}-${USER:-unknown}"
BUILDER_LABEL="publish-apk.sh @ $BUILDER_ID"

CLAIM_BODY=$(python3 -c "import json; print(json.dumps({
  'kind': 'apk',
  'sha': '$BUILD_BASE_SHA',
  'builderId': '$BUILDER_ID',
  'builderLabel': '$BUILDER_LABEL',
  'bump': 'patch',
  'currentVersionName': '$OLD_NAME',
  'currentVersionCode': $OLD_CODE
}))")

CLAIM_RESP=$(call_release_api "claim" "$CLAIM_BODY") || {
  echo -e "${RED}❌ /api/release/claim 失败${NC}" >&2; exit 1
}

RELEASE_TOKEN=$(json_get "$CLAIM_RESP" "token")
NEW_CODE=$(json_get_int "$CLAIM_RESP" "assignedVersionCode")
NEW_NAME=$(json_get "$CLAIM_RESP" "assignedVersionName")

if [[ -z "$RELEASE_TOKEN" || -z "$NEW_NAME" || "$NEW_CODE" -le 0 ]]; then
  echo -e "${RED}❌ release/claim 未返回有效的版本号: $CLAIM_RESP${NC}" >&2; exit 1
fi

echo -e "${GREEN}   ✅ 已分配版本号: v${NEW_NAME} (build ${NEW_CODE})${NC}"

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
  chmod +x "$ANDROID_DIR/gradlew"
  cd "$ANDROID_DIR"
  ./gradlew assembleRelease
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
echo -e "${GREEN}📦 APK: $(basename "$APK_PATH") ($(python3 -c "print(round($FILE_SIZE/1024/1024,2))") MB)${NC}"

# ═══════════════════════════════════════════════════════════════
# Step 4: 还原 build.gradle（版本号不进 git）
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}🧹 还原 build.gradle 到 git 兜底版本 (v${OLD_NAME} / build ${OLD_CODE})...${NC}"
restore_gradle

SHA_FULL="$BUILD_BASE_SHA"
SHA_SHORT="${SHA_FULL:0:7}"
echo -e "${GREEN}   本次发布对应源 SHA: ${SHA_SHORT} (无新增 release commit)${NC}"

# ═══════════════════════════════════════════════════════════════
# Step 5: 上传前检查（防慢构建覆盖）
# ═══════════════════════════════════════════════════════════════
SERVER_SHA_BEFORE=$(get_deployed_apk_sha)
[[ -z "$SERVER_SHA_BEFORE" ]] && SERVER_SHA_BEFORE=""

if [[ "$FORCE" == "0" ]]; then
  SERVER_NOW=$(curl -s --noproxy '*' --max-time 10 "$SERVER_URL/app/version.json" 2>/dev/null || true)
  if [[ -n "$SERVER_NOW" ]]; then
    SERVER_NOW_CODE=$(json_get_int "$SERVER_NOW" "versionCode")
    if [[ "$SERVER_NOW_CODE" -ge "$NEW_CODE" ]]; then
      echo -e "${YELLOW}⚠️  APK 发布已中止：服务器已有更新版本 (build $SERVER_NOW_CODE >= $NEW_CODE)${NC}"
      echo -e "${YELLOW}   如确要覆盖（不推荐）：重跑加 --force${NC}"
      complete_release "false" "" 0 "" "server already has newer apk: build $SERVER_NOW_CODE"
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

APK_STAGE="$SERVER_DIR/ElonSpeed-latest.apk.${SHA_FULL}.tmp"
JSON_STAGE="$SERVER_DIR/version.json.${SHA_FULL}.tmp"

# shellcheck disable=SC2086
ssh $SSH_OPTS "$SERVER_HOST" "mkdir -p $SERVER_DIR"
# shellcheck disable=SC2086
scp -o ProxyCommand=none "$APK_PATH" "${SERVER_HOST}:${APK_STAGE}"
echo -e "${GREEN}   ✅ APK staging 上传完成${NC}"
# shellcheck disable=SC2086
scp -o ProxyCommand=none "$TMP_JSON" "${SERVER_HOST}:${JSON_STAGE}"
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
# shellcheck disable=SC2086
echo "$REMOTE_SCRIPT" | ssh $SSH_OPTS "$SERVER_HOST" "bash -s"
DEPLOY_EXIT=$?
set -e

if [[ $DEPLOY_EXIT -eq 42 ]]; then
  DEPLOYED_SHA=$(get_deployed_apk_sha || true)
  if [[ -n "$DEPLOYED_SHA" ]] && is_git_ancestor "$SHA_FULL" "$DEPLOYED_SHA" 2>/dev/null; then
    echo -e "${CYAN}⏭️  另一台机器已部署更新 APK，本次 staging 不覆盖${NC}"
    # shellcheck disable=SC2086
    ssh $SSH_OPTS "$SERVER_HOST" "rm -f '$APK_STAGE' '$JSON_STAGE'" > /dev/null 2>&1 || true
    complete_release "false" "" 0 "" "superseded by deployed apk $DEPLOYED_SHA"
    exit 0
  fi
  echo -e "${RED}❌ APK 上传 CAS 失败（并发冲突），请重新运行发布脚本${NC}" >&2
  # shellcheck disable=SC2086
  ssh $SSH_OPTS "$SERVER_HOST" "rm -f '$APK_STAGE' '$JSON_STAGE'" > /dev/null 2>&1 || true
  exit 1
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
echo -e "   SHA:  ${SHA_SHORT} (源代码 commit，无新增 release commit)"
echo -e "   下载: $SERVER_URL/app/ElonSpeed-latest.apk"
echo -e "${CYAN}${SEP}${NC}"
