/* Selected-message reader. The group DOM and composer stay mounted underneath the modal. */
(function (root) {
  'use strict';
  const { el, button } = root.ElonAiShareRich;
  let active = null;
  function open(options) {
    if (active) { active.dialog.focus(); return; }
    const share = root.ElonAiConversationShare, { api, ref, list, senderName } = options;
    const previousFocus = document.activeElement, scroll = list?.scrollTop;
    const dialog = el('dialog', null, 'ai-share-reader'); dialog.tabIndex = -1;
    dialog.setAttribute('aria-labelledby', 'ai-share-reader-title');
    const header = el('header', null, 'ai-share-reader-header'), heading = el('h2', 'AI 对话片段');
    heading.id = 'ai-share-reader-title';
    const title = el('div', null, 'ai-share-reader-heading'), source = el('small', ''); title.append(heading, source);
    const searchBar = el('div', null, 'ai-share-search'); searchBar.hidden = true;
    const input = el('input'); input.type = 'search'; input.maxLength = 120;
    input.placeholder = '搜索分享内容'; input.setAttribute('aria-label', '搜索分享内容');
    const count = el('output'); count.setAttribute('aria-live', 'polite');
    const content = el('div', null, 'ai-share-reader-content'); content.tabIndex = 0;
    const status = el('p', '', 'ai-share-reader-status'); status.setAttribute('role', 'status');
    const retry = button('重新读取', () => void load(), 'retry'); retry.hidden = true;
    const state = el('div', null, 'ai-share-reader-state'); state.append(status, retry);
    const historyKey = 'ai-share-' + Date.now() + '-' + Math.random().toString(36).slice(2);
    let controller, media, timer, searchTimer, disposed = false, loaded = false, busy = false, deniedStatus = 0, rows = [];
    const session = { dialog, close, groupId: ref.group_id, messageId: options.messageId,
      revoke: () => { controller?.abort(); clearContent(404); status.textContent = share.errorText(404); retry.hidden = true; } }; active = session;
    function restoreGroup() {
      if (options.isCurrent && !options.isCurrent()) return;
      if (previousFocus?.isConnected) previousFocus.focus({ preventScroll: true });
      if (list?.isConnected && scroll != null) list.scrollTop = scroll;
    }
    function cleanup() {
      if (disposed) return; disposed = true; controller?.abort(); media?.dispose();
      clearInterval(timer); clearTimeout(searchTimer); rows = [];
      root.removeEventListener('popstate', onPop, true); root.removeEventListener('focus', wake);
      document.removeEventListener('visibilitychange', wake);
      if (dialog.open) dialog.close(); dialog.remove(); if (active === session) active = null;
      restoreGroup(); requestAnimationFrame(restoreGroup);
    }
    function close() {
      const owned = history.state?.elonAiSnapshot === historyKey;
      cleanup(); if (owned) history.back();
    }
    function onPop(event) { event.stopImmediatePropagation(); cleanup(); }
    function wake() { if (document.visibilityState !== 'hidden') void load(true); }
    function clearContent(code) {
      deniedStatus = code;
      media?.dispose(); media = null; rows = []; loaded = false;
      content.replaceChildren(); heading.textContent = 'AI 对话片段'; source.textContent = '';
      input.value = ''; searchBar.hidden = true; search.disabled = true; count.textContent = '';
      share.invalidate(ref, code);
    }
    function searchContent() {
      const query = input.value.trim().toLocaleLowerCase(); let matches = 0;
      content.querySelectorAll('mark').forEach(mark => mark.replaceWith(document.createTextNode(mark.textContent)));
      rows.forEach(row => {
        row.normalize(); row.hidden = !!query && !row.textContent.toLocaleLowerCase().includes(query);
        if (row.hidden || !query) return;
        matches++;
        const walker = document.createTreeWalker(row, NodeFilter.SHOW_TEXT), texts = [];
        while (walker.nextNode()) texts.push(walker.currentNode);
        texts.forEach(text => {
          const value = text.nodeValue, lower = value.toLocaleLowerCase(); let index = lower.indexOf(query), from = 0;
          if (index < 0) return;
          const fragment = document.createDocumentFragment();
          while (index >= 0) {
            fragment.append(document.createTextNode(value.slice(from, index)), el('mark', value.slice(index, index + query.length)));
            from = index + query.length; index = lower.indexOf(query, from);
          }
          fragment.append(document.createTextNode(value.slice(from))); text.replaceWith(fragment);
        });
      });
      count.textContent = query ? (matches ? matches + ' 条匹配' : '没有匹配内容') : '';
      content.scrollTop = 0;
    }
    const search = button('搜索分享内容', () => {
      searchBar.hidden = !searchBar.hidden; search.setAttribute('aria-expanded', String(!searchBar.hidden));
      if (!searchBar.hidden) input.focus(); else { input.value = ''; searchContent(); }
    }, 'search'); search.disabled = true; search.setAttribute('aria-expanded', 'false');
    const clear = button('清除搜索', () => { input.value = ''; searchContent(); input.focus(); }, 'close');
    input.addEventListener('input', () => { clearTimeout(searchTimer); searchTimer = setTimeout(searchContent, 120); });
    searchBar.append(input, clear, count); header.append(button('返回群聊', close, 'back'), title, search);
    dialog.append(header, searchBar, state, content);
    dialog.addEventListener('cancel', event => { event.preventDefault(); close(); });
    dialog.addEventListener('close', cleanup);
    document.body.append(dialog); dialog.showModal();
    history.pushState({ ...history.state, elonAiSnapshot: historyKey }, '');
    root.addEventListener('popstate', onPop, true); root.addEventListener('focus', wake);
    document.addEventListener('visibilitychange', wake);
    async function load(background = false) {
      if (disposed || busy || [404, 410].includes(deniedStatus)) return;
      deniedStatus = 0;
      busy = true; retry.disabled = true; controller = new AbortController();
      const request = controller, timeout = setTimeout(() => request.abort(), 20000);
      if (!loaded) { status.textContent = '正在读取分享…'; content.setAttribute('aria-busy', 'true'); }
      try {
        const view = await share.read(api, ref, request.signal);
        if (disposed || request.signal.aborted) return;
        if (!loaded) {
          const doc = view.document;
          heading.textContent = doc.title || 'AI 对话片段';
          const owner = typeof view.owner_name === 'string' && view.owner_name ? view.owner_name : senderName || '分享者';
          source.textContent = doc.messages.length + ' 条消息 · ChatGPT · ' + owner;
          media = share.mediaScope(api, ref, code => {
            if (disposed) return;
            request.abort(); clearContent(code); status.textContent = share.errorText(code); retry.hidden = ![401, 403].includes(code);
          });
          rows = doc.messages.map(message => root.ElonAiShareRich.message(message, part => media.image(part), owner));
          content.replaceChildren(...rows); loaded = true; search.disabled = !rows.length;
          if (input.value) searchContent();
          status.textContent = rows.length ? '' : '没有可阅读的分享内容';
        } else status.textContent = '';
        retry.hidden = true;
      } catch (error) {
        if (disposed) return;
        if (deniedStatus) return;
        if (share.denied(error.status)) clearContent(error.status);
        status.textContent = error.message || '读取失败，请检查网络后重试';
        if (request.signal.aborted) status.textContent = '请求超时，请重试';
        retry.hidden = [404, 410].includes(error.status);
        if (background && loaded) status.textContent = '暂时无法确认分享状态，请重新读取';
      } finally {
        clearTimeout(timeout); busy = false;
        if (!disposed) { retry.disabled = false; content.removeAttribute('aria-busy'); }
      }
    }
    timer = setInterval(() => { if (document.visibilityState !== 'hidden') void load(true); }, 30000);
    void load();
  }
  root.ElonAiConversationReader = { open, close: () => active?.close(), reconcile(group, messages) {
    if (!active || active.groupId !== group) return;
    const source = messages.find(message => message.id === active.messageId);
    if (source?.recalled_at || source?.recalledAt) active.revoke();
  } };
})(globalThis);
