const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),ts=require('typescript');
const base=path.resolve(__dirname,'../src/features'),cache=new Map();
function load(file){file=path.resolve(base,file);if(!file.endsWith('.ts'))file+='.ts';if(cache.has(file))return cache.get(file);const m={exports:{}};cache.set(file,m.exports);const code=ts.transpileModule(fs.readFileSync(file,'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;new Function('require','module','exports',code)(name=>name.startsWith('.')?load(path.resolve(path.dirname(file),name)):require(name),m,m.exports);return m.exports;}
const model=load('grid-history/gridHistoryModel'),chat=load('grid-chat/gridChatSnapshot'),share=load('grid-share/gridShareModel');
cache.set(path.resolve(base,'exchange-webview/exchangeWebviewApi.ts'),{});
const {readHistoryPage}=load('grid-history/readGridHistory');
const now=Date.now(),source={document:'private-document',account:'private-account',accountKind:'sub'};
const row={id:'123',symbol:'龙虾USDT',status:'CANCELED',direction:'SHORT',leverage:'4',count:'169',lower:'0.1',upper:'0.2',profit:'479.92684725',fee:'6.2',fundingFee:'-2.7',investment:'1000.000000000001',created:String(now-86400000),end:String(now-3600000),secret:'must-not-escape',totalPnl:'999999',positionQty:'123'};
test('Chinese history preserves source precision without inventing settlement, positions or total PnL',()=>{
 const f=model.historyFacts(row);assert.equal(f.symbol,'龙虾USDT');assert.equal(f.profit,row.profit);assert.equal(f.investment,row.investment);assert.equal(f.settlement,'UNKNOWN');assert.equal(f.positionState,'UNKNOWN');assert.equal(f.totalPnl,undefined);assert.equal(f.positionQty,undefined);assert.equal(f.secret,undefined);
});
test('invalid symbols, running records and inverted time ranges fail closed',()=>{
 for(const change of [{symbol:'龙虾USDT\n'},{symbol:'../龙虾USDT'},{status:'WORKING'},{end:String(now-999999999)},{end:String(now+100000)}])assert.throws(()=>model.historyFacts({...row,...change}));
});
test('zero and missing remain distinct and final time can remain unknown',()=>{
 const f=model.historyFacts({...row,profit:'0',end:null,fee:null});assert.equal(f.profit,'0');assert.equal(f.end,null);assert.equal(f.fee,null);
});
test('historical draft survives live TTL, preserves chat isolation and states evidence limits',()=>{
 const f=model.historyFacts(row),selection={source,rows:[f],observedAtMs:now-86400000,recordKind:'HISTORY'};
 const a=model.historicalAttachment(selection,'123','chat-one'),prompt=chat.gridPrompt('复盘这笔网格',a,'chat-one',now);
 assert.ok(prompt.includes('龙虾USDT'));assert.ok(prompt.includes('不能据此推算最大回撤'));assert.ok(prompt.includes('手续费是否已含'));assert.ok(!prompt.includes('private-account'));
 assert.throws(()=>chat.gridPrompt('复盘',a,'chat-two',now));assert.throws(()=>model.historicalAttachment(selection,'999','chat-one'));
 assert.throws(()=>chat.gridPrompt('复盘',{...a,facts:{...f,recordKind:null}},'chat-one',now));
});
test('public history uses an immutable historical title and excludes source identities',()=>{
 const a={source,chatScope:'chat',observedAtMs:now,facts:model.historyFacts(row)},g=share.publicGrid(a),d=share.shareDocument(g);
 assert.equal(d.title,'龙虾USDT 历史网格');assert.equal(g.fields.end,row.end);assert.equal(g.fields.recordKind,'HISTORY');assert.equal(g.fields.id,undefined);assert.ok(!JSON.stringify(g).includes('private-'));assert.equal(share.value(g,'settlement'),'未确认');
 const hidden=share.publicGrid(a,false);assert.equal(hidden.fields.profit,undefined);assert.equal(share.value(hidden,'profit'),'未公开');
});
test('history quotation separates positive grid profit from a negative total and honors legacy privacy',()=>{
 const {quoteSummary}=load('friends/quotes/socialQuote'),a={source,chatScope:'chat',observedAtMs:now,facts:model.historyFacts(row)};
 function quote(extra,visible=true){const grid=share.publicGrid(a,visible);Object.assign(grid.fields,extra);const card={...share.shareDocument(grid),snapshot_id:'ai_snapshot_fixture',group_id:'group_fixture'};return quoteSummary({content:'【一龙AI对话】\n'+JSON.stringify(card),attachments:[],unavailable:false});}
 assert.match(quote({}),/龙虾USDT.*网格利润（非总盈亏）/);
 assert.match(quote({totalPnl:'-23.50'}),/总盈亏.*23.5/);
 assert.match(quote({totalPnl:'0'}),/总盈亏 0/);
 assert.match(quote({},false),/未公开/);
});
function historyPort(change=()=>{},never=false){
 let clock=Date.now(),query,reads=0,commands=0;
 const identity={account:'101',account_kind:'sub'},state={windowOpen:true,adapterReady:true,documentToken:'doc',identity,reports:{}};
 const expected={document:'doc',account:'101',accountKind:'sub'};
 return {expected,get commands(){return commands;},port:{get:async()=>{reads++;if(query&&!never){state.reports[query.request]={schema:'yilong.binance_report_observation.v1',request:query.request,kind:'history',account:'101',account_kind:'sub',page:query.page,status:'ready',coverage:'page',total:21,rows:[row]};change(state,query);}return state;},now:()=>clock,sleep:async()=>{clock+=400;}},dispatch:async raw=>{query=JSON.parse(raw);commands++;assert.deepEqual(Object.keys(query).sort(),['days','id','kind','page','request','symbol']);assert.equal(query.id,'');assert.equal(query.symbol,'');}};
}
test('explicit historical page reads preserve Chinese and bind exact requested page/source',async()=>{
 const h=historyPort(),selection=await readHistoryPage(h.port,h.dispatch,30,2,()=>true,h.expected);
 assert.equal(selection.page,2);assert.equal(selection.total,21);assert.equal(selection.rows[0].symbol,'龙虾USDT');assert.equal(h.commands,1);
});
test('history rejects changed source, mismatched page, duplicate strategies and unavailable reports',async()=>{
 for(const mutate of [(s)=>s.identity.account='102',(s,q)=>s.reports[q.request].page=2,(s,q)=>s.reports[q.request].rows=[row,row],(s,q)=>s.reports[q.request].status='error']){
  const h=historyPort(mutate);await assert.rejects(readHistoryPage(h.port,h.dispatch,30,1,()=>true));
 }
 const stale=historyPort();await assert.rejects(readHistoryPage(stale.port,stale.dispatch,30,2,()=>true,{...stale.expected,document:'old'}));assert.equal(stale.commands,0);
});
test('empty history is a successful bounded result while cancellation and timeout do not attach',async()=>{
 const empty=historyPort((s,q)=>Object.assign(s.reports[q.request],{rows:[],total:0}));assert.equal((await readHistoryPage(empty.port,empty.dispatch,7,1,()=>true)).rows.length,0);
 const canceled=historyPort();await assert.rejects(readHistoryPage(canceled.port,canceled.dispatch,7,1,()=>false));assert.equal(canceled.commands,0);
 const timeout=historyPort(()=>{},true);await assert.rejects(readHistoryPage(timeout.port,timeout.dispatch,7,1,()=>true),/超时/);assert.equal(timeout.commands,1);
});
