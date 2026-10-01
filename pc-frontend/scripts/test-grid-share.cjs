const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs'), ts = require('typescript'), vm = require('node:vm'), path = require('node:path')
const source = fs.readFileSync(path.join(__dirname, '../src/features/grid-share/gridShareModel.ts'), 'utf8')
const moduleValue = { exports: {} }
vm.runInNewContext(ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText, { exports: moduleValue.exports, module: moduleValue })
const m = moduleValue.exports
const snapshot = { observedAtMs: 1000, source: { account: 'private-account', document: 'private-proof' }, facts: { id: '123', symbol: 'TESTUSDT', account: 'private-account', cookie: 'secret', lower: '0.1', upper: '0.2', direction: 'SHORT', leverage: '4', count: '60', profit: '1.2300', investment: '1000', perGridQty: '4' } }
test('legacy hidden projection omits quantities, money, session proof and private strategy IDs', () => {
  const grid = m.publicGrid(snapshot, false)
  for (const key of ['id', 'account', 'cookie', 'investment', 'profit', 'perGridQty']) assert.equal(key in grid.fields, false)
  assert.equal(grid.note, undefined); assert.equal(m.value(grid, 'profit'), '未公开'); assert.equal(m.value(grid, 'markPrice'), '未读取')
  assert.equal(JSON.stringify(grid).includes('private-'), false)
})
test('default public amounts retain precision and canonical cross-platform container', () => {
  const current = m.publicGrid(snapshot)
  assert.equal(current.show_amounts, true); assert.equal(current.fields.investment, '1000'); assert.equal(current.fields.perGridQty, '4')
  assert.equal(current.fields.profit, '1.2300'); assert.equal(JSON.stringify(current).includes('private-'), false); assert.equal(current.fields.cookie, undefined)
  const grid = m.publicGrid(snapshot, true, '', 'ai_snapshot_first'), doc = m.shareDocument(grid)
  assert.equal(grid.fields.profit, '1.2300'); assert.equal(grid.fields.positionQty, undefined)
  assert.equal(doc.summary, '做空 · 4× · 0.1–0.2 · 60 格 · 历史快照')
  assert.equal(doc.title, 'TESTUSDT 网格快照'); assert.equal(grid.previous_snapshot_id, 'ai_snapshot_first')
  assert.equal(grid.fields.id, undefined)
})
test('card parser rejects malformed data and keeps original version identity', () => {
  assert.equal(m.gridCard(m.SHARE_PREFIX + '{}'), null)
  const grid = m.publicGrid(snapshot), content = m.SHARE_PREFIX + JSON.stringify({ schema: m.SHARE_SCHEMA, provider: 'binance', snapshot_id: 'ai_snapshot_one', group_id: 'group_one', grid })
  assert.equal(m.gridCard(content).snapshot_id, 'ai_snapshot_one')
})
test('position report keeps exact values but never substitutes another symbol or combines hedge rows', () => {
  const output = { exports: {} }
  const code = fs.readFileSync(path.join(__dirname, '../src/features/grid-share/readSharePositions.ts'), 'utf8')
  vm.runInNewContext(ts.transpileModule(code, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText,
    { exports: output.exports, module: output, require: () => ({ object: value => value ?? {} }) })
  const report = { status: 'ready', coverage: 'strategy_position', rows: [{ symbol: 'TESTUSDT', quantity: '-10.00', entry: '0.12', liquidation: '0', mark: '0.13', accountMarginBalance: 'SECRET' }] }
  const result = output.exports.projectPosition(report, 'TESTUSDT')
  assert.equal(result.positionQty, '-10.00'); assert.equal(result.liquidationPrice, undefined); assert.equal(JSON.stringify(result).includes('SECRET'), false)
  assert.throws(() => output.exports.projectPosition(report, 'OTHERUSDT'))
  assert.throws(() => output.exports.projectPosition({ ...report, rows: [...report.rows, ...report.rows] }, 'TESTUSDT'))
})
