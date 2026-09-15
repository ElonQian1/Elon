'use strict';
// Execute existing fake-transport lifecycle cases with an official Chinese contract identity.
// Every request stays in the harness; no browser session or exchange request is opened.
const fs=require('node:fs');
const path=require('node:path');
const Module=require('node:module');
const assert=require('node:assert/strict');
const pattern=/^[A-Z0-9\u3400-\u4DBF\u4E00-\u9FFF]{1,24}USDT$/;
for(const base of ['龙虾','币安人生','我踏马来了','牛来','哈基米','NEAR','1000SHIB'])assert.ok(pattern.test(base+'USDT'));
for(const value of ['龙虾USDT&symbol=BTCUSDT','龙虾 USDT','龙虾\u200bUSDT','龙'.repeat(25)+'USDT'])assert.ok(!pattern.test(value));
for(const file of ['test-binance-grid-read-adapter.cjs','test-binance-grid-reports.cjs','test-binance-grid-create-adapter.cjs','test-binance-grid-manage-adapter.cjs']) {
  const filename=path.join(__dirname,file);
  const source=fs.readFileSync(filename,'utf8');
  assert.ok(source.includes('NEARUSDT'),file+' must exercise a contract');
  const child=new Module(filename,module);child.filename=filename;child.paths=module.paths;
  child._compile(source.replaceAll('NEARUSDT','龙虾USDT'),filename);
}
