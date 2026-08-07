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
const pwaController = read('server/src/assets/ai_provider_accounts.js');
const pcAccount = read('pc-frontend/src/features/account/AccountPage.tsx');
const pcCard = read('pc-frontend/src/features/account/ChatGptAccountCard.tsx');
const pcBrowser = read('pc-frontend/src/features/user-browser/LocalAiBrowserPanel.tsx');

expect(androidSideMenu.includes('settingsRow("ChatGPT 账号与聊天")'), 'APK side menu must expose ChatGPT directly');
expect(androidProfile.includes('"ChatGPT 账号与聊天", "登录本人账号"'), 'APK account page must expose ChatGPT login');
expect(androidPage.includes('1  打开 ChatGPT 官方页面'), 'APK page must explain the first login step');
expect(androidPage.includes('只保存在本 APK'), 'APK page must explain local-only session storage');

expect(pwaPage.includes('profile-action-title">ChatGPT 网页账号'), 'PWA profile must expose ChatGPT login');
expect(pwaPage.includes('profile-action-title">AI 厂商账号'), 'PWA must keep advanced provider management separate');
expect(pwaAccountPage.includes('<h1 id="accountTitle">登录自己的 ChatGPT</h1>'), 'PWA account page must provide a user-facing login title');
expect(pwaAccountPage.includes('href="https://chatgpt.com/"'), 'PWA account page must open the official ChatGPT origin');
expect(pwaController.includes("'本人完成登录或真人验证'"), 'PWA panel must explain the official login step');
expect(pwaController.includes("open.textContent = '登录或继续使用 ChatGPT'"), 'PWA must provide an explicit login/use action');

expect(pcAccount.includes('<ChatGptAccountCard />'), 'PC account settings must render the ChatGPT entry card');
expect(pcCard.includes('Cookie 和网页登录数据只保存在这台电脑'), 'PC card must explain local-only session storage');
expect(pcCard.includes('to="/user-browser"'), 'PC account card must link to the ChatGPT workspace');
expect(pcBrowser.includes('登录或打开 ChatGPT'), 'Win workspace must expose a plain-language ChatGPT action');

for (const source of [androidPage, pwaPage, pwaAccountPage, pcCard]) {
  expect(!source.includes('已绑定 ChatGPT'), 'UI must not claim that a local web session is cloud-bound');
}

console.log('Cross-client ChatGPT account entry contract passed.');
