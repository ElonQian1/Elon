(function (root) {
  'use strict';
  let currentGroup = '', currentMessages = new Map(), activeDialog = null, noticeTimer;
  const revision = message => Number(message.revision || 1);
  const path = (group, message) => '/api/me/groups/' + encodeURIComponent(group) + '/messages/' + encodeURIComponent(message);
  function element(tag, text, className) {
    const node = document.createElement(tag);
    if (text != null) node.textContent = text;
    if (className) node.className = className;
    return node;
  }
  function button(text, action) { const node = element('button', text); node.type = 'button'; node.onclick = action; return node; }
  function changedText(before, after) {
    const a = Array.from(before), b = Array.from(after);
    let start = 0, end = 0;
    while (start < a.length && start < b.length && a[start] === b[start]) start++;
    while (end < a.length - start && end < b.length - start && a[a.length - end - 1] === b[b.length - end - 1]) end++;
    return { removed: a.slice(start, a.length - end).join(''), added: b.slice(start, b.length - end).join('') };
  }
  function reset() {
    currentGroup = ''; currentMessages.clear();
    if (activeDialog) activeDialog.close();
  }
  function reconcile(group, messages) {
    if (currentGroup !== group) { reset(); currentGroup = group; }
    let edited = false;
    const merged = messages.map(message => {
      const previous = currentMessages.get(message.id);
      if (previous && !message.recalled_at && (previous.recalled_at || revision(previous) > revision(message))) return previous;
      if (previous && revision(message) > revision(previous)) edited = true;
      return message;
    });
    currentMessages = new Map(merged.map(message => [message.id, message]));
    if (activeDialog && currentMessages.get(activeDialog.dataset.messageId)?.recalled_at) activeDialog.close();
    if (edited) notify('群聊文字已更新，可点击“已编辑”查看修改记录');
    return merged;
  }
  function notify(text) {
    let notice = document.getElementById('group-revision-notice');
    if (!notice) { notice = element('div', '', 'group-revision-notice'); notice.id = 'group-revision-notice'; notice.setAttribute('role', 'status'); document.body.append(notice); }
    notice.textContent = text; notice.hidden = false;
    clearTimeout(noticeTimer); noticeTimer = setTimeout(() => { notice.hidden = true; }, 6000);
  }
  async function request(api, url, options) {
    const response = await api(url, options);
    const data = await response.json().catch(() => ({}));
    if (!response.ok) throw Object.assign(new Error(data.error || '请求失败，请重试'), { status: response.status });
    return data;
  }
  function modal(title, messageId) {
    if (activeDialog) activeDialog.close();
    const dialog = element('dialog', null, 'group-revision-dialog');
    dialog.dataset.messageId = messageId;
    const header = element('header'), heading = element('h2', title), body = element('div', null, 'group-revision-body');
    heading.id = 'group-revision-dialog-title'; dialog.setAttribute('aria-labelledby', heading.id);
    header.append(heading, button('关闭', () => dialog.close())); dialog.append(header, body);
    dialog.addEventListener('close', () => { if (activeDialog === dialog) activeDialog = null; dialog.remove(); });
    document.body.append(dialog); activeDialog = dialog;
    return { dialog, body };
  }
  async function history(group, message, api) {
    const { dialog, body } = modal('修改记录', message.id);
    body.append(element('p', '按最新到最早排列，每一版均保留完整文字。', 'group-revision-hint'));
    const list = element('div'), status = element('p'); status.setAttribute('role', 'status');
    let before = null, versions = [], busy = false;
    const more = button('查看更早版本', () => load());
    body.append(list, status, more); dialog.showModal();
    async function load() {
      if (busy) return;
      busy = true; more.disabled = true; status.textContent = '正在读取…';
      try {
        const data = await request(api, path(group, message.id) + '/revisions?limit=20' + (before ? '&before_revision=' + before : ''));
        if (!dialog.open) return;
        versions.push(...data.revisions); before = data.next_before_revision; list.replaceChildren();
        versions.forEach((version, index) => {
          const item = element('article');
          item.append(element('h3', '第 ' + version.revision + ' 版' + (version.revision === 1 ? ' · 原始文字' : '')), element('time', new Date(version.created_at).toLocaleString()), element('pre', version.content));
          if (versions[index + 1]) {
            const diff = changedText(versions[index + 1].content, version.content), details = element('details');
            details.append(element('summary', '与上一版相比'));
            if (diff.removed) { const row = element('p', '删除：'); row.append(element('del', diff.removed)); details.append(row); }
            if (diff.added) { const row = element('p', '新增：'); row.append(element('ins', diff.added)); details.append(row); }
            item.append(details);
          }
          list.append(item);
        });
        status.textContent = before ? '' : '已显示全部 ' + versions.length + ' 个版本';
        more.hidden = !before; more.textContent = '查看更早版本';
      } catch (error) {
        if (!dialog.open) return;
        if ([403, 404, 410].includes(error.status)) { list.replaceChildren(); versions = []; before = null; }
        status.textContent = error.message; more.hidden = false; more.textContent = '重新读取';
      } finally { busy = false; more.disabled = false; }
    }
    await load();
  }
  function edit(group, message, api, refresh) {
    const { dialog, body } = modal('编辑消息', message.id);
    let expected = revision(message), busy = false, conflict = false;
    body.append(element('p', '保存后标记为已编辑，群成员可查看每一版文字。附件保持原样。', 'group-revision-hint'));
    const label = element('label', '消息文字'), input = element('textarea'), count = element('p', '', 'group-revision-hint'), status = element('p');
    input.id = 'group-revision-draft'; label.htmlFor = input.id; input.rows = 7; input.value = message.content; input.autofocus = true;
    status.setAttribute('role', 'status');
    const latest = element('section', null, 'group-revision-conflict'); latest.hidden = true;
    const save = button('保存修改', async () => {
      const content = input.value.trim();
      if (busy || conflict || !content || Array.from(content).length > 4000) return;
      busy = true; input.disabled = true; save.disabled = true; status.textContent = '正在保存…';
      dialog.querySelector('header button').disabled = true;
      try {
        const data = await request(api, path(group, message.id), { method: 'PATCH', body: JSON.stringify({ content, expected_revision: expected }) });
        if (!dialog.open) return;
        const previous = currentMessages.get(message.id);
        if (currentGroup === group && previous && revision(previous) <= revision(data.message) && !previous.recalled_at) currentMessages.set(message.id, Object.assign({}, previous, data.message));
        dialog.close(); notify('修改已保存，群成员可查看历史');
        Promise.resolve(refresh()).catch(() => notify('修改已保存，列表刷新失败，稍后将自动重试'));
      } catch (error) {
        if (!dialog.open) return;
        status.textContent = error.message + '；草稿已保留。';
        if (error.status === 409) {
          try {
            const data = await request(api, path(group, message.id) + '/revisions?limit=1');
            if (!dialog.open || !data.revisions[0]) return;
            const version = data.revisions[0]; conflict = true; latest.hidden = false; latest.replaceChildren();
            latest.append(element('strong', '其他设备已保存第 ' + version.revision + ' 版'), element('pre', version.content), button('已核对，继续编辑我的草稿', () => {
              expected = version.revision; conflict = false; latest.hidden = true; validate(); status.textContent = '已保留你的草稿，请核对后点击保存';
            }));
          } catch { /* Retain old expected revision; another save still detects the conflict. */ }
        }
      } finally { busy = false; input.disabled = false; dialog.querySelector('header button').disabled = false; validate(); }
    });
    function validate() { const length = Array.from(input.value.trim()).length; count.textContent = length + ' / 4000 字'; save.disabled = busy || conflict || length < 1 || length > 4000; }
    input.oninput = validate;
    dialog.addEventListener('cancel', event => { if (busy) event.preventDefault(); });
    body.append(label, input, count, latest, status);
    const footer = element('footer'); footer.append(save); dialog.append(footer); validate(); dialog.showModal();
  }
  function mount(bubble, group, message, api, refresh) {
    if (!bubble || message.recalled_at || message.recalledAt || !message.id) return;
    const actions = element('div', null, 'group-revision-actions');
    if (revision(message) > 1) actions.append(button('已编辑 · ' + (revision(message) - 1) + ' 次', () => history(group, message, api)));
    if (message.outgoing && message.content?.trim() && !message.content.startsWith('【一龙项目卡片】')) actions.append(button('编辑', () => edit(group, message, api, refresh)));
    bubble.append(actions);
  }
  root.ElonGroupMessageRevisions = { mount, reconcile, reset, changedText };
})(typeof window === 'undefined' ? globalThis : window);
