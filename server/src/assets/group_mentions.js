/* Native APK parity: group avatar mentions and searchable member / AI selection. */
(function (root) {
  'use strict';
  function insert(text, start, end, targets) {
    start = Math.max(0, Math.min(start, text.length));
    end = Math.max(start, Math.min(end, text.length));
    const unique = [...new Map(targets.map(item => [item.id, item])).values()];
    const prefix = start > 0 && !/\s/u.test(text[start - 1]) ? ' ' : '';
    const value = prefix + unique.map(item => '@' + item.display_name + ' ').join('');
    return { text: text.slice(0, start) + value + text.slice(end), cursor: start + value.length };
  }
  function trigger(text, at) {
    return /[@＠]/u.test(text[at] || '') && (at === 0 || /[\s，。！？：；、,!?;:([{（【]/u.test(text[at - 1]));
  }
  function filter(items, query) {
    const needle = query.trim().replace(/^[@＠]/u, '').toLocaleLowerCase();
    return items.filter(item => item.display_name.toLocaleLowerCase().includes(needle) ||
      (item.isAi && '群ai 一龙 助手 el'.includes(needle)));
  }
  function create({ input, getGroup, getSelfId, api }) {
    let active = null;
    function apply(edit) {
      input.value = edit.text;
      input.dispatchEvent(new Event('input', { bubbles: true }));
      input.focus();
      input.setSelectionRange(edit.cursor, edit.cursor);
    }
    function element(tag, className, text) {
      const value = document.createElement(tag);
      value.className = className;
      if (text !== undefined) value.textContent = text;
      return value;
    }
    function button(text, callback) {
      const value = element('button', '', text);
      value.type = 'button'; value.addEventListener('click', callback);
      return value;
    }
    function open(at) {
      const group = getGroup();
      if (!group || active) return;
      const snapshot = input.value;
      const selection = [input.selectionStart, input.selectionEnd];
      let pendingEdit = null;
      const controller = new AbortController();
      const dialog = element('dialog', 'group-mention-sheet');
      active = dialog;
      let items = [], multiple = false, selected = new Map();
      const panel = element('div', 'group-mention-panel');
      const header = element('div', 'group-mention-header');
      const title = element('h2', '', '选择提醒的人');
      title.id = 'group-mention-title'; dialog.setAttribute('aria-labelledby', title.id);
      const mode = button('多选', () => { multiple = !multiple; selected.clear(); render(); });
      const search = element('input', 'group-mention-search');
      search.type = 'search'; search.placeholder = '搜索群友或群 AI'; search.setAttribute('aria-label', search.placeholder);
      const status = element('div', 'group-mention-status', '正在加载群成员…');
      status.setAttribute('role', 'status');
      const list = element('div', 'group-mention-list');
      const confirm = button('完成', () => choose([...selected.values()]));
      confirm.className = 'group-mention-confirm'; confirm.hidden = true;
      header.append(button('取消', () => dialog.close()), title, mode);
      panel.append(header, search, status, list, confirm); dialog.append(panel);
      document.body.append(dialog);
      function choose(targets) {
        if (targets.length && getGroup()?.id === group.id && input.value === snapshot) {
          pendingEdit = insert(snapshot, at, at + 1, targets);
        }
        dialog.close();
      }
      function render() {
        list.replaceChildren();
        mode.textContent = multiple ? '单选' : '多选';
        confirm.hidden = !multiple; confirm.disabled = !selected.size;
        confirm.textContent = `完成（${selected.size}）`;
        const visible = filter(items, search.value);
        status.textContent = visible.length ? '' : '没有匹配的群友或群 AI';
        status.hidden = !!visible.length;
        visible.forEach(item => {
          const row = button('', () => {
            if (!multiple) return choose([item]);
            if (selected.has(item.id)) selected.delete(item.id); else selected.set(item.id, item);
            render();
          });
          row.className = 'group-mention-row';
          row.setAttribute('aria-label', item.display_name + ' ' + (item.isAi ? '群 AI · 一龙助手' : '群友'));
          row.setAttribute('aria-pressed', String(selected.has(item.id)));
          const avatar = element('span', 'group-mention-avatar', item.isAi ? 'EL' : [...item.display_name][0]);
          if (/^data:image\/(png|jpeg|webp|gif);base64,/iu.test(item.avatar_data_url || '')) {
            const image = element('img', ''); image.src = item.avatar_data_url; image.alt = ''; avatar.replaceChildren(image);
          }
          const label = element('span', 'group-mention-label');
          label.append(element('span', '', item.display_name), element('small', '', item.isAi ? '群 AI · 一龙助手' : '群友'));
          row.append(avatar, label);
          if (multiple) row.append(element('span', 'group-mention-check', selected.has(item.id) ? '✓' : '○'));
          list.append(row);
        });
      }
      async function load() {
        status.hidden = false; status.textContent = '正在加载群成员…'; search.disabled = mode.disabled = true;
        try {
          const response = await api('/api/me/groups/' + encodeURIComponent(group.id) + '/members', { signal: controller.signal });
          const data = await response.json();
          if (!response.ok) throw new Error('load failed');
          if (active !== dialog || getGroup()?.id !== group.id) { dialog.close(); return; }
          const aiIds = new Set((data.ai_members || []).map(item => item.id));
          const people = (data.members || []).filter(item => item.id !== getSelfId() && !aiIds.has(item.id));
          people.sort((a, b) => a.display_name.localeCompare(b.display_name, 'zh-CN'));
          items = [...new Map([...(data.ai_members || []).map(item => ({ ...item, isAi: true })), ...people]
            .filter(item => item.id && item.display_name).map(item => [item.id, item])).values()];
          search.disabled = mode.disabled = false; render();
        } catch (error) {
          if (controller.signal.aborted) return;
          status.replaceChildren(element('p', '', '群成员加载失败，请重试'), button('重新加载', load));
        }
      }
      search.addEventListener('input', render);
      dialog.addEventListener('close', () => {
        controller.abort(); active = null; dialog.remove();
        if (getGroup()?.id === group.id && input.value === snapshot) {
          if (pendingEdit) apply(pendingEdit);
          else { input.focus(); input.setSelectionRange(...selection); }
        }
      }, { once: true });
      dialog.addEventListener('click', event => { if (event.target === dialog) dialog.close(); });
      dialog.showModal(); load();
    }
    input.addEventListener('input', event => {
      if (!event.isComposing && (event.data === '@' || event.data === '＠')) {
        const at = input.selectionStart - 1;
        if (trigger(input.value, at)) open(at);
      }
    });
    return {
      bindAvatar(avatar, target, groupId) {
        if (!avatar || !target?.id || target.id === getSelfId()) return;
        let timer, origin, fired = false;
        const cancel = () => clearTimeout(timer);
        const mention = () => {
          cancel();
          if (fired || !avatar.isConnected || getGroup()?.id !== groupId) return;
          fired = true;
          apply(insert(input.value, input.selectionStart, input.selectionEnd, [target]));
        };
        avatar.tabIndex = 0; avatar.setAttribute('role', 'button');
        avatar.setAttribute('aria-label', target.display_name + '，长按提及');
        avatar.addEventListener('pointerdown', event => {
          if (event.button !== 0) return;
          fired = false; origin = [event.clientX, event.clientY]; timer = setTimeout(mention, 500);
        });
        avatar.addEventListener('pointermove', event => {
          if (origin && Math.hypot(event.clientX - origin[0], event.clientY - origin[1]) > 10) cancel();
        });
        ['pointerup', 'pointercancel', 'pointerleave'].forEach(name => avatar.addEventListener(name, cancel));
        avatar.addEventListener('contextmenu', event => { event.preventDefault(); mention(); });
        avatar.addEventListener('keydown', event => {
          if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); fired = false; mention(); }
        });
      },
      close() { active?.close(); },
    };
  }
  root.ElonGroupMentions = { create, insert, trigger, filter };
  if (typeof module !== 'undefined' && module.exports) module.exports = root.ElonGroupMentions;
})(globalThis);
