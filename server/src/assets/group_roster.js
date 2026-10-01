/* Shared group contract, adapted to mobile pages and a wide-window sidebar. */
(() => {
  'use strict';
  const roleName = role => role === 'owner' ? '群主' : role === 'admin' ? '管理员' : '成员';
  function el(tag, text, className) { const node = document.createElement(tag); if (text != null) node.textContent = text; if (className) node.className = className; return node; }
  function button(text, action, className) { const node = el('button', text, className); node.type = 'button'; node.addEventListener('click', action); return node; }
  function avatar(person) {
    const node = el('span', Array.from(person.display_name || '群')[0], 'gr-avatar');
    if (person.avatar_data_url?.startsWith('data:image/')) {
      const image = el('img'); image.src = person.avatar_data_url; image.alt = ''; image.loading = 'lazy';
      image.addEventListener('error', () => image.remove()); node.replaceChildren(image);
    }
    return node;
  }
  function modal(title) {
    const dialog = el('dialog', null, 'gr-modal'); const header = el('header'); const body = el('div', null, 'gr-modal-body');
    header.append(el('strong', title), button('关闭', () => dialog.close())); dialog.append(header, body);
    document.body.append(dialog); dialog.addEventListener('close', () => dialog.remove()); dialog.showModal();
    return { dialog, body };
  }
  function install(options) {
    let host = null, group = '', owner = '', data = null, controller = null, serial = 0, busy = false;
    let query = '', filter = 'all', directory = false, managing = false, selected = new Set(), detail = null;
    let head, tools, status, list, footer, search, next, filterButtons = [], preview;
    let refreshTimer = null, searchTimer = null, current = '', collapsed = false;
    const wide = matchMedia('(min-width: 1101px)');
    async function read(path, init = {}) {
      const response = await options.api(path, init); const result = await response.json();
      if (!response.ok) { const error = new Error(String(result.error || '操作失败').replace(/^[A-Z_]+: /, '')); error.status = response.status; throw error; }
      return result;
    }
    const endpoint = suffix => `/api/me/groups/${encodeURIComponent(group)}/${suffix}`;
    function close() {
      serial++; controller?.abort(); controller = null; busy = false; clearTimeout(refreshTimer); clearTimeout(searchTimer);
      detail?.close(); detail = null;
      if (host?.tagName === 'DIALOG' && host.open) host.close(); host?.remove(); host = null;
      document.querySelector('#chatPage')?.classList.remove('gr-with-members');
      document.querySelector('.app')?.classList.remove('gr-group-layout');
    }
    function dismiss() { collapsed = true; close(); }
    function open(showDirectory = false) {
      const active = options.getGroup(); if (!active) return;
      close(); group = active.id; owner = options.getUserId(); current = `${owner}:${group}`; collapsed = false; data = null; directory = showDirectory || wide.matches;
      query = ''; filter = 'all'; selected = new Set(); managing = false;
      host = el(wide.matches ? 'aside' : 'dialog', null, 'gr-host'); host.setAttribute('aria-label', '当前群成员');
      head = el('header'); tools = el('div', null, 'gr-tools'); status = el('p', '正在加载群成员…', 'gr-status'); status.setAttribute('role', 'status');
      preview = el('div', null, 'gr-preview'); list = el('div', null, 'gr-scroll'); footer = el('footer');
      search = el('input'); search.type = 'search'; search.placeholder = '搜索全群昵称'; search.setAttribute('aria-label', '搜索群成员');
      search.addEventListener('input', () => { query = search.value; selected.clear(); clearTimeout(searchTimer); searchTimer = setTimeout(() => load(false, true), 250); });
      const filters = el('div', null, 'gr-filters'); filterButtons = ['all', 'admins', 'recent'].map((key, index) => {
        const node = button(['全部', '群主与管理员', '最近加入'][index], () => { filter = key; selected.clear(); load(false, true); }); node.dataset.filter = key; filters.append(node); return node;
      });
      tools.append(search, filters); host.append(head, preview, tools, status, list, footer);
      if (wide.matches) { document.querySelector('#chatPage').append(host); document.querySelector('#chatPage').classList.add('gr-with-members'); document.querySelector('.app')?.classList.add('gr-group-layout'); }
      else { document.body.append(host); host.addEventListener('cancel', event => { event.preventDefault(); dismiss(); }); host.showModal(); }
      load(false, true);
    }
    async function load(more = false, reset = false) {
      if (!host || options.getUserId() !== owner || options.getGroup()?.id !== group || (more && !data?.next_cursor)) return;
      if (busy && !reset) return;
      controller?.abort(); controller = new AbortController(); const signal = controller.signal, generation = ++serial; busy = true;
      clearTimeout(refreshTimer); if (reset) { data = null; list.replaceChildren(); status.textContent = '正在加载群成员…'; }
      const params = new URLSearchParams({ q: query.trim(), filter }); if (more) params.set('cursor', data.next_cursor);
      try {
        const page = await read(endpoint(`roster?${params}`), { signal, cache: 'no-store' });
        if (signal.aborted || generation !== serial || !host) return;
        if (!Array.isArray(page.members)) throw new Error('成员名单格式异常');
        const previous = data;
        if (more) data = { ...page, members: [...new Map([...data.members, ...page.members].map(member => [member.id, member])).values()] };
        else if (data?.revision === page.revision) data = { ...data, pending_count: page.pending_count };
        else { data = page; selected.clear(); if (previous) { detail?.close(); detail = null; } }
        render();
      } catch (error) {
        if (!signal.aborted && generation === serial && host) {
          if ([401, 403, 404].includes(error.status)) { data = null; selected.clear(); preview.replaceChildren(); tools.hidden = true; list.replaceChildren(); footer.replaceChildren(); head.replaceChildren(el('strong', '群成员'), button('关闭', dismiss)); detail?.close(); }
          status.replaceChildren(el('span', error.message), button('重新加载', () => load(false, true)));
          if (error.status === 409) { busy = false; load(false, true); }
        }
      } finally { if (generation === serial) { busy = false; if (host) refreshTimer = setTimeout(() => { if (!document.hidden) load(); else refreshTimer = setTimeout(() => load(), 15000); }, 15000); } }
    }
    function render() {
      if (!data || !host) return;
      head.replaceChildren(el('strong', `${directory ? '群成员' : '群聊信息'}（${data.total_count}）`), button(wide.matches ? '收起' : '关闭', dismiss));
      preview.replaceChildren(); preview.hidden = directory; tools.hidden = !directory; list.hidden = !directory;
      if (!directory) {
        const grid = el('div', null, 'gr-grid'); data.members.slice(0, 15).forEach(member => {
          const tile = button('', () => profile(member), 'gr-tile'); tile.append(avatar(member), el('span', member.display_name)); grid.append(tile);
        });
        preview.append(grid, button(`查看全部成员（${data.total_count}）`, () => { directory = true; render(); }, 'gr-all'));
      }
      filterButtons.forEach(node => node.setAttribute('aria-pressed', String(node.dataset.filter === filter)));
      status.textContent = query || filter !== 'all' ? `找到 ${data.matched_count} 人 · 全群 ${data.total_count} 人` : `${data.name} · 共 ${data.total_count} 位成员`;
      const position = list.scrollTop; list.replaceChildren();
      if (!data.members.length) list.append(el('p', query || filter !== 'all' ? '未找到匹配的群成员' : '暂无群成员'));
      data.members.forEach(member => {
        const row = el('div', null, 'gr-row');
        if (managing && removable(member)) {
          const checkbox = el('input'); checkbox.type = 'checkbox'; checkbox.checked = selected.has(member.id); checkbox.setAttribute('aria-label', `选择${member.display_name}`);
          checkbox.disabled = !checkbox.checked && selected.size >= 100;
          checkbox.addEventListener('change', () => { if (checkbox.checked) selected.add(member.id); else selected.delete(member.id); render(); }); row.append(checkbox);
        }
        const person = button('', () => profile(member), 'gr-person'); const copy = el('span', null, 'gr-copy');
        copy.append(el('span', `${member.display_name}${member.id === data.viewer_id ? '（我）' : ''}`), el('small', member.role === 'member' ? `入群 ${member.joined_at.slice(0, 10)}` : roleName(member.role)));
        person.append(avatar(member), copy); row.append(person); list.append(row);
      });
      if (data.next_cursor) { next = button(`加载更多（${data.members.length}/${data.matched_count}）`, () => load(true), 'gr-all'); list.append(next); }
      list.scrollTop = position; footer.replaceChildren();
      if (data.permissions.invite) footer.append(button('邀请成员', invite));
      if (data.permissions.manage) {
        footer.append(button(managing ? '完成管理' : '管理成员', () => { managing = !managing; directory = true; selected.clear(); render(); }));
        footer.append(button(`待审邀请（${data.pending_count}）`, () => invitations(0)));
        if (managing) { const remove = button(`移出所选成员（${selected.size}）`, () => confirm('移出所选成员', `将 ${data.members.filter(member => selected.has(member.id)).map(member => member.display_name).join('、')} 移出群聊。`, { action: 'remove', user_ids: [...selected] }), 'gr-danger'); remove.disabled = !selected.size; footer.append(remove); }
      }
      if (data.permissions.owner) footer.append(button('邀请规则', policy));
      footer.append(button('刷新', () => load(false, true)), button('退出群聊', () => confirm('退出群聊', data.viewer_role === 'owner' && data.total_count > 1 ? '请先转让群主，再退出群聊。' : `退出 ${data.name} 后将不再接收群消息。`, { action: 'leave' }), 'gr-danger'));
      if (data.permissions.owner) footer.append(button('解散群聊', () => confirm('解散群聊', `全部 ${data.total_count} 位成员将失去群聊访问权限。此操作无法撤销。`, { action: 'dissolve' }), 'gr-danger'));
    }
    function removable(member) { return member.id !== data.viewer_id && member.role !== 'owner' && (data.permissions.owner || (data.permissions.manage && member.role === 'member')); }
    function child(title) { detail?.close(); const result = modal(title); detail = result.dialog; return result; }
    function confirm(title, explanation, command) {
      const { dialog, body } = child(title), requestId = Array.from(crypto.getRandomValues(new Uint8Array(16)), byte => byte.toString(16).padStart(2, '0')).join(''), target = group, user = owner;
      body.append(el('p', explanation)); const result = el('p'); result.setAttribute('role', 'status'); let sending = false;
      const submit = button(title, async () => {
        if (sending || options.getUserId() !== user || options.getGroup()?.id !== target) return;
        sending = true; submit.disabled = true; result.textContent = '正在处理…';
        dialog.querySelector('header button').disabled = true;
        try {
          const receipt = await read(`/api/me/groups/${encodeURIComponent(target)}/membership`, { method: 'POST', body: JSON.stringify({ ...command, request_id: requestId }) });
          if (!dialog.isConnected) return;
          dialog.close(); selected.clear(); options.onChanged();
          if (receipt.exited) { dismiss(); options.onExit(); } else { load(false, true); options.onNotice?.(receipt.message); }
        } catch (error) { result.textContent = `${error.message}。可重试此操作。`; }
        finally { sending = false; submit.disabled = false; const closeButton = dialog.querySelector('header button'); if (closeButton) closeButton.disabled = false; }
      });
      dialog.addEventListener('cancel', event => { if (sending) event.preventDefault(); }); body.append(result, submit);
    }
    function profile(member) {
      const { body, dialog } = child('群成员资料'); const title = el('div', null, 'gr-profile'); title.append(avatar(member), el('strong', member.display_name), el('span', roleName(member.role)));
      body.append(title, el('p', `入群时间：${member.joined_at.slice(0, 10)}`));
      if (member.id !== data.viewer_id) {
        body.append(button('在群里 @TA', () => { dialog.close(); if (!wide.matches) dismiss(); options.onMention(member); }));
        const feedback = el('p'); const dm = button('添加好友并私聊', async () => {
          dm.disabled = true;
          try { await read('/api/me/friends', { method: 'POST', body: JSON.stringify({ query: member.id, search_type: 'user_id' }) }); if (dialog.isConnected) { dismiss(); options.onMessage(member); } }
          catch (error) { feedback.textContent = error.message; } finally { dm.disabled = false; }
        }); body.append(dm, feedback);
        if (data.permissions.owner) body.append(button(member.role === 'admin' ? '取消管理员' : '设为管理员', () => confirm(member.role === 'admin' ? '取消管理员' : '设为管理员', `修改 ${member.display_name} 的群管理权限。`, { action: 'role', user_ids: [member.id], role: member.role === 'admin' ? 'member' : 'admin' })), button('转让群主', () => confirm('转让群主', `将群主转让给 ${member.display_name}，你将成为普通成员。`, { action: 'transfer', user_ids: [member.id] })));
        if (removable(member)) body.append(button('移出群聊', () => confirm('移出群聊', `将 ${member.display_name} 移出群聊。`, { action: 'remove', user_ids: [member.id] }), 'gr-danger'));
      }
    }
    async function invite() {
      const { body, dialog } = child('邀请群成员'); body.append(el('p', '正在加载好友…'));
      try {
        const result = await read('/api/me/friends', { cache: 'no-store' }); if (!dialog.isConnected) return;
        const friends = result.friends || [], choices = new Set(); const input = el('input'); input.type = 'search'; input.placeholder = '搜索好友'; input.setAttribute('aria-label', '搜索好友');
        const rows = el('div', null, 'gr-invite-list'); const submit = button('邀请（0）', () => confirm('邀请成员', `${data.viewer_role === 'member' && data.invitation_policy === 'approval' ? '提交审核，邀请' : '邀请'} ${friends.filter(friend => choices.has(friend.id)).map(friend => friend.nickname || friend.account).join('、')} 加入群聊。已在群中的好友不会重复加入。`, { action: 'invite', user_ids: [...choices] }));
        submit.disabled = true; body.replaceChildren(input, el('p', '最多选择 100 位好友。'), rows, submit);
        function show() {
          rows.replaceChildren(); const visible = friends.filter(friend => (friend.nickname || friend.account || '').toLowerCase().includes(input.value.toLowerCase()));
          if (!visible.length) rows.append(el('p', friends.length ? '未找到匹配的好友' : '暂无好友，请先添加好友'));
          visible.forEach(friend => { const row = el('label', null, 'gr-choice'), checkbox = el('input'); checkbox.type = 'checkbox'; checkbox.checked = choices.has(friend.id); checkbox.disabled = !checkbox.checked && choices.size >= 100;
            checkbox.addEventListener('change', () => { if (checkbox.checked) choices.add(friend.id); else choices.delete(friend.id); submit.textContent = `邀请（${choices.size}）`; submit.disabled = !choices.size; show(); }); row.append(checkbox, el('span', friend.nickname || friend.account)); rows.append(row); });
        }
        input.addEventListener('input', show); show();
      } catch (error) { if (dialog.isConnected) body.replaceChildren(el('p', error.message), button('重新加载', invite)); }
    }
    function policy() {
      const { body } = child('成员邀请规则'); [['members', '所有成员可邀请好友'], ['admins', '仅群主和管理员可邀请'], ['approval', '普通成员邀请需审核']].forEach(([value, label]) => {
        body.append(button(`${data.invitation_policy === value ? '当前：' : ''}${label}`, () => confirm('更新邀请规则', `邀请规则将改为“${label}”。`, { action: 'policy', invitation_policy: value })));
      });
    }
    async function invitations(offset) {
      const { body, dialog } = child('待审邀请'); body.append(el('p', '正在加载…'));
      try {
        const result = await read(endpoint(`invitations?offset=${offset}`), { cache: 'no-store' }); if (!dialog.isConnected) return;
        body.replaceChildren(el('p', result.total_count ? `共 ${result.total_count} 条待审邀请` : '暂无待审邀请'));
        result.requests.forEach(request => {
          const copy = `${request.actor_name} 邀请 ${request.members.map(member => member.display_name).join('、')}`;
          body.append(el('p', copy), button('通过', () => confirm('通过邀请', copy, { action: 'approve', invitation_id: request.id })), button('拒绝', () => confirm('拒绝邀请', copy, { action: 'reject', invitation_id: request.id })));
        });
        if (offset) body.append(button('上一页', () => invitations(Math.max(0, offset - 50))));
        if (result.next_offset != null) body.append(button('下一页', () => invitations(result.next_offset)));
      } catch (error) { if (dialog.isConnected) body.replaceChildren(el('p', error.message), button('重试', () => invitations(offset))); }
    }
    function sync() {
      const active = options.getGroup(), identity = active ? `${options.getUserId()}:${active.id}` : '';
      if (identity !== current) { close(); current = identity; collapsed = false; if (identity && wide.matches) open(true); }
    }
    const watcher = setInterval(sync, 1000);
    const resume = () => { sync(); if (host && !document.hidden) load(); };
    window.addEventListener('focus', resume); window.addEventListener('online', resume); document.addEventListener('visibilitychange', resume);
    wide.addEventListener('change', () => { const wasOpen = !!host; close(); if (options.getGroup() && (wasOpen || (wide.matches && !collapsed))) open(wide.matches); });
    return { open, sync, close, destroy() { clearInterval(watcher); close(); window.removeEventListener('focus', resume); window.removeEventListener('online', resume); document.removeEventListener('visibilitychange', resume); } };
  }
  window.ElonGroupRoster = { install };
})();
