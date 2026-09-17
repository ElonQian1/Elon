import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import assert from 'node:assert/strict';
const context = vm.createContext({ URL, TextEncoder, AbortController, setTimeout, clearTimeout, queueMicrotask, console });
const base = new URL('../server/src/assets/', import.meta.url);
vm.runInContext(readFileSync(new URL('social_links.js', base), 'utf8'), context);
vm.runInContext(readFileSync(new URL('social_link_viewer.js', base), 'utf8'), context);
const links = context.ElonSocialLinks;
const example = '【高糖VS戒糖14天！真的差别很大吗？】 【精准空降到 00:02】 https://www.bilibili.com/video/BV1enYL6SEtU/?share_source=copy_web&t=2&p=3';
assert.equal(links.links(example)[0].embed.url, 'https://player.bilibili.com/player.html?bvid=BV1enYL6SEtU&autoplay=0&poster=1&t=2&p=3');
assert.equal(links.links(example)[0].title, '高糖VS戒糖14天！真的差别很大吗？');
const xhs = 'https://www.xiaohongshu.com/discovery/item/example?xsec_token=synthetic&xsec_source=pc_share';
assert.equal(links.links(xhs)[0].url, xhs);
assert.equal(links.links(xhs + ' ' + xhs).length, 1);
const xhsShare = `45 【一次看完多位画师卡位实拍，哪张击中你？✨ - OC猫 | 小红书 - 你的生活兴趣社区】 😆 vEtl5AZaiF1Cpm2 😆 ${xhs}`;
const sharedXhs = links.links(xhsShare)[0];
assert.equal(sharedXhs.title, '一次看完多位画师卡位实拍，哪张击中你？✨');
assert.equal(sharedXhs.author, 'OC猫');
assert.equal(links.presentation(sharedXhs).source, '小红书 · OC猫');
assert.equal(links.presentation(links.links(example)[0]).time, '从 00:02 开始');
assert.equal(links.presentation(links.links('https://www.bilibili.com/video/BV1enYL6SEtU/?t=3680')[0]).time, '从 1:01:20 开始');
const binance = 'https://app.binance.com/uni-qr/cpos/123456?r=synthetic&l=zh-CN';
assert.equal(links.links(binance)[0].url, binance);
assert.equal(links.compact(binance), true);
assert.equal(links.presentation(links.links(binance)[0]).title, '币安广场帖子');
assert.equal(links.links('https://app.binance.com.evil.example/a')[0].site, 'app.binance.com.evil.example', 'look-alike hosts are plain generic pages');
assert.equal(links.presentation(links.links('https://www.example.org/post/1')[0]).title, '网页链接');
assert.equal(links.presentation(links.links('https://www.example.org/post/1')[0]).source, 'example.org');
assert.equal(links.presentation(links.links('https://www.example.org/post/1')[0]).badge, '↗');
assert.equal(links.links('https://10.0.0.1/a').length, 0, 'IP literals never become cards');
assert.equal(links.sanitize({ schema: 1, url: 'https://www.example.org/post/1', title: 'example.org', status: 'ready' }, links.links('https://www.example.org/post/1')[0]).status, 'unavailable', 'host-only titles are not real previews');
assert.equal(links.presentation(links.links('https://x.com/example/status/123456')[0]).source, 'X · @example');
assert.equal(links.presentation(links.links('https://x.com/i/status/123456')[0]).source, 'X');
assert.equal(links.sanitize({ ...sharedXhs, title: '小红书 - 你的生活兴趣社区', author: '', status: 'ready' }, sharedXhs).title, sharedXhs.title);
assert.equal(links.compact(xhsShare), true);
assert.equal(links.compact(`我的评论 ${xhsShare}`), false);
assert.equal(links.compact(`${xhsShare} 我的评论`), false);
assert.equal(links.compact('3.53 复制打开抖音，看看【作者的作品】《世界》 https://v.douyin.com/_XMEsxVKKOY/ aNW:/ i@p.QX :9pm 02/07'), true);
const douyinShare = links.links('3.53 复制打开抖音，看看【作者的作品】《世界》第一季合集 https://v.douyin.com/fixture/ aNW:/ i@p.QX :9pm 02/07')[0];
assert.equal(douyinShare.title, '《世界》第一季合集'); assert.equal(douyinShare.author, '作者');
for (const bad of ['https://user:pass@x.com/a', 'javascript:alert(1)', '【一龙文章】\nhttps://x.com/a/status/123456']) assert.equal(links.links(bad).length, 0);
assert.equal(links.links('https://x.com.evil.example/a/status/123456')[0].embed, null, 'look-alike hosts never get an official player');
assert.equal(links.embed('https://x.com/i/article/123456'), null);
assert.equal(links.embed('https://www.binance.com/en/square/post/123456'), null);
const x = links.embed('https://twitter.com/Interior/status/463440424141459456');
assert.equal(x.kind, 'x');
assert.equal(context.ElonSocialLinkViewer.frameSource({ ...x, id: '<script>alert(1)</script>' }), null);
assert.match(context.ElonSocialLinkViewer.frameSource(x).srcdoc, /platform\.x\.com\/widgets\.js/);

// Exercise async mount against both PWA Response and PC decoded-JSON API contracts.
class Node {
  constructor(tag) { this.tag = tag; this.children = []; this.parentNode = null; this.hidden = false; this.style = {}; }
  append(...items) { for (const item of items) { item.parentNode = this; this.children.push(item); } }
  remove() { if (this.parentNode) this.parentNode.children = this.parentNode.children.filter(n => n !== this); this.parentNode = null; }
  setAttribute(key, value) { this[key] = value; }
  removeAttribute(key) { delete this[key]; }
  showModal() { this.open = true; }
  close() { this.open = false; this.onclose?.(); }
  focus() {}
}
context.document = { createElement: tag => new Node(tag), body: new Node('body') };
vm.runInContext(readFileSync(new URL('social_source_links.js', base), 'utf8'), context);
const sourceHost = new Node('host');
context.ElonSourceLinks.text(sourceHost, 'https://www.bilibili.com/video/BV1enYL6SEtU/?t=2');
assert.equal(sourceHost.children.length, 0, 'rich links do not get a second source card');
context.ElonSourceLinks.text(sourceHost, 'https://example.org/article');
assert.equal(sourceHost.children.length, 0, 'generic https pages are now rich cards, not a second source entry');
context.ElonSourceLinks.text(sourceHost, 'http://example.org/article');
assert.equal(sourceHost.children.length, 1, 'plain http links retain their existing source entry');
context.ElonSocialLinkViewer.open({ url: 'https://x.com/Interior/status/463440424141459456', site: 'X', embed: x });
const xFrame = context.document.body.children[0].children.find(n => n.tag === 'iframe');
assert.ok(!xFrame.sandbox.includes('allow-same-origin'), 'srcdoc scripts must not inherit the privileged application origin');
context.ElonSocialLinkViewer.close();
const flush = () => new Promise(resolve => setTimeout(resolve, 10));
for (const responseStyle of [true, false]) {
  const host = new Node('host'); let requests = 0; let opened;
  const item = links.links(example)[0];
  const result = { ...item, title: '<script>safe text</script>', status: 'ready', image: null };
  const cleanup = links.mount(host, example, { owner: 'user-' + responseStyle, api: async () => { requests++; return responseStyle ? { ok: true, json: async () => result } : result; }, open: p => { opened = p; } });
  const card = host.children[0].children[0].children[0];
  assert.ok(card.children[0].children[0].textContent, 'fallback is synchronous');
  await flush();
  assert.equal(requests, 1); assert.equal(card.children[0].children[0].textContent, '<script>safe text</script>');
  card.onclick({ preventDefault() {} }); assert.equal(opened.url, item.url);
  cleanup(); assert.equal(host.children.length, 0);
}
let resolveLate;
const host = new Node('host');
const cleanup = links.mount(host, example, { owner: 'late', api: () => new Promise(resolve => { resolveLate = resolve; }) });
await flush(); const detached = host.children[0].children[0].children[0];
const initial = detached.children[0].children[0].textContent; cleanup();
resolveLate({ ...links.links(example)[0], title: 'late result', status: 'ready' }); await flush();
assert.equal(detached.children[0].children[0].textContent, initial);
const failed = new Node('host');
links.mount(failed, example, { owner: 'failed', api: async () => { throw new Error('offline'); } }); await flush();
assert.equal(failed.children[0].children[0].children[1].hidden, false, 'failure offers retry without blocking open');
let finishOldRequest;
const readHost = new Node('host');
const disposeRead = links.mount(readHost, binance, { owner: 'read-account', api: () => new Promise(resolve => { finishOldRequest = resolve; }) });
await flush();
const observed = { ...links.links(binance)[0], title: 'Original-page title', status: 'ready' };
links.remember('another-account', observed);
assert.notEqual(readHost.children[0].children[0].children[0].children[0].children[0].textContent, observed.title);
links.remember('read-account', observed);
finishOldRequest({ ...observed, title: 'Stale server result' }); await flush();
assert.equal(readHost.children[0].children[0].children[0].children[0].children[0].textContent, observed.title, 'native read wins over a late server response');
links.remember('read-account', { ...observed, title: 'Expired read' }, Date.now() - 1);
assert.equal(readHost.children[0].children[0].children[0].children[0].children[0].textContent, observed.title);
disposeRead(); links.remember('read-account', { ...observed, title: 'After disposal' });
assert.equal(readHost.children.length, 0);
// Description, server-copied covers and member read-back marker.
const wechat = links.links('https://mp.weixin.qq.com/s/test')[0];
const inline = 'data:image/jpeg;base64,/9j/4AAQSkZJRg==';
const rawRich = { ...wechat, title: '标题', description: '摘要', status: 'ready', source: 'member', image: 'https://mmbiz.qpic.cn/a.jpg', cover_data_url: inline };
const rich = links.sanitize(rawRich, wechat);
assert.equal(rich.image, inline); assert.equal(rich.description, '摘要'); assert.equal(rich.source, 'member');
assert.equal(links.presentation(rich).source, '微信公众号 · 成员回填'); assert.equal(links.presentation(rich).summary, '摘要');
assert.equal(links.sanitize({ ...rawRich, cover_data_url: 'data:text/html;base64,PGI+' }, wechat).image, 'https://mmbiz.qpic.cn/a.jpg');
assert.equal(links.sanitize({ ...rawRich, source: 'anything' }, wechat).source, 'server');
const richHost = new Node('host');
links.mount(richHost, wechat.url, { owner: 'rich', api: async () => ({ ...rich }) }); await flush();
const richCopy = richHost.children[0].children[0].children[0].children[0];
assert.equal(richCopy.children[1].textContent, '摘要'); assert.equal(richCopy.children[1].hidden, false);
const compactHost = new Node('host');
links.mount(compactHost, wechat.url, { owner: 'rich-compact', compact: true, api: async () => ({ ...rich }) }); await flush();
assert.equal(compactHost.children[0].children[0].children[0].children[0].children[1].hidden, true, 'compact cards keep summaries hidden');
console.log('PASS: six-platform URL policy, Bilibili time/page, X isolated embed, both API contracts, stale callbacks, failure recovery, summaries and member read-back');
