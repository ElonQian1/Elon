const fs = require('node:fs')
const vm = require('node:vm')
const path = require('node:path')
const assert = require('node:assert/strict')
const ts = require('../../pc-frontend/node_modules/typescript')
const root = path.resolve(__dirname, '../..')
const channels = { URL }
vm.runInNewContext(fs.readFileSync(path.join(root, 'server/src/assets/social_links.js'), 'utf8'), channels)
const code = ts.transpileModule(fs.readFileSync(path.join(root, 'pc-frontend/src/features/friends/wechatCardAction.ts'), 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const url = 'https://weixin.qq.com/sph/example'
const receipt = () => ({ schema: 1, source_url: 'https://channels.weixin.qq.com/finder-preview/pages/sph?id=example',
  launch_url: 'weixin://biz/finder/openFinderFeed/exportId%3Dexport%2Fabcdefgh12345678%26actionType%3D0', expires_at_ms: Date.now() + 60000 })
function load(result, beforeReturn = () => {}, desktop = true) {
  const effects = []
  const exports = {}
  vm.runInNewContext(code, { exports, Date, Number, Error, ElonSocialLinks: channels.ElonSocialLinks,
    require(name) {
      if (name === '../shell/desktopShell') return { getDesktopInvoke: () => desktop ? async (command, args) => effects.push({ command, args }) : null }
      if (name === './socialReadBack') return { previewApi: async (endpoint, request) => {
        effects.push({ endpoint, request }); beforeReturn(); return result
      } }
      throw new Error('Unexpected dependency: ' + name)
    },
  })
  return { open: exports.openWechatCard, effects }
}
;(async () => {
  const signal = new AbortController().signal
  const valid = load(receipt()); await valid.open(url, signal)
  assert.equal(valid.effects.length, 2)
  assert.equal(valid.effects[0].endpoint, '/api/me/link-preview/wechat-open')
  assert.equal(valid.effects[0].request.body, JSON.stringify({ url }))
  assert.equal(valid.effects[1].command, 'open_wechat_feed_url')
  assert.equal(valid.effects[1].args.sourceUrl, receipt().source_url)
  for (const patch of [{ schema: 2 }, { source_url: 'https://weixin.qq.com/sph/other' }, { launch_url: 'file:///C:/test' }, { expires_at_ms: Date.now() - 1 }]) {
    const invalid = load({ ...receipt(), ...patch }); await assert.rejects(invalid.open(url, signal))
    assert.equal(invalid.effects.length, 1)
  }
  const controller = new AbortController()
  const canceled = load(receipt(), () => controller.abort()); await canceled.open(url, controller.signal)
  assert.equal(canceled.effects.length, 1)
  const browser = load(receipt(), undefined, false); await assert.rejects(browser.open(url, signal)); assert.equal(browser.effects.length, 0)
  const unrelated = load(receipt()); await assert.rejects(unrelated.open('https://example.com/', signal)); assert.equal(unrelated.effects.length, 0)
  console.log('PASS Win card action: fixed endpoint, native dispatch, receipt identity, expiry, cancellation and browser boundary')
})().catch(error => { console.error(error); process.exitCode = 1 })
