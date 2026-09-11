const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const window={};
vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../android/app/src/main/assets/binance_grid_trailing_rules.js'),'utf8'),{window});
const api=window.__elonBinanceTrailingRulesV1;
const fixture=JSON.parse(fs.readFileSync(path.join(__dirname,'fixtures/binance-grid-trailing-vectors.json'),'utf8'));
test('288 independent captured-oracle vectors cover quantity, cap and composed editor range',()=>{
  assert.equal(fixture.vectors.length,288);
  for(const v of fixture.vectors)assert.equal(JSON.stringify(api[v.operation](...v.args)),JSON.stringify(v.expected),JSON.stringify(v));
});
const cases=[
  {args:['10','10','0.01','10','5','ARITH',2,'100000'],expected:'1000'},
  {args:['10','10','0.01','10','5','GEO',2,'100000'],expected:'970.05860256'},
  {args:['3','33','1','0.004573','0.003343','ARITH',6,'1'],expected:'0.99997845'},
  {args:['3','33','1','0.004573','0.003343','GEO',6,'1'],expected:'0.99550233'}
];
test('matches captured official 15323.W arithmetic and geometric vectors',()=>{
  for(const c of cases)assert.equal(api.cap(...c.args),c.expected,JSON.stringify(c.args));
});
test('price filter validation is exact at both grid bounds and exchange bounds',()=>{
  const current={up:true,down:true,lower:'5',upper:'10'};
  const rules={minimum:'0.01',maximum:'1000',tick:'0.01'};
  assert.equal(api.validate(current,{up_price:'1000',down_price:'0.01'},rules),true);
  for(const draft of [
    {up_price:'10',down_price:'1'}, {up_price:'1000.01',down_price:'1'},
    {up_price:'11',down_price:'5'}, {up_price:'11',down_price:'0.009'},
    {up_price:'11.001',down_price:'1'}, {up_price:'1.1e1',down_price:'1'},
    {up_price:11,down_price:'1'}, {up_price:'11',down_price:'1',extra:true}
  ])assert.equal(api.validate(current,draft,rules),false);
  assert.equal(api.validate({up:true,down:false,lower:'1',upper:'999999999999999999999999999998'},
    {up_price:'999999999999999999999999999999.00000000000000000001',down_price:''},
    {minimum:'0.01',maximum:'999999999999999999999999999999.01',tick:'0.01'}),false);
});
test('inactive directions are not silently edited and unavailable rules do not permit a draft',()=>{
  const current={up:false,down:true,lower:'5',upper:'10'};
  const rules={minimum:'0.01',maximum:'1000',tick:'0.01'};
  assert.equal(api.validate(current,{up_price:'',down_price:'1'},rules),true);
  assert.equal(api.validate(current,{up_price:'11',down_price:'1'},rules),false);
  for(const edit of [{minimum:''},{tick:'0'},{maximum:'9'},{maximum:'NaN'}])
    assert.equal(api.validate({...current,up:true},{up_price:'11',down_price:'1'},{...rules,...edit}),false);
  assert.equal(api.validate({...current,down:false},{up_price:'',down_price:''},rules),false);
});
test('unknown precision, inverted grids, zero quantities and excessive exponents fail closed',()=>{
  for(const [index,value] of [[0,'0'],[1,'0'],[1,'2.5'],[2,'0'],[3,'4'],[5,'OTHER'],[6,-1],[6,21],[7,'Infinity']]) {
    const args=[...cases[0].args];args[index]=value;assert.equal(api.cap(...args),null);
  }
});
test('displayed cap follows source flooring and must still satisfy the symbol tick',()=>{
 const value=api.bounds({qty:'3',count:'33',upper:'0.004573',lower:'0.003343',type:'GEO'},
  {min_quantity:'1',min_price:'0.000001',max_price:'1',tick:'0.000001'});
 assert.equal(value.maximum,'0.995502');assert.equal(value.minimum,'0.000001');assert.equal(value.tick,'0.000001');
 assert.equal(api.bounds({qty:'3',count:'33',upper:'0.004573',lower:'0.003343',type:'GEO'},
  {min_quantity:'',min_price:'0.000001',max_price:'1',tick:'0.000001'}),null);
});
test('running editor recalculates quote quantity rather than substituting saved per-grid quantity',()=>{
  const args=['LONG','100','5','10','7','','10','3','0.8',2,2,'ARITH','7',169];
  assert.equal(api.quoteQuantity(...args),'15.2727');
  assert.equal(api.quoteQuantity(...args.slice(0,5),'8',...args.slice(6)),'17.4545');
  assert.equal(api.quoteQuantity('SHORT',...args.slice(1)),'15.5844');
  assert.equal(api.quoteQuantity('UNKNOWN',...args.slice(1)),null);
});
test('editor range binds complete fresh public inputs to the actual strategy symbol',()=>{
 const snapshot={symbol:'TESTUSDT',provider_status:'WORKING',trailing:{up:true,down:false,lower:'5',upper:'10',count:10,type:'ARITH'},
  protection:{direction:'LONG',trigger_price:''},investment:{initial_value:'100',initial_leverage:3,total_adjustment:'0'}};
 const market={symbol:'TESTUSDT',tick:'0.01',min_quantity:'0.01',min_price:'0.01',max_price:'100000',quantity_step:'0.01',price_precision:2,
  mark:'7',last:'7',observed_at:100000};
 const coefficients={adjust:'0.8',window_count:169};
 assert.equal(api.editorBounds(snapshot,market,coefficients,100000).maximum,'509');
 for(const bad of [{symbol:'OTHERUSDT'},{observed_at:70000},{observed_at:106000},{min_quantity:''},{quantity_step:'0'},{price_precision:2.1}])
  assert.equal(api.editorBounds(snapshot,{...market,...bad},coefficients,100000),null);
 assert.equal(api.editorBounds(snapshot,market,{adjust:'',window_count:169},100000),null);
 assert.equal(api.editorBounds({...snapshot,investment:{...snapshot.investment,total_adjustment:'-100'}},market,coefficients,100000),null);
});
