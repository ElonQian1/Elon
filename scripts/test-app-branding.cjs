const { readFileSync } = require('node:fs');
const { resolve } = require('node:path');
const { runInNewContext } = require('node:vm');
const assert = require('node:assert/strict');
const { test } = require('node:test');
const read = file => readFileSync(resolve(__dirname, '..', file), 'utf8');
const branding = JSON.parse(read('server/src/assets/app_branding.json'));

test('PWA keeps shared downloads and its requested home-screen display name', () => {
  const page = read('server/src/assets/web_page.html');
  const apkUrl = page.match(/function apkUrl\(\)\s*\{[^}]+\}/)[0];
  assert.equal(runInNewContext(`${apkUrl}; apkUrl()`, { location: { origin: 'https://example.test' } }),
    `https://example.test/app/${branding.apkFileName}`);
  assert.ok(page.includes(`href="/app/${branding.apkFileName}"`));
  assert.ok(page.includes('<title>一龙ai</title>'));
  assert.ok(page.includes('name="apple-mobile-web-app-title" content="一龙ai"'));
  assert.ok(page.includes('rel="apple-touch-icon" sizes="180x180" href="/app/icon-180.png"'));
  assert.ok(read('server/src/assets/download_page.html').includes(`<h1>${branding.displayName} APK 下载</h1>`));
  assert.ok(!page.includes(branding.legacyApkFileName));
});

test('Android uses shared public branding while preserving installed identity', () => {
  const gradle = read('android/app/build.gradle');
  assert.match(gradle, /applicationId "com\.elon\.app"/);
  assert.match(gradle, /resValue "string", "app_name", appBranding\.displayName/);
  assert.match(gradle, /"APP_APK_DOWNLOAD_PATH".*appBranding\.apkFileName/);
  assert.match(read('android/app/src/main/kotlin/com/elon/app/MainActivity.kt'), /\$serverUrl\$\{BuildConfig\.APP_APK_DOWNLOAD_PATH\}/);
});
