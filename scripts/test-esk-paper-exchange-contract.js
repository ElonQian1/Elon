const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const read = (relative) => fs.readFileSync(path.join(root, relative), 'utf8')

const routes = read('server/src/esk_exchange/mod.rs')
const api = read('server/src/esk_exchange/api.rs')
const model = read('server/src/esk_exchange/model.rs')
const quote = read('server/src/esk_exchange/quote.rs')
const store = read('server/src/store/common/esk_exchange.rs')
const migration = read('server/src/esk_exchange_migration.rs')
const pcApi = read('pc-frontend/src/features/assets/eskAssetApi.ts')
const pcPanel = read('pc-frontend/src/features/assets/EskPaperExchangePanel.tsx')
const androidApi = read('android/app/src/main/kotlin/com/elon/app/esk/EskAssetApi.kt')
const androidDialog = read('android/app/src/main/kotlin/com/elon/app/esk/EskPaperExchangeDialog.kt')
const pwa = read('server/src/assets/esk_exchange.js')
const webPage = read('server/src/assets/web_page.html')
const webRouter = read('server/src/router.rs')

for (const endpoint of [
  '/api/me/assets/esk/exchange-account',
  '/api/me/assets/esk/exchange-quotes',
  '/api/me/assets/esk/exchanges',
  '/api/admin/assets/usdt/paper-credits',
]) assert.ok(routes.includes(endpoint), `missing exchange endpoint: ${endpoint}`)

for (const key of [
  'ESK_PAPER_EXCHANGE_MODE',
  'ESK_PAPER_USDT_PER_ESK',
  'ESK_PAPER_EXCHANGE_FEE_BPS',
]) assert.ok(model.includes(key), `missing fail-closed configuration: ${key}`)

assert.ok(model.includes('Some("paper")'), 'Paper must be the only enabled exchange mode')
assert.ok(model.includes('Some(_) => Self::Invalid'), 'unknown exchange modes must fail closed')
assert.equal(model.includes('Some("live")'), false, 'server must not expose a live-funds mode')
assert.ok(model.includes('CONFIRM PAPER ESK USDT EXCHANGE'), 'execution needs explicit confirmation')
assert.ok(model.includes('RECORD PAPER USDT CREDIT'), 'Paper USDT credit needs explicit confirmation')

assert.ok(quote.includes('checked_mul'), 'quote math must be overflow checked')
assert.ok(quote.includes('/ 10_000'), 'fee must use integer basis-point math')
assert.ok(quote.includes('checked_add(9_999)'), 'fee must round upward')
assert.equal(/f32|f64/.test(quote), false, 'quote math must not use floating point')

for (const source of [api, pcApi, androidApi, pwa]) {
  for (const field of ['simulated', 'funds_moved', 'on_chain_settlement', 'trading_mode']) {
    assert.ok(source.includes(field), `safe envelope field missing from a client/server surface: ${field}`)
  }
}

for (const schema of [
  'yilong.esk.paper_exchange_account.v1',
  'yilong.esk.paper_exchange_quote.v1',
  'yilong.esk.paper_exchange_execution.v1',
]) {
  assert.ok(api.includes(schema), `server response schema missing: ${schema}`)
  assert.ok(pcApi.includes(schema), `PC schema missing: ${schema}`)
  assert.ok(androidApi.includes(schema), `Android schema missing: ${schema}`)
  assert.ok(pwa.includes(schema), `PWA schema missing: ${schema}`)
}

assert.ok(store.includes('TransactionBehavior::Immediate'), 'exchange settlement must serialize balance checks')
assert.ok(store.includes('quote.config_revision != input.config_revision'), 'execution must reject stale pricing configuration')
for (const posting of [
  'exchange_user_debit',
  'exchange_market_credit',
  'exchange_market_debit',
  'exchange_user_credit',
  'platform_fee',
]) assert.ok(store.includes(posting), `balanced posting missing: ${posting}`)

for (const table of ['esk_exchange_quotes', 'esk_exchange_executions', 'esk_exchange_ledger_entries']) {
  assert.ok(migration.includes(`BEFORE UPDATE ON ${table}`), `${table} must reject updates`)
  assert.ok(migration.includes(`BEFORE DELETE ON ${table}`), `${table} must reject deletes`)
}

for (const source of [pcPanel, androidDialog, pwa]) {
  for (const boundary of ['Paper', '手续费', '未上链', '不移动真实资金']) {
    assert.ok(source.includes(boundary), `user-visible Paper boundary missing: ${boundary}`)
  }
  assert.ok(source.includes('60 秒'), 'quote expiry must be visible')
  assert.ok(source.includes('确认 Paper 模拟兑换'), 'UI must use a second explicit confirmation step')
  assert.equal(/年化\s*6%|保证收益|固定收益|保本/.test(source), false, 'exchange UI must not promise yield')
}

assert.ok(webPage.includes('/assets/esk_exchange.css'), 'mobile page must load exchange styles')
assert.ok(webPage.includes('/assets/esk_exchange.js'), 'mobile page must load exchange behavior')
assert.ok(webRouter.includes('/assets/esk_exchange.css'), 'server must publish exchange styles')
assert.ok(webRouter.includes('/assets/esk_exchange.js'), 'server must publish exchange behavior')

console.log('ESK_PAPER_EXCHANGE_CONTRACT_TEST=passed')
