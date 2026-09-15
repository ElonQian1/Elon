/* Square author publishing UI. The host supplies only its existing authenticated fetch. */
(() => {
  'use strict';
  const CREATOR='https://www.binance.com/square/creator-center/home',ROOT='/api/me/article-channels/binance-square';
  const labels={queued:'等待发布',preparing:'准备媒体',submitting:'提交中',published:'已发布',failed:'发布失败',uncertain:'结果待核实',cancelled:'已取消'};
  const modes={article:'长文章',text:'文字动态',images:'图片动态',video:'视频动态'};
  const el=(tag,text,cls)=>{const n=document.createElement(tag);if(text!=null)n.textContent=text;if(cls)n.className=cls;return n;};
  const btn=(text,fn)=>{const b=el('button',text);b.type='button';b.onclick=fn;return b;};
  const link=(text,url)=>{const a=el('a',text);a.href=url;a.target='_blank';a.rel='noopener noreferrer';return a;};
  const img=src=>{const i=el('img');i.src=src;i.alt='将发布的媒体';i.style.maxHeight='190px';i.style.maxWidth='100%';return i;};
  const stamp=t=>new Date(t*1000).toLocaleString();
  const requestId=()=>Array.from(crypto.getRandomValues(new Uint8Array(24)),b=>b.toString(16).padStart(2,'0')).join('');
  function secureBase(){if(location.hostname==='43.139.149.158')return 'https://43.139.149.158:8443';if(location.protocol!=='https:')throw Error('当前服务器未配置受信任HTTPS，请联系管理员');return location.origin;}
  function open(api,article){
    if(!window.isSecureContext){const dialog=el('dialog',null,'article-dialog square-dialog'),panel=el('section',null,'article-fields');panel.append(el('h2','打开币安发布安全页面'),el('p','请在完整HTTPS页面中登录并管理发帖凭证。'));try{panel.append(link('打开安全页面 ↗',secureBase()+'/square'+(article?'?article='+encodeURIComponent(article.id):'')));}catch(e){panel.append(el('p',e.message));}panel.append(btn('返回文章',()=>{dialog.close();dialog.remove();}));dialog.append(panel);document.body.append(dialog);dialog.showModal();return;}
    const dialog=el('dialog',null,'article-dialog square-dialog'),panel=el('section',null,'article-panel');dialog.append(panel);document.body.append(dialog);dialog.showModal();
    let account,tab=article?'publish':'history',busy=false,rows=[],next=null,preview=null,mode='article',cover=article?.document.cover||null,ids=[],video=null;
    const media={...(article?.media||{})};let requestKey=requestId();
    const status=el('p','','article-status');status.setAttribute('role','status');
    const close=()=>{if(!busy){dialog.close();dialog.remove();}};dialog.addEventListener('cancel',e=>{e.preventDefault();close();});
    const request=async(path,method='GET',value)=>{const c=new AbortController(),timer=setTimeout(()=>c.abort(),value instanceof FormData?120000:45000);try{
      const r=await api(secureBase()+ROOT+path,{method,credentials:'omit',redirect:'error',signal:c.signal,...(value===undefined?{}:{body:value instanceof FormData?value:JSON.stringify(value)})});const data=await r.json().catch(()=>null);if(!r.ok)throw Error(data?.error||`请求失败（${r.status}）`);if(!data)throw Error('回执不完整，请刷新发布记录');return data;
    }catch(e){if(e.name==='AbortError'||e instanceof TypeError)throw Error('安全连接未完成。若刚才正在发布，请先刷新发布记录核实。');throw e;}finally{clearTimeout(timer);}};
    const run=async fn=>{if(busy)return;busy=true;status.textContent='正在处理…';const controls=[...panel.querySelectorAll('button,input,select')];controls.forEach(n=>{n.dataset.wasDisabled=String(n.disabled);n.disabled=true;});try{await fn();if(status.textContent==='正在处理…')status.textContent='';}catch(e){status.textContent=e.message||'操作失败，请重试';}finally{busy=false;controls.forEach(n=>{n.disabled=n.dataset.wasDisabled==='true';});}};
    const field=(parent,title,type,value='')=>{const label=el('label',title),input=el('input');input.type=type;input.value=value;if(type==='checkbox'){label.className='square-choice';label.replaceChildren(input,document.createTextNode(title));}else label.append(input);parent.append(label);return input;};
    function header(){panel.replaceChildren();const bar=el('header');bar.append(btn('‹ 返回文章',close),el('strong','币安广场'));panel.append(bar,status);const nav=el('nav');if(article)nav.append(btn('发布内容',()=>{if(!busy){tab='publish';render();}}));nav.append(btn('发布记录',()=>{if(!busy){tab='history';render();void run(()=>loadHistory());}}),btn('绑定账号',()=>{if(!busy){tab='account';render();}}));panel.append(nav);}
    function render(){header();if(!account){panel.append(btn('重试连接',()=>void run(loadAccount)));return;}if(tab==='account')accountView();else if(tab==='history')historyView();else compose();}
    async function loadAccount(){try{account=await request('/account');if(!account.bound)tab='account';render();if(tab==='history')await loadHistory();}catch(e){if(!account)render();throw e;}}
    function accountView(){
      const f=el('section',null,'article-fields');panel.append(f);f.append(el('h3',account.bound?account.label:'绑定币安广场账号'),el('p',account.bound?`${account.masked_key} · ${account.verified_at?'已通过发帖验证 · '+stamp(account.verified_at):'凭证已保存，尚未通过实际发帖验证'}`:'每个一龙账号绑定自己的Square发帖凭证。'),link('打开币安创作者中心，获取发帖凭证 ↗',CREATOR));
      const label=field(f,'账号备注','text',account.label);label.maxLength=60;const key=field(f,account.bound?'更换发帖凭证':'发帖凭证','password');key.autocomplete='new-password';key.maxLength=512;
      f.append(el('p','通过HTTPS传输并加密保存，无需交易密钥。更换或解绑会取消尚未提交的任务。'),btn('保存绑定',()=>void run(async()=>{account=await request('/account','PUT',{label:label.value,api_key:key.value});key.value='';accountViewReplace();status.textContent='已保存，首次发帖成功后更新验证状态';})));
      if(account.bound)f.append(btn('解绑账号',()=>{if(confirm('解绑会取消尚未提交任务并删除凭证，币安已有帖子不受影响。'))void run(async()=>{account=await request('/account','DELETE');key.value='';accountViewReplace();});}));
    }
    function accountViewReplace(){header();accountView();}
    let historyList;
    function historyView(){const f=el('section',null,'article-fields');historyList=el('div');f.append(el('p','关闭页面不影响定时任务。一龙撤下文章不会删除币安帖子。'),link('在币安管理已有内容 ↗',CREATOR),btn('刷新记录',()=>void run(()=>loadHistory())),historyList);panel.append(f);}
    async function loadHistory(offset=0){const p=await request('/jobs?offset='+offset);rows=offset?rows.concat(p.items):p.items;next=p.next_offset;historyList.replaceChildren();if(!rows.length)historyList.append(el('p','还没有发布记录，从文章预览页选择币安广场开始。'));
      rows.forEach(j=>{const r=el('section',null,'article-block');r.append(el('h3',j.title),el('p',`${modes[j.mode]} · v${j.revision} · ${labels[j.status]||j.status}`),el('small','计划 '+stamp(j.scheduled_at)+' · 尝试 '+j.attempts+' 次'),el('p',j.message));
        if(j.post_url&&/^https:\/\/www\.binance\.com\/en\/square\/post\/\d+$/.test(j.post_url))r.append(link('打开币安帖子 ↗',j.post_url));
        const act=(verb,data={})=>run(async()=>{await request(`/jobs/${j.id}/${verb}`,'POST',data);await loadHistory();});
        if(['queued','preparing','failed'].includes(j.status))r.append(btn('取消任务',()=>void act('cancel')));
        if(['failed','cancelled'].includes(j.status))r.append(btn('重试此版本',()=>{if(confirm('将按记录保存的文章版本重新发布，确定继续？'))void act('retry');}));
        if(j.status==='uncertain')r.append(btn('核实发布结果',()=>{const box=el('div');box.append(el('p','请先到币安核实，避免重复发帖。'));const input=field(box,'帖子链接或数字ID','text');box.append(btn('记录已发布',()=>void run(async()=>{const match=input.value.trim().match(/^(?:https:\/\/www\.binance\.com\/(?:[\w-]+\/)?square\/post\/)?(\d{1,64})(?:\?.*)?$/);if(!match)throw Error('请填写有效的币安帖子链接或ID');await request(`/jobs/${j.id}/resolve`,'POST',{post_id:match[1]});await loadHistory();})),btn('我已核实未发布',()=>{if(confirm('我已在币安核实，此内容没有发布；确认后可重试。'))void act('resolve',{not_published:true});}));r.append(box);}));historyList.append(r);
      });if(next!==null)historyList.append(btn('加载更多',()=>void run(()=>loadHistory(next))));
    }
    function invalidate(){preview=null;requestKey=requestId();}
    function compose(){
      const f=el('section',null,'article-fields');panel.append(f);if(!account.bound){f.append(el('p','请先绑定币安广场账号'));return;}if(!article){f.append(el('p','请先打开一篇自己的文章'));return;}
      f.append(el('h3',article.document.title),el('p',`使用已保存文章 v${article.revision}。群聊权限保持原样。`));
      const select=el('select');Object.entries(modes).forEach(([v,t])=>{const o=el('option',t);o.value=v;select.append(o);});select.value=mode;select.onchange=()=>{mode=select.value;invalidate();header();compose();};const modeLabel=el('label','发布形式');modeLabel.append(select);f.append(modeLabel);
      if(mode==='article'){const c=el('select');const none=el('option','不使用封面');none.value='';c.append(none);Object.keys(media).forEach((id,i)=>{const o=el('option','图片 '+(i+1));o.value=id;c.append(o);});c.value=cover||'';c.onchange=()=>{cover=c.value||null;invalidate();header();compose();};f.append(el('p','文章封面'),c);if(cover&&media[cover])f.append(img(media[cover]));}
      if(mode==='images')Object.entries(media).forEach(([id,url],i)=>{const l=el('label','图片 '+(i+1)),c=el('input');c.type='checkbox';c.checked=ids.includes(id);c.disabled=!c.checked&&ids.length>=4;c.onchange=()=>{ids=c.checked?ids.concat(id):ids.filter(x=>x!==id);invalidate();header();compose();};l.append(img(url),c);f.append(l);});
      if(mode==='article'||mode==='images'){const picker=field(f,'添加图片','file');picker.accept='image/png,image/jpeg,image/webp';picker.onchange=()=>{const file=picker.files[0];if(file)void run(async()=>{const m=await window.ElonSquareMedia.image(file,request);media[m.id]=m.data_url;if(mode==='article')cover=m.id;else if(ids.length<4&&!ids.includes(m.id))ids.push(m.id);invalidate();header();compose();});};}
      if(mode==='video'){f.append(el('p','MP4/WebM，最多32MB、10分钟，自动使用首帧封面'));const picker=field(f,'选择视频','file');picker.accept='video/mp4,video/webm';picker.onchange=()=>{const file=picker.files[0];if(file)void run(async()=>{video=await window.ElonSquareMedia.video(file,request);invalidate();header();compose();});};if(video)f.append(img(video.cover),el('p',video.name));}
      f.append(btn('生成币安版本预览',()=>void run(async()=>{preview=await request('/preview','POST',{article_id:article.id,version:article.revision,mode,media_ids:mode==='images'?ids:[],cover_id:mode==='article'?cover:null,video_id:mode==='video'?video?.id||null:null});header();compose();})));
      if(preview)previewView(f);
    }
    function previewView(f){const p=el('section',null,'article-publish');p.setAttribute('aria-label','币安版本预览');p.append(el('h3','发布到：'+preview.account_label));if(mode==='article')p.append(el('h2',preview.title));(preview.selection.mode==='images'?preview.selection.media_ids:Object.keys(preview.media)).forEach(id=>p.append(img(preview.media[id])));const text=el('p',preview.text);text.style.whiteSpace='pre-wrap';text.style.overflowWrap='anywhere';text.style.maxHeight='420px';text.style.overflow='auto';p.append(text);preview.warnings.forEach(w=>p.append(el('p',w)));
      const schedule=field(p,'发送时间（留空立即发送）','datetime-local'),converted=field(p,'已核对文字转换和选定媒体','checkbox'),publicOk=field(p,'将此内容公开发布到上述币安账号','checkbox');
      p.append(el('p','一龙撤下文章不会删除币安帖子。提交结果不确定时，先到发布记录核实。'));
      const send=btn('确认公开发布',()=>void run(async()=>{const at=schedule.value?Math.floor(new Date(schedule.value).getTime()/1000):null;if(at!==null&&(!Number.isFinite(at)||at<=Date.now()/1000))throw Error('请选择未来的发送时间');const j=await request('/jobs','POST',{selection:preview.selection,preview_hash:preview.preview_hash,request_key:requestKey,scheduled_at:at,public_confirmed:publicOk.checked,conversion_confirmed:converted.checked});tab='history';header();historyView();await loadHistory();status.textContent='任务已记录 · '+(labels[j.status]||j.status);}));send.disabled=true;
      const update=()=>{send.disabled=!converted.checked||!publicOk.checked;send.textContent=schedule.value?'确认定时发布':'确认公开发布';};converted.onchange=update;publicOk.onchange=update;schedule.onchange=update;p.append(send);f.append(p);
    }
    header();void run(loadAccount);
  }
  window.ElonSquare={open};
})();
