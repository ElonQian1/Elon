(function (root) {
  'use strict';
  const states = { ready: '已同步', no_update: '暂无新结果', missing: '原任务已删除或不可访问', auth_required: '分享者需登录原账号', requires_action: '需分享者处理', unavailable: '同步暂不可用' };
  const date = value => value ? new Date(value).toLocaleString() : '尚未同步';
  // Group-only chrome shares the existing feature triggers and their permission checks.
  function installLayout(currentGroup, session, trigger, summary) {
    const app = document.getElementById('appView'), more = document.getElementById('moreBtn');
    if (!app || !more || !summary) return () => {};
    const { el, button } = root.ElonAiShareRich;
    let menu, menuGroup, menuOwner, forwarding = false, frame;
    const active = () => !!currentGroup() && !summary.classList.contains('hidden');
    const list = document.getElementById('chatList');
    let pinned = true, layoutGroup;
    let viewportWidth = root.innerWidth, restingHeight = root.visualViewport?.height || root.innerHeight;
    list?.addEventListener('scroll', () => {
      pinned = list.scrollHeight - list.scrollTop - list.clientHeight <= 24;
    }, { passive: true });
    if (list && root.ResizeObserver) new ResizeObserver(() => {
      if (active() && pinned) list.scrollTop = list.scrollHeight;
    }).observe(list);
    function viewport() {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const visual = root.visualViewport;
        if (!active() || (visual && Math.abs(visual.scale - 1) > .01)) return;
        const height = visual?.height || root.innerHeight;
        // iOS may shrink innerHeight alongside VisualViewport, so retain the pre-keyboard
        // height. Rotation changes the width and starts a fresh baseline. Browser bars
        // alone do not open the keyboard; only a focused editor can begin that state.
        const resizedWidth = Math.abs(root.innerWidth - viewportWidth) > 1;
        const wasOpen = !resizedWidth && app.classList.contains('group-keyboard-open');
        if (resizedWidth) { viewportWidth = root.innerWidth; restingHeight = height; }
        const typing = document.activeElement === document.getElementById('messageInput');
        const keyboard = (typing || wasOpen) && Math.max(root.innerHeight, restingHeight) - height > 120;
        // Keep the baseline through the opening animation, including its first small frames.
        if (!typing && !wasOpen) restingHeight = height;
        else restingHeight = Math.max(restingHeight, height);
        app.style.setProperty('--group-viewport-height', height + 'px');
        app.style.setProperty('--group-viewport-top', (visual?.offsetTop || 0) + 'px');
        app.classList.toggle('group-keyboard-open', keyboard);
      });
    }
    function sync() {
      const enabled = active(); app.classList.toggle('group-chat-active', enabled);
      if (layoutGroup !== currentGroup()?.id) { layoutGroup = currentGroup()?.id; pinned = true; }
      if (enabled) {
        more.title = '群聊工具'; more.setAttribute('aria-label', '群聊工具');
        more.setAttribute('aria-haspopup', 'dialog'); more.setAttribute('aria-expanded', String(!!menu?.open));
      } else {
        app.classList.remove('group-keyboard-open');
        app.style.removeProperty('--group-viewport-height'); app.style.removeProperty('--group-viewport-top');
        if (more.title === '群聊工具') { more.title = '聊天设置'; more.setAttribute('aria-label', more.title); }
        more.removeAttribute('aria-haspopup'); more.removeAttribute('aria-expanded');
      }
      if (menu && (!enabled || menuGroup !== currentGroup()?.id || menuOwner !== session())) menu.close();
      viewport();
    }
    more.addEventListener('click', event => {
      if (!active() || forwarding || more.classList.contains('project-members-mode')) return;
      event.preventDefault(); event.stopImmediatePropagation();
      if (menu?.open) return;
      const dialog = el('dialog', null, 'group-chat-tools'), heading = el('header');
      let chosen = false;
      menu = dialog; menuGroup = currentGroup().id; menuOwner = session();
      dialog.setAttribute('aria-label', '群聊工具');
      heading.append(el('h2', '群聊工具'), button('关闭', () => dialog.close())); dialog.append(heading);
      const action = (label, run) => dialog.append(button(label, () => {
        const valid = active() && menuGroup === currentGroup()?.id && menuOwner === session();
        chosen = true; dialog.close(); if (valid) run();
      }));
      action('群 AI 助手', () => trigger.click());
      action('文章', () => document.querySelector('#chatView > .article-entry')?.click());
      action(summary.querySelector('.summary-action')?.textContent === '生成' ? '生成总结帖' : '总结帖', () => summary.click());
      action('群聊设置', () => { forwarding = true; try { more.click(); } finally { forwarding = false; } });
      dialog.addEventListener('close', () => {
        dialog.remove(); if (menu === dialog) menu = null;
        more.setAttribute('aria-expanded', 'false');
        if (!chosen && active()) more.focus({ preventScroll: true });
      }, { once: true });
      document.body.append(dialog); dialog.showModal(); more.setAttribute('aria-expanded', 'true');
    }, true);
    root.addEventListener('resize', viewport);
    document.getElementById('messageInput')?.addEventListener('focus', viewport);
    root.visualViewport?.addEventListener('resize', viewport);
    root.visualViewport?.addEventListener('scroll', viewport);
    return sync;
  }
  function install(api, currentGroup, session, trigger, summary) {
    let dialog, controller, epoch = 0;
    const { el, button, markdown } = root.ElonAiShareRich;
    const syncLayout = installLayout(currentGroup, session, trigger, summary);
    const visible = () => { trigger.hidden = !currentGroup() || summary.classList.contains('hidden'); syncLayout(); };
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
