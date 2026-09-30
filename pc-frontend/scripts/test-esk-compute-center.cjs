const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const ts = require('typescript')
const root = path.resolve(__dirname,'../..')
function load(file,stub) {
  const source = fs.readFileSync(path.join(root,file),'utf8')
  const code = ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText
  const module = {exports:{}}
  new Function('exports','module','require',code)(module.exports,module,stub || require)
  return module.exports
}
const model = load('pc-frontend/src/features/billing/eskComputeCenterModel.ts')
const fixture = () => JSON.parse(fs.readFileSync(path.join(root,'contracts/esk/compute-center-v1.fixture.json'),'utf8'))
const value = model.parseSnapshot(fixture(),2000)
assert.equal(model.formatUnits(value.asset.total_base_units),'80.000000')
assert.equal(model.formatUnits(value.billing.balance_fen,2),'12.34')
assert.equal(model.formatUnits('-1',2),'-0.01')
assert.equal(model.formatUnits('9223372036854775807'),'9,223,372,036,854.775807')
for (const mutate of [
  v => { v.asset.remaining_base_units = '79000000' },
  v => { v.valuation.cny_base_units = '584000001' },
  v => { v.quote.usdt_per_esk_base_units = '0' },
  v => { v.quote.observed_at_ms = 1001 },
  v => { v.quote.valid_until_ms = 301001 },
  v => { v.billing.currency = 'ESK' },
  v => { v.capabilities.esk_service_spending = true },
  v => { v.usage_sources.push(v.usage_sources[0]) },
  v => { v.usage_sources[0].total_tokens = '9223372036854775808' },
  v => { v.billing.holds[0].status = 'settled' },
]) { const v=fixture(); mutate(v); assert.throws(() => model.parseSnapshot(v,2000)) }
assert.throws(() => model.parseSnapshot(fixture(),61000))
const unavailable = fixture(); unavailable.quote=null; unavailable.valuation=null; unavailable.quote_status='not_configured'
assert.equal(model.parseSnapshot(unavailable,2000).valuation,null)
let credentialsRead = false
global.location = {href:'http://unknown.example/pc/'}
const api = load('pc-frontend/src/features/billing/eskComputeCenterApi.ts',name => {
  if(name.includes('runtime'))return {resolveApiUrl:p=>`http://unknown.example${p}`}
  if(name.includes('Model'))return model
  throw Error(`unexpected import ${name}`)
})
api.readCenter(1,()=>{credentialsRead=true;return 'synthetic'},new AbortController().signal).then(
  () => assert.fail('Expected unsafe source rejection'),
  () => { assert.equal(credentialsRead,false); console.log('ESK_COMPUTE_PC_CONTRACT=passed cases=17') },
)
