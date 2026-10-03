const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const ts = require('typescript')
const source = fs.readFileSync(path.join(__dirname, '../src/features/conversation/projectWebEntry.ts'), 'utf8')
const api = {}, opened = [], external = []
let desktop = true, full = false
vm.runInNewContext(ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText, {
  exports: api, URL,
  window: { open: (...args) => external.push(args) },
  require: name => name.includes('desktopShell')
    ? { getDesktopInvoke: () => desktop ? () => {} : null }
    : { useReaderTabs: { getState: () => ({ open: (preview, scope) => { opened.push({ preview, scope }); return full ? null : { id: 'web' } } }) } },
})
for (const invalid of [null, '', 'javascript:alert(1)', 'http://shop.test/', '//shop.test/', 'https://user:pass@shop.test/', 'https://shop.test/\n', 'https://shop.test/\\other', 'https://shop.test/' + 'x'.repeat(2048)]) {
  assert.equal(api.projectWebUrl(invalid), null)
  assert.throws(() => api.openProjectWeb(invalid, '商店', 'shop'))
}
assert.equal(api.projectWebUrl('https://shop.test'), 'https://shop.test/')
api.openProjectWeb('https://shop.test/', '商店', 'shop')
assert.equal(opened.length, 1)
assert.equal(opened[0].preview.url, 'https://shop.test/')
assert.equal(opened[0].preview.title, '商店')
assert.equal(opened[0].scope, 'project:shop')
assert.equal(external.length, 0, 'desktop must stay in the internal webview')
full = true
assert.throws(() => api.openProjectWeb('https://shop.test/', '商店', 'shop'), /标签已满/)
desktop = false
api.openProjectWeb('https://shop.test/', '商店', 'shop')
assert.deepEqual(external[0], ['https://shop.test/', '_blank', 'noopener,noreferrer'])
const catalog = JSON.parse(fs.readFileSync(path.join(__dirname, '../../server/src/official_project_catalog/catalog.json'), 'utf8'))
assert.equal(catalog.projects.find(p => p.id === 'cofficethinking').landing.web_url, 'https://182.254.168.75/')
console.log('Project web entry: native routing, browser fallback, invalid URLs and tab limit passed')
