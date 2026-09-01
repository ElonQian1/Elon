const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const read = (relative) => fs.readFileSync(path.join(root, relative), 'utf8')

const router = read('server/src/esk_asset/mod.rs')
const serverModel = read('server/src/esk_asset/model.rs')
const serverService = read('server/src/esk_asset/service.rs')
const pcCard = read('pc-frontend/src/features/assets/EskAssetCard.tsx')
const pcApi = read('pc-frontend/src/features/assets/eskAssetApi.ts')
const androidCard = read('android/app/src/main/kotlin/com/elon/app/esk/EskAssetCard.kt')
const androidDialog = read('android/app/src/main/kotlin/com/elon/app/esk/EskSellbackDialog.kt')
const androidApi = read('android/app/src/main/kotlin/com/elon/app/esk/EskAssetApi.kt')

for (const endpoint of [
  '/api/me/assets/esk',
  '/api/me/assets/esk/sellback-requests',
  '/api/admin/assets/esk/paper-allocations',
]) assert.ok(router.includes(endpoint), `missing ESK route: ${endpoint}`)

for (const source of [serverModel, pcApi, androidApi]) {
  assert.ok(source.includes('ESK'), 'ESK identity must be explicit across the contract')
}
for (const source of [serverService, pcApi, androidApi]) {
  assert.ok(source.includes('not_deployed'), 'chain status must remain explicit')
}

for (const source of [pcCard, androidCard]) {
  assert.ok(source.includes('Paper 登记'), 'Paper status must be visible')
  assert.ok(source.includes('尚未上链'), 'not-deployed status must be visible')
  assert.ok(source.includes('未设置官方卖回价格'), 'undefined sellback price must be visible')
  assert.ok(source.includes('申请不代表成交或付款'), 'application-only boundary must be visible')
}

for (const source of [pcCard, androidDialog]) {
  assert.ok(source.includes('暂无卖回申请'), 'empty sellback state must be explicit')
  assert.ok(source.includes('撤销申请'), 'submitted requests must be cancellable')
  assert.ok(source.includes('最多六位小数'), 'exact amount validation must be visible')
}

assert.ok(serverService.includes('ESK_DECIMALS'), 'server must parse exact decimal amounts')
assert.ok(pcApi.includes('CANCEL ESK SELLBACK REQUEST'), 'PC cancellation must use explicit confirmation')
assert.ok(androidApi.includes('CANCEL ESK SELLBACK REQUEST'), 'Android cancellation must use explicit confirmation')

for (const source of [pcCard, pcApi, androidCard, androidDialog, androidApi]) {
  assert.equal(/年化\s*6%|保证收益|固定收益|官方回购价/.test(source), false, 'UI must not promise yield or a fixed official price')
}

console.log('ESK_ASSET_CONTRACT_TEST=passed')
