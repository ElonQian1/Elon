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

const androidSideMenu = read('android/app/src/main/kotlin/com/elon/app/ChatSideMenuController.kt');
const androidProfile = read('android/app/src/main/kotlin/com/elon/app/PersonalProfileActivity.kt');
const androidPage = read('android/app/src/main/res/layout/activity_ai_provider_accounts.xml');
const pwaPage = read('server/src/assets/web_page.html');
const pwaAccountPage = read('server/src/assets/chatgpt_web_account.html');
const pwaAccountStyles = read('server/src/assets/chatgpt_web_account.css');
const pwaController = read('server/src/assets/ai_provider_accounts.js');
const pcAccount = read('pc-frontend/src/features/account/AccountPage.tsx');
const pcCard = read('pc-frontend/src/features/account/ChatGptAccountCard.tsx');
const pcBrowser = read('pc-frontend/src/features/user-browser/LocalAiBrowserPanel.tsx');
const androidChatKit = read('android/app/src/main/kotlin/com/elon/app/chatkit/OpenAiChatKitActivity.kt');
const pcChatKitCard = read('pc-frontend/src/features/account/OpenAiChatKitCard.tsx');
const pcChatKitPage = read('pc-frontend/src/features/chatkit/OpenAiChatKitPage.tsx');

expect(androidSideMenu.includes('settingsRow("ChatGPT 账号与聊天")'), 'APK side menu must expose ChatGPT directly');
expect(androidProfile.includes('"ChatGPT 账号与聊天", "登录本人账号"'), 'APK account page must expose ChatGPT login');
expect(androidPage.includes('1  打开 ChatGPT 官方页面'), 'APK page must explain the first login step');
expect(androidPage.includes('只保存在本 APK'), 'APK page must explain local-only session storage');
expect(androidPage.includes('aiProviderOpenAiChatKit'), 'APK must expose an OpenAI ChatKit entry');
expect(androidPage.includes('不需要登录 ChatGPT'), 'APK must distinguish ChatKit from ChatGPT login');
expect(androidChatKit.includes('api.createChatKitSession()'), 'APK ChatKit must request a short-lived session natively');
expect(!androidChatKit.includes('Authorization'), 'APK ChatKit page must not receive the Yilong bearer token');

expect(pwaPage.includes('profile-action-title">ChatGPT 网页账号'), 'PWA profile must expose ChatGPT login');
expect(pwaPage.includes('profile-action-subtitle">当前页打开官方网页'), 'PWA profile must explain same-tab ChatGPT navigation');
expect(pwaPage.includes('profile-action-title">AI 厂商账号'), 'PWA must keep advanced provider management separate');
expect(pwaPage.includes('profile-action-title">OpenAI ChatKit（API 聊天）'), 'PWA must expose ChatKit as API chat');
expect(pwaPage.includes('<openai-chatkit id="openAiChatKitElement"'), 'PWA must render the official ChatKit component');
expect(pwaAccountPage.includes('<h1 id="accountTitle">ChatGPT</h1>'), 'PWA account page must make ChatGPT the first-screen product signal');
expect(pwaAccountPage.includes('id="chatGptOfficialLogin"'), 'PWA account page must expose a dedicated login action');
expect(pwaAccountPage.includes('href="https://chatgpt.com/auth/login"'), 'PWA login action must open the official ChatGPT login route');
expect(pwaAccountPage.includes('<span>登录 ChatGPT</span>'), 'PWA account page must use a clear login command');
expect(!pwaAccountPage.includes('target="_blank"'), 'PWA account page must keep ChatGPT in the current tab');
expect(!pwaAccountPage.includes('<iframe'), 'PWA account page must not embed the cross-origin login page');
expect(!pwaAccountPage.includes('type="password"'), 'PWA account page must not collect ChatGPT credentials');
expect(pwaAccountStyles.includes('min-height: 68px'), 'PWA account page must keep the primary login action touch-friendly');
expect(pwaAccountStyles.includes('@media (max-width: 420px)'), 'PWA account page must define a mobile layout');
expect(pwaAccountStyles.includes('@media (prefers-reduced-motion: reduce)'), 'PWA account page must respect reduced-motion preferences');
expect(pwaController.includes("'本人完成登录或真人验证'"), 'PWA panel must explain the official login step');
expect(pwaController.includes("open.textContent = '登录或继续使用 ChatGPT'"), 'PWA must provide an explicit login/use action');

expect(pcAccount.includes('<ChatGptAccountCard />'), 'PC account settings must render the ChatGPT entry card');
expect(pcCard.includes('Cookie 和网页登录数据只保存在这台电脑'), 'PC card must explain local-only session storage');
expect(pcCard.includes('to="/user-browser"'), 'PC account card must link to the ChatGPT workspace');
expect(pcBrowser.includes('登录 ChatGPT 并打开聊天窗'), 'Win workspace must expose a plain-language ChatGPT action');
expect(pcAccount.includes('<OpenAiChatKitCard />'), 'PC account settings must render the ChatKit entry card');
expect(pcChatKitCard.includes('不需要再次登录 ChatGPT'), 'PC card must distinguish ChatKit from ChatGPT account login');
expect(pcChatKitPage.includes("'/api/openai-chatkit/session'"), 'PC ChatKit must use the authenticated session endpoint');
expect(pcChatKitPage.includes('不读取 ChatGPT Cookie、历史或 Plus 额度'), 'PC ChatKit must show the account boundary');

for (const source of [androidPage, pwaPage, pwaAccountPage, pcCard]) {
  expect(!source.includes('已绑定 ChatGPT'), 'UI must not claim that a local web session is cloud-bound');
}

console.log('Cross-client ChatGPT account entry contract passed.');
