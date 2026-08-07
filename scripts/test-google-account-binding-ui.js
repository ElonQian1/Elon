'use strict';

const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const read = (relative) => fs.readFileSync(path.join(root, relative), 'utf8');
const expect = (condition, message) => {
  if (!condition) throw new Error(message);
};

const profile = read('android/app/src/main/kotlin/com/elon/app/MainProfileQuickActions.kt');
const profileEntry = read('android/app/src/main/kotlin/com/elon/app/ProfileAccountSecurityEntry.kt');
const presentation = read('android/app/src/main/kotlin/com/elon/app/AccountIdentityPresentation.kt');
const activity = read('android/app/src/main/kotlin/com/elon/app/AccountIdentityActivity.kt');
const layout = read('android/app/src/main/res/layout/activity_account_identities.xml');
const googleAuth = read('android/app/src/main/kotlin/com/elon/app/GoogleFederatedAuth.kt');
const web = read('server/src/assets/web_page.html');
const webAuth = read('server/src/assets/federated_auth.js');
const webCss = read('server/src/assets/federated_auth.css');

expect(profile.includes('ProfileAccountSecurityEntry(activity, binding)'), 'APK profile must assemble account security entry');
expect(profileEntry.includes('binding.profilePrimaryActionsCard'), 'APK entry must live on the primary profile card');
expect(profileEntry.includes('AccountIdentityActivity::class.java'), 'APK entry must open account identity management');
expect(profileEntry.includes('Google 未绑定') || presentation.includes('Google 未绑定'), 'APK profile must expose Google binding state');
expect(presentation.includes('156****92') || presentation.includes('takeLast(2)'), 'APK must mask the current phone account');
expect(activity.includes('继续使用 Google'), 'APK must confirm the target account before binding');
expect(layout.includes('@+id/accountCurrentAccount'), 'APK account screen must show the current Yilong account');
expect(layout.includes('@+id/accountGoogleBindingSummary'), 'APK account screen must show Google binding state');
expect(googleAuth.includes('google_oidc_not_configured'), 'APK must explain missing Google OIDC configuration');
expect(!googleAuth.includes('GoogleIdTokenCredential.createFrom(credential.data).idToken.also'), 'APK must not persist the Google ID token');

['accountIdentitiesSubtitle', 'accountIdentityCurrentAccount', 'accountGoogleBindingState', 'federatedBindGoogle']
  .forEach((id) => expect(web.includes(`id="${id}"`), `PWA is missing #${id}`));
expect(web.includes('不会保存 Google 密码或访问令牌'), 'PWA must state the credential boundary');
expect(webAuth.includes('https://accounts.google.com/gsi/client'), 'PWA must use Google Identity Services');
expect(webAuth.includes('provider.configured'), 'PWA must render the server configuration state');
expect(webAuth.includes('Google 登录尚未配置，暂时无法绑定'), 'PWA must explain unavailable production configuration');
expect(webAuth.includes('identity_owned_by_another_account'), 'PWA must explain identity ownership conflicts');
expect(webAuth.includes("subtitle.textContent = 'Google 已绑定"), 'PWA profile must summarize bound Google identity');
expect(webCss.includes('.identity-account-card') && webCss.includes('.identity-state.bound'), 'PWA must style account and binding state cards');

console.log('Google account binding APK/PWA UI contract passed.');
