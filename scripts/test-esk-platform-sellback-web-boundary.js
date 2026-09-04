const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const web = fs.readFileSync(path.join(root, 'server/src/assets/web_page.html'), 'utf8')
const entry = web.match(/<section\b[^>]*\bid="profileEskPlatformEntry"[^>]*>([\s\S]*?)<\/section>/)
assert.ok(entry, 'Keep the existing formal platform entry separate from the Paper card')
const content = entry[1]

for (const label of [
  '正式 ESK 平台登记', '查看完整审核流水', '卖回申请与占用',
  '正式总量、申请占用与可申请量', '新申请生产默认关闭', '独立政策与条款',
  '申请不代表报价、成交或付款', '本网页暂不读取正式私有余额',
  '不提供正式卖回申请', 'Paper 模拟资产不包含正式登记数量',
  '可申请量不是现金或即时兑付承诺', '当前 HTTP 不可用',
  '新版子 APK 尚待正式签名、上传与双 APK 联调',
]) assert.ok(content.includes(label), `Missing truthful boundary: ${label}`)

assert.match(content, /href="\/app\/ElonSpeed-latest\.apk"/)
assert.doesNotMatch(content, /<(?:form|input|button|script)\b/i,
  'This entry describes native functionality; it must not impersonate an authorized web write surface')
assert.doesNotMatch(content, /\bon(?:click|submit)\s*=/i)
assert.ok(web.indexOf('id="profileEskPlatformEntry"') > web.indexOf('id="profileEskAssetCard"'))

console.log('ESK formal sellback Web/native explanatory boundary passed (no private web writes)')
