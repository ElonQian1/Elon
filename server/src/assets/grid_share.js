/* Group-authorized historical grid reader. Never opens or requests an exchange session. */
(function (root) {
  'use strict';
  const { el, button } = root.ElonAiShareRich;
  const prefix = '【一龙AI对话】\n';
  let current = null;
  function reference(text, group) {
    try {
      if (!text.startsWith(prefix) || text.length > 8000) return null;
      const ref=JSON.parse(text.slice(prefix.length));
      return ref.schema === 'elon.ai_conversation_share.v1' && ref.provider === 'binance' && ref.group_id === group
        && /^ai_snapshot_[\w-]+$/.test(ref.snapshot_id) && /^[\w-]{1,160}$/.test(group) && ref.grid?.schema === 'yilong.grid_share.v1' ? ref : null;
    } catch { return null; }
  }
  function close() { const run=current;current=null;run?.abort();run?.cleanup(); }
  function summary(text) {
    try {
      if (typeof text!=='string' || text.length>8000 || !text.startsWith(prefix)) return null;
      const value=JSON.parse(text.slice(prefix.length)),ref=reference(text,value.group_id);if(!ref)return null;
      const fields=ref.grid.fields||{},symbol=String(fields.symbol||'未读取').slice(0,40);
      const direction={LONG:'做多',SHORT:'做空',NEUTRAL:'中性'}[fields.direction]||'方向未读取';
      const leverage=/^\d{1,3}(?:\.\d{1,2})?$/.test(String(fields.leverage))?' '+fields.leverage+'×':'';
      return '[网格快照] '+symbol+' · '+direction+leverage;
    } catch { return null; }
  }
  async function open(ref, options) {
    close(); const run=new AbortController(); current=run;
    const previousFocus=document.activeElement;
    const dialog=el('dialog',null,'ai-share-reader grid-share-reader'), body=el('div',null,'grid-share-reader-body');
    dialog.setAttribute('aria-label','网格快照详情');
    run.cleanup=()=>{dialog.remove();if(previousFocus?.isConnected)previousFocus.focus();};
    const header=el('header'); header.append(el('strong','网格快照详情'),button('关闭',close)); dialog.append(header,body);
    dialog.addEventListener('cancel',e=>{e.preventDefault();close();});
    document.body.append(dialog); dialog.showModal(); body.append(el('p','正在读取…'));
    const timer=setTimeout(()=>{if(current===run){body.replaceChildren(el('p','读取超时，请重试'),button('重试',()=>open(ref,options)));run.abort();}},25000);
    try {
      if (!options.isCurrent()) throw Error('群聊已变化');
      const response=await options.api('/api/me/groups/'+encodeURIComponent(ref.group_id)+'/ai-snapshots/'+encodeURIComponent(ref.snapshot_id),{signal:run.signal,cache:'no-store',redirect:'error'});
      if (!response.ok) throw Error([401,403,404,410].includes(response.status)?'分享已撤回或你已无权查看':'读取失败，请重试');
      const view=await response.json(),grid=view.document?.grid;
      if (run.signal.aborted || current!==run) return;
      if (!options.isCurrent()) {close();return;}
      if(view.group_id!==ref.group_id || view.snapshot_id!==ref.snapshot_id || grid?.schema!=='yilong.grid_share.v1') throw Error('分享格式不受支持');
      body.replaceChildren(el('p',(view.owner_name || '群成员')+' 分享 · 历史快照','grid-share-owner'),root.ElonGridShareView.summary(grid),root.ElonGridShareView.details(grid));
      body.append(el('p','未读取不代表零；网格利润与策略总盈亏不同。打开详情不会刷新币安。','grid-share-notice'));
      if(/^ai_snapshot_[\w-]+$/.test(view.latest_snapshot_id||''))body.append(button('已有更新 · 查看最新快照',()=>open({...ref,snapshot_id:view.latest_snapshot_id},options)));
    } catch(error){if(!run.signal.aborted){body.replaceChildren(el('p',error.message),button('重试',()=>open(ref,options)));}}
    finally {clearTimeout(timer);}
  }
  function mount(bubble,message,options){
    const ref=reference(message.content,options.groupId);if(!ref)return false;
    const card=button('',()=>open(ref,options));card.className='ai-share-card grid-share-card';card.setAttribute('aria-label','查看 '+ref.title);
    card.append(root.ElonGridShareView.summary(ref.grid),el('span','查看网格详情 ›','grid-share-open'));
    bubble.replaceChildren(card);bubble.classList.add('grid-share-bubble');return true;
  }
  root.ElonGridShare={mount,close,reference,summary};
})(globalThis);
