import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const url = 'https://weixin.qq.com/sph/Aur6t4pfk3';
const scene = () => ({ schema: 1, source_url: url, expires_at_ms: Date.now() + 60000,
  launch_url: 'weixin://biz/finder/openFinderFeed/' + encodeURIComponent('exportId=export/abcdefgh123&actionType=0&entryScene=1') });
class Node {
  constructor() { this.children = []; this.hidden = false; }
  append(...items) { this.children.push(...items); }
  setAttribute() {}
  remove() {}
}
const listeners = new Map(), navigations = [];
const document = { hidden: false, createElement: () => new Node(), addEventListener: (k, f) => listeners.set(k, f), removeEventListener: k => listeners.delete(k) };
const context = vm.createContext({ URL, URLSearchParams, AbortController, setTimeout, clearTimeout, console, document,
  navigator: { userAgent: 'Android Chrome', userActivation: { isActive: true }, clipboard: { writeText: async () => {} } },
  location: { assign: value => navigations.push(value) },
  addEventListener: (k, f) => listeners.set(k, f), removeEventListener: k => listeners.delete(k) });
for (const name of ['social_links', 'social_wechat_handoff']) vm.runInContext(readFileSync(new URL(`../server/src/assets/${name}.js`, import.meta.url), 'utf8'), context);
const handoff = context.ElonWechatHandoff;
assert.match(handoff.intent(scene(), url), /^intent:\/\/biz\/finder\/openFinderFeed\/.*#Intent;scheme=weixin;package=com\.tencent\.mm;end$/);
for (const changed of [
  { source_url: 'https://weixin.qq.com/sph/other' }, { source_url: 'https://weixin.qq.com.evil.test/sph/Aur6t4pfk3' },
  { expires_at_ms: Date.now() }, { schema: 2 }, { launch_url: 'javascript:alert(1)' },
  { launch_url: scene().launch_url + '#Intent;component=other;end' },
  { launch_url: scene().launch_url + '%26exportId%3Dexport%2Fduplicate' },
  { launch_url: scene().launch_url + '%26browser_fallback_url%3Dhttps%3A%2F%2Fevil.test' },
]) assert.equal(handoff.intent({ ...scene(), ...changed }, url), null);

let current = true;
function create(api) {
  const host = new Node();
  const opener = handoff.create(host, () => ({ url }), { api, isCurrent: () => current });
  return { opener, status: () => host.children[0].children[0].textContent, controls: host.children[0].children };
}
let calls = 0;
let test = create(async (_path, options) => { calls++; assert.equal(JSON.parse(options.body).url, url); return { ok: true, json: async () => scene() }; });
await test.opener.open();
assert.equal(calls, 1); assert.equal(navigations.length, 1); assert.equal(test.status(), '已请求打开微信');
assert.ok(!navigations[0].includes('browser_fallback_url'), 'failed app launch must not reload the PWA');
test.opener.dispose();

context.navigator.userActivation.isActive = false;
test = create(async () => scene()); await test.opener.open();
assert.equal(navigations.length, 1); assert.match(test.status(), /已准备好/);
context.navigator.userActivation.isActive = true; await test.controls[1].onclick();
assert.equal(navigations.length, 2); test.opener.dispose();

for (const action of ['cancel', 'hidden', 'dispose', 'account']) {
  let resolve; test = create(() => new Promise(r => { resolve = r; }));
  const job = test.opener.open(); await Promise.resolve(); await test.opener.open();
  if (action === 'cancel') test.controls[3].onclick();
  if (action === 'hidden') { document.hidden = true; listeners.get('visibilitychange')(); }
  if (action === 'dispose') test.opener.dispose();
  if (action === 'account') current = false;
  resolve(scene()); await job;
  assert.equal(navigations.length, 2, `late response cannot launch after ${action}`);
  test.opener.dispose(); document.hidden = false; current = true;
}
test = create(async () => { throw new Error('offline'); }); await test.opener.open();
assert.match(test.status(), /暂时无法/); assert.equal(test.controls[1].disabled, false);
test.opener.dispose();
context.navigator.userAgent = 'Windows';
assert.equal(handoff.create(new Node(), () => ({ url }), {}), null, 'desktop keeps its existing native handoff');
assert.equal(listeners.size, 0);
console.log('PASS: PWA scene validation, direct Intent, fresh click, single-flight, cancel/hide/account/dispose, retry and cleanup');
