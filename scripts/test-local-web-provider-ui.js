'use strict';

const fs = require('fs');
const path = require('path');
const repoRoot = path.resolve(__dirname, '..');

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

function expect(condition, message) {
  if (!condition) throw new Error(message);
}

const registry = read('server/src/assets/local_web_providers.js');
const pwa = read('server/src/assets/web_page.html');
const pwaController = read('server/src/assets/ai_provider_accounts.js');
const pwaAccountPage = read('server/src/assets/chatgpt_web_account.html');
const router = read('server/src/router.rs');
const androidAdapter = read('android/app/src/main/assets/chatgpt_web_adapter.js');
const androidBridge = read('android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt');

expect(registry.includes("id: 'chatgpt_web'"), 'PWA registry must include ChatGPT Web');
expect(registry.includes("officialUrl: 'https://chatgpt.com/'"), 'ChatGPT Web must use the official HTTPS origin');
expect(registry.includes("semanticProtocol: 'yilong.ai.ui.v1'"), 'provider registry must share the cross-client semantic protocol');
expect(registry.includes('nativeProjectionInPwa: false'), 'PWA must not claim cross-origin native projection');
expect(registry.includes('nativeProjectionInApk: true'), 'registry must expose the APK enhancement capability');
expect(!registry.includes('document.cookie'), 'PWA registry must not read browser cookies');
expect(!registry.includes('fetch('), 'PWA web-provider registry must not proxy provider traffic');
expect(pwa.includes('id="localWebProviderList"'), 'PWA provider panel must expose the local web-provider list');
expect(pwa.includes('profile-action-title">ChatGPT 网页账号'), 'PWA profile must expose the ChatGPT account entry');
expect(pwa.includes('profile-action-title">AI 厂商账号'), 'PWA must keep advanced provider management separate');
expect(pwaController.includes('renderLocalWebProviders()'), 'PWA provider panel must render registered web providers');
expect(pwaController.includes("open.textContent = '登录或继续使用 ChatGPT'"), 'PWA must label the official login and chat action clearly');
expect(pwa.includes('id="chatGptWebAccountRow"'), 'PWA profile must expose a direct ChatGPT account entry');
expect(pwaController.includes("window.location.assign('/chatgpt-web')"), 'PWA direct entry must open the dedicated account page');
expect(pwaController.includes('window.location.assign(provider.officialUrl)'), 'PWA must open ChatGPT in the current tab');
expect(router.includes('"/chatgpt-web", get(web::chatgpt_web_account_page)'), 'server must expose a stable ChatGPT account URL');
expect(pwaAccountPage.includes('href="https://chatgpt.com/"'), 'account page must link to the official ChatGPT origin');
expect(!pwaAccountPage.includes('target="_blank"'), 'account page must keep ChatGPT navigation in the current tab');
expect(!pwaAccountPage.includes('document.cookie'), 'account page must not read ChatGPT cookies');

['document.cookie', 'fetch(', 'XMLHttpRequest', 'WebSocket', 'Authorization'].forEach((forbidden) => {
  expect(!androidAdapter.includes(forbidden), `Android page adapter must not contain ${forbidden}`);
});
expect(androidAdapter.includes('new MutationObserver'), 'Android adapter must observe visible page semantics');
expect(androidAdapter.includes("schema: 'yilong.ai.ui.v1'"), 'Android adapter must emit the shared native UI protocol');
expect(androidAdapter.includes("action === 'send_prompt'"), 'Android adapter must support the typed send action');
expect(androidBridge.includes('WebViewCompat.addWebMessageListener'), 'Android must use an origin-scoped WebMessage listener');
expect(androidBridge.includes('const val ALLOWED_ORIGIN = "https://chatgpt.com"'), 'Android bridge must pin the ChatGPT origin');
expect(!androidBridge.includes('addJavascriptInterface'), 'Android must not expose a broad JavaScript interface');
expect(!androidBridge.includes('getCookie('), 'Android bridge must not export WebView cookies');

console.log('Local web provider APK/PWA contract passed.');
