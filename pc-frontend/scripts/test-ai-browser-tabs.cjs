const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..', '..')

const host = read('desktop-shell/src-tauri/src/internal_browser.rs')
const embedded = read('desktop-shell/src-tauri/src/local_ai_browser/embedded_view.rs')
const localBrowser = read('desktop-shell/src-tauri/src/local_ai_browser.rs')
const semanticBridge = read('desktop-shell/src-tauri/src/codex_semantic_bridge.rs')
const main = read('desktop-shell/src-tauri/src/main.rs')
const permission = read('desktop-shell/src-tauri/permissions/local-ai-web-session.toml')
const api = read('pc-frontend/src/features/user-browser/internalBrowserApi.ts')
const experience = read('pc-frontend/src/features/user-browser/AiBrowserExperience.tsx')
const experienceStyles = read('pc-frontend/src/features/user-browser/AiBrowserExperience.module.css')
const sidebar = read('pc-frontend/src/features/user-browser/AiWebChatSidebar.tsx')
const message = read('pc-frontend/src/features/ai/AiChatMessageRow.tsx')
const sourceLinks = read('pc-frontend/src/features/ai/AiSourceLinks.tsx')
const topbar = read('pc-frontend/src/features/ai/AiChatTopbar.tsx')
const chat = read('pc-frontend/src/features/ai/AiChatPage.tsx')
const controller = read('pc-frontend/src/features/user-browser/useLocalAiWebChatController.ts')
const sendDispatcher = read('pc-frontend/src/features/user-browser/dispatchPreparedLocalAiPrompt.ts')
const sendSuccessBranch = sendDispatcher.slice(sendDispatcher.indexOf('export async function'))
const nativeCommandBranch = localBrowser.slice(
  localBrowser.indexOf('pub async fn run_local_ai_web_adapter_command'),
  localBrowser.indexOf('pub async fn open_local_ai_cached_conversation'),
)

assert.match(host, /WebviewBuilder::new/)
assert.match(host, /\.incognito\(true\)/)
assert.match(host, /validate_external_url/)
assert.match(host, /\.on_navigation/)
assert.match(host, /NewWindowResponse::Deny/)
assert.doesNotMatch(host, /additional_browser_args\(WEBVIEW2_BROWSER_ARGS\)/)
assert.doesNotMatch(host, /cookies\(|document\.cookie|Authorization/i)
assert.doesNotMatch(localBrowser, /webview:\s*WebviewWindow/)
assert.match(localBrowser, /WebviewBuilder::new/)
assert.match(localBrowser, /WindowBuilder::new/)
assert.match(localBrowser, /main_window\s*\.add_child/)
assert.ok(
  (nativeCommandBranch.match(/embedded_view::hide\(&app, &label\)/g) || []).length >= 2,
  'the Win shell must park the provider page before and after native send evaluation',
)
assert.ok(
  nativeCommandBranch.indexOf('embedded_view::hide(&app, &label)') < nativeCommandBranch.indexOf('page.eval('),
)
assert.doesNotMatch(localBrowser, /WebviewWindowBuilder::new/)
assert.doesNotMatch(embedded, /webview:\s*WebviewWindow/)
assert.doesNotMatch(semanticBridge, /(?:window|webview):\s*WebviewWindow/)
assert.doesNotMatch(main, /get_webview_window\(MAIN_WINDOW_LABEL\)/)
assert.match(main, /get_window\(MAIN_WINDOW_LABEL\)/)
assert.match(embedded, /\.reparent\(/)
assert.match(embedded, /\.set_position\(/)
assert.match(embedded, /\.set_size\(/)
assert.match(embedded, /\.hide\(\)/)
assert.match(embedded, /\.show\(\)/)
assert.match(
  embedded,
  /if webview\.window\(\)\.label\(\) == MAIN_WINDOW_LABEL \{[\s\S]*?park\(&webview\)\?/,
)
const hideBranch = embedded.slice(embedded.indexOf('pub(crate) fn hide('), embedded.indexOf('pub(crate) fn park_if_background'))
assert.doesNotMatch(hideBranch, /\.reparent\(/, 'hiding an active provider page must not synchronously reparent WebView2')
assert.match(embedded, /const PARK_OFFSET: i32 = 20_000/)
assert.match(embedded, /present_local_ai_web_session_embedded/)
assert.match(embedded, /hide_local_ai_web_session_embedded/)
assert.match(
  embedded,
  /present\(&app, &label, bounds\)\?[\s\S]*?reconnect_adapter\(provider, &page\)/,
  'showing an embedded official page must resume the adapter and request a fresh snapshot',
)
assert.match(
  localBrowser,
  /session_control::apply[\s\S]*?if action == "restore"[\s\S]*?reconnect_adapter\(provider, &page\)/,
  'restoring an official popout must resume the adapter and request a fresh snapshot',
)
assert.match(main, /internal_browser::open_internal_browser_tab/)
assert.match(main, /local_ai_browser::embedded_view::present_local_ai_web_session_embedded/)
assert.match(permission, /open_internal_browser_tab/)
assert.match(permission, /present_local_ai_web_session_embedded/)

assert.match(api, /open_internal_browser_tab/)
assert.match(api, /control_internal_browser_tab/)
assert.match(api, /present_local_ai_web_session_embedded/)
assert.match(experience, /聊天/)
assert.match(experience, /官方页/)
assert.match(experience, /系统浏览器/)
assert.match(experience, /ResizeObserver/)
assert.match(experience, /requestAnimationFrame/)
assert.match(experience, /官网原生内容/)
assert.match(experience, /页面加载失败/)
assert.match(experience, /createPortal/)
assert.match(experience, /data-ai-surface="production-home"/)
assert.match(experienceStyles, /\.surface\s*{[^}]*inset:\s*0;/s)
assert.match(experience, /className={styles\.tabStrip}/)
assert.match(experience, /className={styles\.navigationBar}/)
assert.match(experience, /className={styles\.siteIdentity}/)
assert.match(experience, /className={styles\.viewport}/)
assert.match(experienceStyles, /grid-template-rows:\s*38px 40px minmax\(0, 1fr\)/)
assert.match(experience, /await hideLocalAiWebSessionEmbedded/)
assert.match(
  experience,
  /await presentLocalAiWebSessionEmbedded\(official, bounds\)[\s\S]*?if \(generation !== generationRef\.current\) \{[\s\S]*?await hideOfficialSurface\(official\)/,
)
assert.doesNotMatch(experience, /contentOnly/)
assert.doesNotMatch(api, /contentOnly/)
assert.doesNotMatch(embedded, /content_only|answer_surface/)
assert.match(experience, /if \(next === 'chat'\) \{[\s\S]*?generationRef\.current \+= 1[\s\S]*?activateSurface\('chat'\)/)
assert.match(experience, /announceAiBrowserSurface\(next\)[\s\S]*?setSurface\(next\)/)
assert.match(experience, /windowVisible/)
assert.match(experience, /event\.key === 'Escape'/)
assert.match(api, /REQUEST_RETURN_TO_AI_CHAT_EVENT/)
assert.match(api, /CustomEvent<OfficialAiTabRequest \| undefined>/)
assert.match(api, /officialSurfaceRequestedVisible = false/)
assert.match(api, /officialSurfaceIntentVersion === version/)
assert.match(sidebar, /requestReturnToAiChat/)
assert.doesNotMatch(sidebar, /controller\.control\('background'\)/)
assert.doesNotMatch(
  sidebar,
  /requestOfficialAiTab/,
  'background official WebView visibility must not be promoted into a foreground tab request',
)
assert.match(message, /AiSourceLinks/)
assert.match(sourceLinks, /openInternalBrowserLink/)
assert.match(sourceLinks, /isLocalAiBrowserAvailable/)
assert.match(sourceLinks, /使用系统浏览器打开/)
assert.match(topbar, /onOpenOfficial/)
assert.match(chat, /AiBrowserExperience/)
assert.match(chat, /data-ai-chat-main/)
assert.match(chat, /chatMode && web\.ready/)
assert.doesNotMatch(chat, /AiOfficialAnswerSurface/)
assert.match(sendSuccessBranch, /onResponseRefresh/)
assert.doesNotMatch(sendSuccessBranch, /requestOfficialAiTab|showOfficialAfterSend|openOfficial/)
assert.doesNotMatch(controller, /showOfficialAfterSend/)
assert.match(controller, /dispatchPreparedLocalAiPrompt/)
assert.match(sendDispatcher, /requestReturnToAiChat/)
assert.match(sendDispatcher, /controlLocalAiWebSession\(provider\.id, ownerKey, 'background'\)/)
assert.ok(
  (sendDispatcher.match(/controlLocalAiWebSession\(provider\.id, ownerKey, 'background'\)/g) || []).length >= 2,
  'native send must park the official page before dispatch and after its matching receipt',
)
assert.match(sendDispatcher, /消息已交给官方网页发送；正在一龙聊天界面同步回复/)

process.stdout.write('PASS Win AI internal browser tab contract\n')

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8')
}
