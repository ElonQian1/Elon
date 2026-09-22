(function (root) {
  'use strict';
  const states = { ready: '已同步', no_update: '暂无新结果', missing: '原任务已删除或不可访问', auth_required: '分享者需登录原账号', requires_action: '需分享者处理', unavailable: '同步暂不可用' };
  const date = value => value ? new Date(value).toLocaleString() : '尚未同步';
  function install(api, currentGroup, session, trigger, summary) {
    let dialog, controller, epoch = 0;
    const { el, button, markdown } = root.ElonAiShareRich;
    const visible = () => { trigger.hidden = !currentGroup() || summary.classList.contains('hidden'); };
    new MutationObserver(visible).observe(summary, { attributes: true, attributeFilter: ['class'] });
    visible();
    trigger.onclick = () => {
      const group = currentGroup(), owner = session();
      if (!group || !owner) return;
      dialog?.close();
      const modal = el('dialog', null, 'group-assistant-dialog'), header = el('header'), body = el('div', null, 'group-assistant-body'), status = el('p');
      const title = el('h2', '群 AI 助手'); status.setAttribute('role', 'status');
      header.append(title, button('关闭', () => modal.close(), 'close'));
      modal.append(header, status, body); document.body.append(modal); dialog = modal;
      modal.onclose = () => { epoch++; controller?.abort(); modal.remove(); if (dialog === modal) dialog = null; };
      const base = '/api/me/groups/' + encodeURIComponent(group.id) + '/ai-assistant';
      const valid = id => epoch === id && modal.open && currentGroup()?.id === group.id && session() === owner;
      async function request(path, method = 'GET', render) {
        controller?.abort(); const flight = new AbortController(); controller = flight; const id = ++epoch;
        const timeout = setTimeout(() => flight.abort(), 15000);
        status.textContent = '正在读取';
        try {
          const response = await api(path, { method, signal: flight.signal, cache: 'no-store' });
          if (!response.ok) throw Error('关注事项不可用或权限已变更，请刷新后重试');
          const data = await response.json();
          if (valid(id)) { status.textContent = '分享者在线时同步 · 最近结果保留 100 条'; render(data); }
          else if (session() !== owner || currentGroup()?.id !== group.id) modal.close();
        } catch (error) { if (valid(id)) status.textContent = error.name === 'AbortError' ? '读取超时，请重试' : error.message; }
        finally { clearTimeout(timeout); }
      }
      function tools(back, refresh) {
        const bar = el('div', null, 'group-assistant-tools');
        if (back) bar.append(button('返回', back, 'back'));
        bar.append(button('刷新', refresh, 'retry')); body.append(bar);
      }
      function list() { void request(base, 'GET', data => {
        title.textContent = '群 AI 助手'; body.replaceChildren(); tools(null, list);
        if (!data.items.length) body.append(el('p', '暂无关注事项'));
        data.items.forEach(row => {
          const item = el('section', null, 'group-assistant-row');
          item.append(button(row.title, () => updates(row)), el('p', `${row.owner_name} · ChatGPT · ${states[row.state] || states.unavailable}`), el('small', '最近同步：' + date(row.synced_at)));
          if (row.owned) item.append(button('取消分享', () => {
            if (root.confirm('取消分享并撤下群内结果？ChatGPT 原任务不变，别人已保存的副本无法收回。')) void request(base + '/' + encodeURIComponent(row.id), 'DELETE', list);
          }));
          body.append(item);
        });
      }); }
      function updates(row, before = 0, previous = []) { void request(base + '/' + encodeURIComponent(row.id) + '?before=' + before, 'GET', data => {
        const items = [...previous, ...data.items]; title.textContent = row.title; body.replaceChildren(); tools(list, () => updates(row));
        if (!items.length) body.append(el('p', '尚无已同步更新'));
        items.forEach(update => {
          const item = el('section', null, 'group-assistant-row');
          item.append(button(date(update.created_at), () => {
            body.replaceChildren(); tools(() => updates(row), () => updates(row)); body.append(el('p', date(update.created_at)), markdown(update.content));
          }), el('p', update.content.slice(0, 160))); body.append(item);
        });
        if (data.next_cursor != null) body.append(button('较早更新', () => updates(row, data.next_cursor, items)));
      }); }
      modal.showModal(); list();
    };
  }
  root.ElonGroupAssistant = { install };
})(globalThis);
