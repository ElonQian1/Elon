(function (root) {
  'use strict';
  function mount(bubble, message, options, media) {
    const meta = message.ai_reply;
    if (!meta || meta.schema !== 1) return;
    const { el, button, markdown } = root.ElonAiShareRich;
    const path = '/api/me/groups/' + encodeURIComponent(options.group) + '/messages/' + encodeURIComponent(message.id);
    async function open(continuation) {
      const dialog = el('dialog', null, 'ai-share-reader');
      const content = el('div', null, 'ai-share-reader-content'), status = el('p', '正在读取记录…');
      const title = el('h2', continuation ? '使用 ChatGPT 继续讨论' : '群聊的聊天记录');
      const header = el('header', null, 'ai-share-reader-header');
      const focus = document.activeElement, scroll = options.list.scrollTop;
      function close() {
        clearInterval(guard); document.removeEventListener('visibilitychange', verify);
        dialog.close(); dialog.remove();
        if (options.current()) { options.list.scrollTop = scroll; focus?.focus({ preventScroll: true }); }
      }
      function verify() { if (!options.current()) close(); }
      const guard = setInterval(verify, 1000);
      document.addEventListener('visibilitychange', verify);
      header.append(button('返回群聊', close, 'back'), title); dialog.append(header, status, content);
      dialog.addEventListener('cancel', e => { e.preventDefault(); close(); });
      document.body.append(dialog); dialog.showModal();
      try {
        const data = await options.api(path + (continuation ? '/ai-context' : '/ai-sources'));
        if (!dialog.isConnected || !options.current()) { close(); return; }
        status.textContent = '';
        if (continuation) {
          content.append(el('p', '请在一龙 APK 或 Windows 客户端打开这条回答，使用自己的 ChatGPT 继续私人讨论。浏览器阅读页不会借用分享者的账号发送。'));
          data.document.messages.forEach(m => content.append(markdown(m.content)));
          return;
        }
        if (data.requester_id === options.owner && data.provider === 'chatgpt_web') {
          const label = el('label'), check = el('input'); check.type = 'checkbox'; check.checked = data.allow_continue;
          label.append(check, document.createTextNode('允许群成员使用自己的 ChatGPT 继续讨论')); content.append(label);
          check.onchange = async () => {
            check.disabled = true;
            try {
              if (!options.current()) return close();
              const updated = await options.api(path + '/ai-sources', { method: 'PATCH', body: JSON.stringify({ allow_continue: check.checked, version: data.version }) });
              data.version = updated.version; data.allow_continue = updated.allow_continue;
              status.textContent = '分享设置已更新'; options.changed();
            } catch { check.checked = data.allow_continue; status.textContent = '设置未确认，请重新打开后重试'; }
            finally { check.disabled = false; }
          };
        }
        data.sources.forEach(source => {
          const row = el('article', null, 'ai-share-message');
          row.append(el('strong', source.sender_name), el('time', ' · ' + new Date(source.created_at).toLocaleString()), markdown(source.content));
          if (!source.recalled_at) { media(row, source.attachments); root.ElonSourceLinks?.text(row, source.content); }
          content.append(row);
        });
      } catch { status.textContent = '记录未开放、已撤回或网络暂不可用，请关闭后重试'; }
    }
    if (meta.provider === 'chatgpt_web') bubble.append(button('使用 ChatGPT 继续讨论', () => void open(true), 'continue'));
    const card = el('button', null, 'ai-share-card group-ai-source-card'); card.type = 'button';
    card.append(el('strong', meta.source_count === 1 ? '引用的消息' : '群聊的聊天记录'));
    meta.previews.forEach(p => card.append(el('span', p.sender_name + '：' + (p.text || '[附件]'), 'ai-share-excerpt')));
    card.append(el('small', meta.source_count + ' 条来源消息')); card.onclick = () => void open(false);
    bubble.after(card);
  }
  root.ElonGroupAiReplyContext = { mount };
})(globalThis);
