'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const web = fs.readFileSync(path.join(root, 'server/src/assets/web_page.html'), 'utf8');
const entry = web.match(/<section\b[^>]*\bid="profileEskPlatformEntry"[^>]*>([\s\S]*?)<\/section>/);
assert.ok(entry, 'FORMAL_ENTRY_REQUIRED');
for (const label of [
  '正式 ESK · 占用与卖回进度', '仅供原生 APK', '每页需在主项目重新确认账户',
  '只读临时进度', '不会在此网页发起卖回或付款', '可申请量不是现金或即时兑付承诺',
  '不会自动绑定网页账户', '当前 HTTP 不可用', 'Paper 模拟资产不包含正式登记数量',
]) assert.ok(entry[1].includes(label), `NATIVE_PROGRESS_BOUNDARY_REQUIRED: ${label}`);
assert.doesNotMatch(entry[1], /<(?:script|form|input|button)\b|\bon(?:click|submit)\s*=/i);
assert.doesNotMatch(web, /READ_ESK_PLATFORM_PROGRESS|yilong\.esk\.platform_android_progress\.v1/,
  'Browser must not imitate the OS-authenticated native progress exchange');
assert.match(entry[1], /href="\/app\/ElonSpeed-latest\.apk"/);
console.log('ESK formal progress explanatory Web/native boundary passed (no browser IPC or private writes)');
