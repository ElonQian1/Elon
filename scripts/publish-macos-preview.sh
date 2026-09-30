#!/usr/bin/env bash
# macOS-only preview lane; no Windows node/server release or credentials involved.
set -euo pipefail
[[ "$(uname -s)" == Darwin ]] || { echo 'macOS build host required' >&2; exit 1; }
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
[[ -z "$(git status --porcelain --untracked-files=normal)" ]] || {
  echo 'Preview publishing requires a clean committed checkout' >&2; exit 1;
}
[[ "${GITHUB_RUN_NUMBER:-}" =~ ^[0-9]+$ ]] || {
  echo 'GITHUB_RUN_NUMBER required for preview version allocation' >&2; exit 1;
}
version="0.1.${GITHUB_RUN_NUMBER}"
git_sha="$(git rev-parse HEAD)"
[[ "$git_sha" == "${GITHUB_SHA:-}" ]] || { echo 'Build SHA mismatch' >&2; exit 1; }
export ELON_DESKTOP_RELEASE_IDENTITY="${version}+${git_sha}"
export MACOSX_DEPLOYMENT_TARGET=14.0
export APPLE_SIGNING_IDENTITY=-
mkdir -p .ai-tmp/macos-preview
output="$repo_root/.ai-tmp/macos-preview"
export PREVIEW_VERSION="$version"
export PREVIEW_GIT_SHA="$git_sha"
export PREVIEW_OUTPUT="$output"
npx --yes @tauri-apps/cli@2.12.0 icon desktop-shell/src-tauri/icons/icon.png --output .ai-tmp/macos-icons

# Merge the Mac-only config; no tracked Cargo/Tauri version edits.
node --input-type=module <<'JS'
import fs from 'node:fs';
import path from 'node:path';
const config = JSON.parse(fs.readFileSync('desktop-shell/src-tauri/tauri.macos.conf.json'));
config.version = process.env.PREVIEW_VERSION;
config.bundle.icon = [path.resolve('.ai-tmp/macos-icons/icon.icns'), path.resolve('desktop-shell/src-tauri/icons/icon.png')];
fs.writeFileSync('.ai-tmp/tauri.macos.release.json', JSON.stringify(config, null, 2));
JS
echo "MACOS_PREVIEW_VERSION=$version"
echo "MACOS_PREVIEW_GIT_SHA=$git_sha"
bash scripts/cargo-dev.sh test --manifest-path desktop-shell/src-tauri/Cargo.toml --locked
(
  cd desktop-shell
  npx --yes @tauri-apps/cli@2.12.0 build --ci --target universal-apple-darwin \
    --config "$repo_root/.ai-tmp/tauri.macos.release.json" --bundles app,dmg
)
bundle_root="${CARGO_TARGET_DIR:?}/universal-apple-darwin/release/bundle"
app="$bundle_root/macos/一龙工作台 Mac 测试版.app"
binary="$app/Contents/MacOS/elon-desktop"
[[ -f "$binary" ]] || { echo 'Bundled executable missing' >&2; exit 1; }
lipo "$binary" -verify_arch arm64 x86_64
codesign --verify --deep --strict --verbose=2 "$app"
/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$app/Contents/Info.plist" | grep -qx '14.0'
/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist" | grep -qx "$version"
node scripts/smoke-macos-preview.mjs "$binary"
cp .ai-tmp/macos-smoke.json "$output/renderer-smoke.json"
images=("$bundle_root"/dmg/*.dmg)
[[ ${#images[@]} -eq 1 && -f "${images[0]}" ]] || { echo 'Expected exactly one DMG' >&2; exit 1; }
hdiutil verify "${images[0]}"
cp "${images[0]}" "$output/elon-macos-preview-${version}-universal.dmg"
ditto -c -k --sequesterRsrc --keepParent "$app" "$output/elon-macos-preview-${version}-universal.app.zip"
cp docs/requirements/macos-desktop-preview-v1.md "$output/ACCEPTANCE.md"
node --input-type=module <<'JS'
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
const output = process.env.PREVIEW_OUTPUT;
const manifest = {
  schema: 'elon.macos_preview.release.v1',
  version: process.env.PREVIEW_VERSION,
  git_sha: process.env.PREVIEW_GIT_SHA,
  platform: 'macos', architectures: ['arm64', 'x86_64'], minimum_macos: '14.0',
  signing: 'ad-hoc', notarized: false,
  build_and_unit_tests: 'passed', renderer_fixture_smoke: 'passed',
  real_account_acceptance: 'pending', local_node_bundled: false,
  workflow_run: `https://github.com/${process.env.GITHUB_REPOSITORY}/actions/runs/${process.env.GITHUB_RUN_ID}`,
};
fs.writeFileSync(path.join(output, 'release-manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
const notes = `# 一龙 Mac 测试版 ${manifest.version}\n\n` +
  `macOS 14+；一个通用包支持 Apple Silicon（M 系列）和 Intel。\n\n` +
  `源码提交：\`${manifest.git_sha}\`。测试、双架构构建、签名完整性、DMG 检查和真实 WKWebView 本地 fixture 启动/IPC smoke 已通过；厂商账号和用户实机验收待完成。\n\n` +
  `## 安装\n\n下载 universal.dmg，打开后将“一龙工作台 Mac 测试版”拖入 Applications。与 Win 版同样连接现有生产工作台，不包含本地节点服务。\n\n` +
  `本包仅 ad-hoc 签名，未使用 Apple Developer ID、未公证。macOS 可能阻止首次打开。确认来源及 SHA256SUMS.txt 后，按 Apple 官方说明在“系统设置 → 隐私与安全性”使用“仍要打开”；若系统仍拒绝，请反馈原文。不要关闭 Gatekeeper。\n\n` +
  `Apple 说明：https://support.apple.com/zh-cn/102445\n\n` +
  `## 协助测试\n\n1. 记录芯片、macOS 版本、包版本和安装/启动结果。\n2. 测试工作台登录、项目/频道导航、窗口关闭后从菜单栏恢复和退出。\n3. 用户自行操作官网账号，测试 ChatGPT/Google/交易所 WebView、发送/回答、嵌入/弹出、后台及重启恢复。\n4. 测试不同账号/厂商隔离、清理会话和语音/附件，并记录失败步骤和脱敏截图。\n\n` +
  `已补 Mac 会话 DataStore 隔离及系统浏览器跳转；研究 MCP/CDP、DPAPI 快照、会话正文读取、自启动和本地 Windows 节点安装升级未移植。官网兼容、真实登录/语音/附件尚未验收。生产工作台目前沿用 HTTP 地址。请勿提交密码、Token、Cookie 或私有聊天正文。\n\n` +
  `完整范围见 ACCEPTANCE.md；构建证据：${manifest.workflow_run}\n`;
fs.writeFileSync(path.join(output, 'TESTING.md'), notes);
const sums = fs.readdirSync(output).sort().map(name => {
  const hash = crypto.createHash('sha256').update(fs.readFileSync(path.join(output, name))).digest('hex');
  return `${hash}  ${name}`;
});
fs.writeFileSync(path.join(output, 'SHA256SUMS.txt'), sums.join('\n') + '\n');
JS
echo "MACOS_PREVIEW_VERIFIED=true"
echo "MACOS_PREVIEW_OUTPUT=$output"
