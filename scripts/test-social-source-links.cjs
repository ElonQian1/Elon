// Exercise the real browser contract with synthetic data; no article is fetched.
const assert = require('node:assert/strict')
const fs = require('node:fs')
const vm = require('node:vm')
const path = require('node:path')
const root = path.resolve(__dirname, '..')
const { createRequire } = require('node:module')
const requirePc = createRequire(path.join(root, 'pc-frontend/package.json'))
const ts = requirePc('typescript')
const exportsTs = {}
vm.runInNewContext(ts.transpileModule(fs.readFileSync(path.join(root, 'pc-frontend/src/features/friends/source-links/sourceLink.ts'), 'utf8'), { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText, { exports: exportsTs, URL, TextEncoder })
const context = { URL, TextEncoder }; vm.createContext(context)
vm.runInContext(fs.readFileSync(path.join(root, 'server/src/assets/social_source_links.js'), 'utf8'), context)
const web = context.ElonSourceLinks
for (const raw of ['javascript:alert(1)', 'file:///a', 'intent://a', '/relative', 'https://user:pass@a.test', 'https://a.test/\npath', 'https://a.test\\@b.test', 'https://a.test/' + 'x'.repeat(4096)]) {
  assert.equal(web.webUrl(raw), null, raw); assert.equal(exportsTs.webSourceUrl(raw), null, raw)
}
const url = 'https://mp.weixin.qq.com/s/test?start=1&end=2&scene=90#part'
const source = { version: 1, url, method: 'qr' }
assert.equal(web.webUrl(url), url); assert.equal(exportsTs.webSourceUrl(url), url)
assert.equal(web.source(source), source); assert.equal(exportsTs.sourceLink(source), source)
assert.equal(web.source({ ...source, version: 2 }), null)
assert.equal(exportsTs.sourceLink({ ...source, method: 'untrusted' }), null)
assert.equal(web.source(null), null)
assert.equal(web.label(url), '阅读原文'); assert.equal(exportsTs.sourceLabel(url), '阅读原文')
assert.equal(web.label('https://mp.weixin.qq.com.evil.test/s/test'), '打开链接')
// Forwarding data is ordinary message JSON; metadata is a few bytes beyond its URL.
assert.deepEqual(JSON.parse(JSON.stringify({ attachments: [{ kind: 'image', source_link: source }] })).attachments[0].source_link, source)
assert.ok(Buffer.byteLength(JSON.stringify(source)) < 200)
console.log('PASS: PC/PWA source validation, classification, legacy metadata and forward round-trip')
