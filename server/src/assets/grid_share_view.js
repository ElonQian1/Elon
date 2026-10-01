/* Presentation only: local icons and immutable, allowlisted snapshot fields. */
(function (root) {
  'use strict';
  const { el, button } = root.ElonAiShareRich;
  const labels = {
    direction:'方向', leverage:'杠杆', lower:'区间下限', upper:'区间上限', count:'网格数量', spacing:'间距',
    markPrice:'标记价格', roi:'策略总收益率 (%)', entryPrice:'持仓均价', liquidationPrice:'预估强平价', positionQty:'实际持仓数量', positionNotional:'持仓货值 (USDT)',
    totalPnl:'策略总盈亏 (USDT)', unrealizedPnl:'未实现盈亏 (USDT)', profit:'网格利润 (USDT)', matchedPnl:'已配对收益 (USDT)', fee:'手续费 (USDT)', fundingFee:'资金费 (USDT)',
    investment:'投入保证金 (USDT)', initialNotional:'初始货值 (USDT)', perGridQty:'每格基础币数量', perGridQuoteQty:'每格报价币数量', matchedCount:'配对次数', marginType:'保证金模式', orderCurrency:'下单计量', stopUpper:'止损上限', stopLower:'止损下限',
  };
  const sections = { 持仓:['positionQty','positionNotional','entryPrice','markPrice','liquidationPrice','marginType'], 收益:['roi','totalPnl','profit','matchedPnl','unrealizedPnl','fee','fundingFee','matchedCount'], 参数:['direction','leverage','lower','upper','count','spacing','investment','initialNotional','perGridQty','perGridQuoteQty','orderCurrency','stopUpper','stopLower'] };
  const money = new Set(['positionQty','positionNotional','totalPnl','unrealizedPnl','profit','matchedPnl','fee','fundingFee','investment','initialNotional','perGridQty','perGridQuoteQty']);
  const gains = new Set(['roi','totalPnl','unrealizedPnl','profit','matchedPnl']);
  const words = {LONG:'做多',SHORT:'做空',NEUTRAL:'中性',ARITH:'等差',GEO:'等比',CROSSED:'全仓',ISOLATED:'逐仓',BASE:'基础币',QUOTE:'报价币',WORKING:'运行中',RUNNING:'运行中',NEW:'运行中'};
  function raw(grid,key) {
    if (grid.show_amounts !== true && money.has(key)) return null;
    const item = grid.fields?.[key];
    return typeof item === 'string' || typeof item === 'number' ? String(item) : null;
  }
  function numeric(value) { return value != null && /^-?\d{1,40}(?:\.\d{1,30})?$/.test(value); }
  function tone(value) { return !numeric(value) || /^-?0(?:\.0+)?$/.test(value) ? 'neutral' : value.startsWith('-') ? 'negative' : 'positive'; }
  // Keep exact decimal strings; never convert financial display values through Number.
  function compact(value,signed=false) {
    if (!numeric(value)) return '未读取';
    let [whole,fraction=''] = value.split('.');
    fraction=fraction.replace(/0+$/,'');
    const truncated=fraction.length>8;
    const clipped=fraction.slice(0,8).replace(/0+$/,'');
    const display=whole.replace(/\B(?=(\d{3})+(?!\d))/g,',')+(clipped?'.'+clipped:'');
    const sign=tone(value);
    if(truncated && /^-?0$/.test(display))return sign==='negative'?'−<0.00000001':(signed?'+':'')+'<0.00000001';
    return (signed && sign==='positive'?'+':'')+display+(truncated?'…':'');
  }
  function value(grid,key) {
    if(grid.show_amounts !== true && money.has(key))return '未公开';
    const item=raw(grid,key);return item == null || item === '' ? '未读取' : words[item] || item;
  }
  function time(grid) {
    const date=new Date(grid.observed_at_ms);
    return Number.isFinite(date.getTime())?date.toLocaleString()+' 的快照':'采集时间未读取';
  }
  function icon(symbol) {
    const coin=String(symbol||'').toUpperCase().replace(/(?:USDT|USDC|BUSD)$/,'');
    const holder=el('span',coin.slice(0,2)||'币','grid-share-logo');holder.setAttribute('aria-hidden','true');
    if(/^[A-Z0-9]{1,24}$/.test(coin)){
      const img=el('img');img.alt='';img.width=40;img.height=40;
      img.src='/assets/grid-token-icons/'+coin.toLowerCase()+'.png';
      img.addEventListener('error',()=>img.remove(),{once:true});holder.append(img);
    }
    return holder;
  }
  function metric(grid) {
    const key=['roi','totalPnl','profit'].find(key=>numeric(raw(grid,key)));
    return key?{key,label:labels[key],value:raw(grid,key)}:null;
  }
  function stat(grid,key,label) {
    const item=raw(grid,key),cell=el('span',null,'grid-share-stat');
    const amount=el('b',item==null?value(grid,key):compact(item,gains.has(key)),gains.has(key)?'grid-share-'+tone(item):'');
    amount.title=value(grid,key);cell.append(el('small',label),amount);return cell;
  }
  function range(grid) {
    const wrap=el('span',null,'grid-share-range'),bar=el('span',null,'grid-share-range-track');
    const lower=Number(raw(grid,'lower')),upper=Number(raw(grid,'upper')),mark=Number(raw(grid,'markPrice'));
    let state='标记价未读取';
    if(['lower','upper','markPrice'].every(key=>numeric(raw(grid,key))) && upper>lower){
      const ratio=Math.max(0,Math.min(1,(mark-lower)/(upper-lower)));
      const fill=el('span',null,'grid-share-range-fill'),dot=el('span',null,'grid-share-range-dot');
      fill.style.width=ratio*100+'%';dot.style.left=ratio*100+'%';bar.append(fill,dot);
      state=mark<lower?'低于区间':mark>upper?'高于区间':'区间内';
    }
    bar.setAttribute('aria-hidden','true');
    const bounds=el('span',null,'grid-share-range-bounds');bounds.append(el('span',compact(raw(grid,'lower'))),el('span',compact(raw(grid,'upper'))));
    wrap.append(el('small','价格区间 · '+state),bar,bounds,el('small','标记价 '+compact(raw(grid,'markPrice'))));return wrap;
  }
  function summary(grid) {
    const card=el('span',null,'grid-share-summary');
    const source=el('span','币安 · 网格持仓分享','grid-share-source');
    const heading=el('span',null,'grid-share-heading'),title=el('span',null,'grid-share-title');
    title.append(el('strong',value(grid,'symbol')),el('small',value(grid,'count')+' 格 · '+value(grid,'spacing')));
    heading.append(icon(raw(grid,'symbol')),title);
    const tags=el('span',null,'grid-share-tags');
    const direction=raw(grid,'direction');tags.append(el('span',value(grid,'direction'),'grid-share-tag grid-share-'+(direction==='LONG'?'positive':direction==='SHORT'?'negative':'neutral')),el('span',value(grid,'leverage')+'×','grid-share-tag'));
    if(raw(grid,'status'))tags.append(el('span',value(grid,'status'),'grid-share-tag'));
    const hero=metric(grid),panel=el('span',null,'grid-share-profit grid-share-'+tone(hero?.value));
    panel.append(el('small',hero?.label || (grid.show_amounts===true?'收益未读取':'历史分享未公开金额')),el('b',hero?compact(hero.value,true)+(hero.key==='roi'?'%':''):'等待完整数据','grid-share-profit-value'));
    if(hero)panel.title=hero.value;
    if(hero?.key==='profit')panel.append(el('small','网格利润不代表策略总盈亏'));
    card.append(source,heading,tags,panel);
    if(grid.show_amounts===true){const stats=el('span',null,'grid-share-stats');[['unrealizedPnl','未实现盈亏 · USDT'],['investment','投入保证金 · USDT'],['positionQty','持仓数量'],['positionNotional','持仓货值 · USDT']].forEach(([key,label])=>stats.append(stat(grid,key,label)));card.append(stats);}
    card.append(range(grid),el('small',time(grid),'grid-share-time'),el('small',grid.show_amounts===true?'金额与数量已公开':'历史分享未公开金额','grid-share-disclosure'));
    return card;
  }
  function details(grid) {
    const section=el('section',null,'grid-share-details'),tabs=el('div',null,'grid-share-tabs'),rows=el('dl',null,'grid-share-rows');
    tabs.setAttribute('role','tablist');tabs.setAttribute('aria-label','网格详情分区');rows.setAttribute('role','tabpanel');
    const controls=Object.keys(sections).map(tab=>{
      const control=button(tab,()=>show(tab));control.setAttribute('role','tab');control.id='grid-share-tab-'+tab;control.setAttribute('aria-controls','grid-share-rows');return control;
    });
    function show(tab){
      controls.forEach(control=>{const selected=control.textContent===tab;control.setAttribute('aria-selected',String(selected));control.tabIndex=selected?0:-1;});
      rows.setAttribute('aria-labelledby','grid-share-tab-'+tab);rows.replaceChildren();
      sections[tab].forEach(key=>{const dd=el('dd',value(grid,key),gains.has(key)?'grid-share-'+tone(raw(grid,key)):'');rows.append(el('dt',labels[key]),dd);});
    }
    tabs.addEventListener('keydown',event=>{const i=controls.indexOf(document.activeElement);if(i<0 || !['ArrowLeft','ArrowRight','Home','End'].includes(event.key))return;event.preventDefault();const next=event.key==='Home'?0:event.key==='End'?2:(i+(event.key==='ArrowRight'?1:2))%3;controls[next].click();controls[next].focus();});
    rows.id='grid-share-rows';tabs.append(...controls);section.append(tabs,rows);show('持仓');
    if(grid.note)section.append(el('blockquote',grid.note,'grid-share-note'));
    return section;
  }
  root.ElonGridShareView={summary,details,metric,compact,tone};
})(globalThis);
