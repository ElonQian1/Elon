const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const test = require('node:test');
const assert = require('node:assert/strict');
const code = fs.readFileSync(path.join(__dirname, '../android/app/src/main/assets/binance_grid_reports_adapter.js'), 'utf8');
const tick = () => new Promise(resolve => setImmediate(resolve));
function fixture() {
  const events = [], calls = [], data = [], diagnostics = [];
  const account = {account:'42', account_kind:'sub', headers:new Headers({'x-fixture-secret':'local-only'})};
  const known = new Set(['123']);
  const window = {__elonBinanceDiagnosticsV1:{report:value=>diagnostics.push(value)}}; window.top = window;
  vm.runInNewContext(code, {window,location:{origin:'https://www.binance.com'},Headers,AbortController,setTimeout,clearTimeout});
  let prove = async () => ({account:account.account,account_kind:account.account_kind});
  const reports = window.__elonBinanceReportsFactoryV1({context:()=>account,known:id=>known.has(id),prove:headers=>prove(headers),emit:event=>events.push(event),
    fetch:async (url, init) => {
      calls.push({url,init}); const reply = data.shift();
      if(reply instanceof Error) throw reply;
      return {status:200,text:async()=>JSON.stringify({code:'000000',success:true,...reply})};
    }});
  const q = (kind='history', overrides={}) => ({request:'a'.repeat(32),kind,id:kind==='history'?'':'123',symbol:kind==='history'?'':'NEARUSDT',page:1,days:30,...overrides});
  return {events,calls,data,account,known,reports,q,diagnostics,setProof:f=>{prove=f;}};
}
const grid = {strategyId:'123',symbol:'NEARUSDT',rootUserId:'42',strategyUserId:'99',strategyStatus:'WORKING'};
test('history uses the actual v2 POST and never forwards credential or unrelated fields', async()=>{
  const h=fixture(); h.data.push({data:{grids:[{...grid,gridProfit:'0.1234567890123456789',email:'private@example.test'}],total:1}});
  assert.equal(h.reports.query(h.q()),true); await tick();
  assert.equal(h.calls[0].url,'/bapi/futures/v2/private/future/grid/query-grid-history');
  const input=JSON.parse(h.calls[0].init.body); assert.equal(input.rows,20);assert.equal(input.page,1);
  assert.equal(input.endTime-input.startTime,30*86400000);
  assert.equal(h.events[0].rows[0].profit,'0.1234567890123456789');
  assert.ok(!JSON.stringify(h.events).includes('local-only'));assert.ok(!JSON.stringify(h.events).includes('private@example'));
});
test('unknown strategy, invented route, or excessive pagination never dispatch',()=>{
  const h=fixture();
  for(const q of [h.q('orders',{id:'456'}),h.q('close'),h.q('history',{url:'/close-grid'}),h.q('history',{page:1001}),h.q('history',{days:365})]) assert.equal(h.reports.query(q),false);
  assert.equal(h.calls.length,0);
});
test('shadow-account positions use strategyUserId from verified detail, not the logged-in UID',async()=>{
  const h=fixture();h.data.push({data:grid},{data:{'42':[{symbol:'NEARUSDT',positionSide:'BOTH',positionAmount:'999'}],'99':[{symbol:'NEARUSDT',positionSide:'BOTH',positionAmount:'-1.234',entryPrice:'2',isolatedWallet:'3',isolated:true}]}});
  h.reports.query(h.q('positions'));await tick();assert.equal(h.events[0].status,'ready');
  assert.equal(h.events[0].rows[0].quantity,'-1.234');assert.equal(h.calls[1].init.body,'{}');
});
test('normal open orders filter the exact strategy and preserve decimal quantities',async()=>{
  const h=fixture();h.data.push({data:{...grid,slideWindow:false}},{data:[{strategyId:'321',symbol:'BTCUSDT',side:'SELL',price:'100'},
    {strategyId:'123',symbol:'NEARUSDT',side:'BUY',price:'1.1',origQty:'1.000000000000000001',executedQty:'0',status:'NEW'}]});
  h.reports.query(h.q('orders'));await tick();assert.equal(h.events[0].rows.length,1);assert.equal(h.events[0].coverage,'open_orders');
  assert.equal(h.events[0].rows[0].quantity,'1.000000000000000001');
});
test('window slots retain pending status instead of pretending they are live orders',async()=>{
  const h=fixture();h.data.push({data:{...grid,slideWindow:true}},{data:{bidItems:[{price:'1',qty:'2',status:'PENDING'}],askItems:[]}});
  h.reports.query(h.q('orders'));await tick();assert.equal(h.events[0].coverage,'grid_slots');assert.equal(h.events[0].rows[0].status,'PENDING');
});
test('matched data uses top-level total and retains raw fees and both deal sides',async()=>{
  const h=fixture();h.data.push({data:grid},{total:21,data:[{matchedSeq:1,profit:'0.01',profitAsset:'USDT',completionTime:1788890000000,
    details:[{side:'BUY',avgPrice:'1',executedQty:'2',feeAmount:'-0.0001',feeAsset:'USDT'},{side:'SELL',avgPrice:'2'}]}]});
  h.reports.query(h.q('matches',{page:2}));await tick();assert.equal(h.events[0].total,21);assert.equal(h.events[0].rows[0].details[0].fee,'-0.0001');
  assert.deepEqual(JSON.parse(h.calls[1].init.body),{strategyId:'123',page:2,rows:20});
});
test('a different detail symbol or account cannot be used as a strategy scope',async()=>{
  const h=fixture();h.data.push({data:{...grid,symbol:'BTCUSDT'}});h.reports.query(h.q('orders'));await tick();
  assert.equal(h.events[0].status,'error');assert.equal(h.calls.length,1);
});
test('account change or reset discards late report data',async()=>{
  const h=fixture();let release;h.setProof(()=>new Promise(resolve=>{release=resolve;}));
  h.reports.query(h.q());h.reports.reset();h.account.account='43';release({account:'42',account_kind:'sub'});await tick();
  assert.equal(h.events.length,0);assert.equal(h.calls.length,0);
});
test('business errors never become a successful empty history',async()=>{
  const h=fixture();h.data.push({success:false,data:{grids:[],total:0}});h.reports.query(h.q());await tick();assert.equal(h.events[0].status,'error');
});
test('history pagination keeps one time window and resets it with account context',async()=>{
  const h=fixture();assert.equal(h.reports.query(h.q('history',{page:2})),false);
  h.data.push({data:{grids:[],total:40}});h.reports.query(h.q());await tick();
  h.data.push({data:{grids:[],total:40}});h.reports.query(h.q('history',{page:2,request:'b'.repeat(32)}));await tick();
  const first=JSON.parse(h.calls[0].init.body), second=JSON.parse(h.calls[1].init.body);
  assert.equal(first.startTime,second.startTime);assert.equal(first.endTime,second.endTime);assert.equal(second.page,2);
  h.reports.reset();assert.equal(h.reports.query(h.q('history',{page:2})),false);
});

test('optional upstream display drift keeps the verified page and preserves unknowns',async()=>{
  const h=fixture();h.data.push({data:{...grid,slideWindow:true}},{data:{bidItems:[{price:1.25,qty:'2',status:{newField:true},insertTime:''}],askItems:[]}});
  h.reports.query(h.q('orders'));await tick();
  const e=h.events.at(-1);assert.equal(e.status,'ready');assert.equal(e.rows.length,1);
  assert.equal(e.rows[0].price,null);assert.equal(e.rows[0].status,null);assert.equal(e.rows[0].time,null);
  assert.equal(e.rows[0].quantity,'2');assert.equal(h.diagnostics.at(-1).outcome,'ready');
});
test('unscoped source rows cannot be mistaken for a verified empty strategy',async()=>{
  const h=fixture();h.data.push({data:grid},{data:[{symbol:'BTCUSDT',side:'BUY'},
    {strategyId:'123',symbol:'NEARUSDT',side:'SELL',origQty:'3'}]});
  h.reports.query(h.q('orders'));await tick();assert.equal(h.events.at(-1).status,'error');assert.equal(h.events.at(-1).rows.length,0);
});
test('required side still rejects malformed records and exposes only fixed parse stage',async()=>{
  const h=fixture();h.data.push({data:grid},{data:[{strategyId:'123',symbol:'NEARUSDT',side:'private-canary'}]});
  h.reports.query(h.q('orders'));await tick();assert.equal(h.events.at(-1).status,'error');
  assert.equal(h.diagnostics.at(-1).stage,'parse_orders');assert.equal(h.diagnostics.at(-1).error,'unsupported_field');
  assert.ok(!JSON.stringify(h.diagnostics).includes('private-canary'));
});
test('business failure records endpoint and bounded code without a fake empty success',async()=>{
  const h=fixture();h.data.push({data:grid},{success:false,code:'123456',message:'private-canary'});
  h.reports.query(h.q('positions'));await tick();assert.equal(h.events.at(-1).status,'error');
  assert.deepEqual(JSON.parse(JSON.stringify(h.diagnostics.at(-1))),{kind:'positions',stage:'positions',outcome:'failed',error:'business_failed',http:200,business:'123456'});
  assert.ok(!JSON.stringify(h.diagnostics).includes('private-canary'));
});
