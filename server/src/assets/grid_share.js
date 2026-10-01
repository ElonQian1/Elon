/* Group-authorized historical grid reader. Never opens or requests an exchange session. */
(function (root) {
  'use strict';
  const { el, button } = root.ElonAiShareRich;
  const prefix = '【一龙AI对话】\n';
  let current = null;
  const labels = {
    symbol:'合约', direction:'方向', leverage:'杠杆', status:'采集时状态', lower:'区间下限', upper:'区间上限', count:'网格数量', spacing:'间距',
    markPrice:'标记价格', roi:'策略总收益率 (%)', entryPrice:'持仓均价', liquidationPrice:'预估强平价', positionQty:'实际持仓数量', positionNotional:'持仓货值 (USDT)',
    totalPnl:'策略总盈亏 (USDT)', unrealizedPnl:'未实现盈亏 (USDT)', profit:'网格利润 (USDT)', matchedPnl:'已配对收益 (USDT)', fee:'手续费 (USDT)', fundingFee:'资金费 (USDT)',
    investment:'投入保证金 (USDT)', initialNotional:'初始货值 (USDT)', perGridQty:'每格基础币数量', perGridQuoteQty:'每格报价币数量', matchedCount:'配对次数', marginType:'保证金模式', orderCurrency:'下单计量', stopUpper:'止损上限', stopLower:'止损下限',
  };
  const sections = { 持仓:['positionQty','positionNotional','entryPrice','markPrice','liquidationPrice','marginType'], 收益:['roi','totalPnl','profit','matchedPnl','unrealizedPnl','fee','fundingFee','matchedCount'], 参数:['direction','leverage','lower','upper','count','spacing','investment','initialNotional','perGridQty','perGridQuoteQty','stopUpper','stopLower'] };
  const money = new Set(['positionQty','positionNotional','totalPnl','unrealizedPnl','profit','matchedPnl','fee','fundingFee','investment','initialNotional','perGridQty','perGridQuoteQty']);
  const value = (grid, key) => !grid.show_amounts && money.has(key) ? '未公开' : ({LONG:'做多',SHORT:'做空',NEUTRAL:'中性',ARITH:'等差',GEO:'等比',CROSSED:'全仓',ISOLATED:'逐仓'}[grid.fields[key]] || grid.fields[key] || '未读取');
  function reference(text, group) {
    try {
      if (!text.startsWith(prefix) || text.length > 8000) return null;
      const ref=JSON.parse(text.slice(prefix.length));
      return ref.schema === 'elon.ai_conversation_share.v1' && ref.provider === 'binance' && ref.group_id === group
        && /^ai_snapshot_[\w-]+$/.test(ref.snapshot_id) && /^[\w-]{1,160}$/.test(group) && ref.grid?.schema === 'yilong.grid_share.v1' ? ref : null;
    } catch { return null; }
  }
  function close() { current?.abort(); current=null; }
  async function open(ref, options) {
    close(); const run=new AbortController(); current=run;
    const dialog=el('dialog',null,'ai-share-reader'), body=el('div',null,'ai-share-reader-body');
    const header=el('header'); header.append(el('strong','网格快照详情'),button('关闭',close)); dialog.append(header,body);
    run.signal.addEventListener('abort',()=>dialog.remove(),{once:true}); dialog.addEventListener('cancel',e=>{e.preventDefault();close();});
    document.body.append(dialog); dialog.showModal(); body.append(el('p','正在读取…'));
    const timer=setTimeout(()=>run.abort(),25000);
    try {
      if (!options.isCurrent()) throw Error('群聊已变化');
      const response=await options.api('/api/me/groups/'+encodeURIComponent(ref.group_id)+'/ai-snapshots/'+encodeURIComponent(ref.snapshot_id),{signal:run.signal,cache:'no-store',redirect:'error'});
      if (!response.ok) throw Error([401,403,404,410].includes(response.status)?'分享已撤回或你已无权查看':'读取失败，请重试');
      const view=await response.json(),grid=view.document?.grid;
      if (run.signal.aborted || !options.isCurrent()) {close();return;}
      if(view.group_id!==ref.group_id || view.snapshot_id!==ref.snapshot_id || grid?.schema!=='yilong.grid_share.v1') throw Error('分享格式不受支持');
      body.replaceChildren(el('h2',view.document.title),el('p',new Date(grid.observed_at_ms).toLocaleString()+' 的历史快照'));
      const tabs=el('nav'),rows=el('dl');
      function show(tab){rows.replaceChildren();sections[tab].forEach(key=>rows.append(el('dt',labels[key]),el('dd',value(grid,key))));}
      Object.keys(sections).forEach(tab=>tabs.append(button(tab,()=>show(tab))));body.append(tabs,rows);show('参数');
      if(grid.note)body.append(el('blockquote',grid.note));
      body.append(el('p','未读取不代表零；网格利润与策略总盈亏不同。打开详情不会刷新币安。'));
      if(/^ai_snapshot_[\w-]+$/.test(view.latest_snapshot_id||''))body.append(button('已有更新 · 查看最新快照',()=>open({...ref,snapshot_id:view.latest_snapshot_id},options)));
    } catch(error){if(!run.signal.aborted){body.replaceChildren(el('p',error.message),button('重试',()=>open(ref,options)));}}
    finally {clearTimeout(timer);}
  }
  function mount(bubble,message,options){
    const ref=reference(message.content,options.groupId);if(!ref)return false;
    const card=button('',()=>open(ref,options));card.className='ai-share-card';card.setAttribute('aria-label','查看 '+ref.title);
    card.append(el('small','币安 · 网格持仓分享'),el('strong',ref.title),el('span',ref.summary),el('small',new Date(ref.grid.observed_at_ms).toLocaleString()+' 的快照'),el('span','查看网格详情 ›'));
    bubble.replaceChildren(card);return true;
  }
  root.ElonGridShare={mount,close,reference};
})(globalThis);
