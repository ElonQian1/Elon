(function (root) {
  'use strict';
  function quotePreview(bubble, quote, own, list) {
    if (!quote) return;
    const button = document.createElement('button'); button.type = 'button'; button.title = '查看引用消息';
    button.style.cssText = `display:flex;align-items:center;gap:8px;max-width:min(100%,320px);min-height:48px;margin-top:6px;padding:4px 8px;border:0;border-${own ? 'right' : 'left'}:2px solid var(--line-soft,#555);border-radius:0;background:transparent;color:var(--text-secondary,#aaa);font:inherit;font-size:13px;line-height:1.5;text-align:left;${own ? 'margin-left:auto;' : ''}`;
    const label = document.createElement('span');
    label.style.cssText = 'min-width:0;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;overflow-wrap:anywhere';
    label.textContent = (quote.sender_name ? quote.sender_name + '：' : '') + (quote.unavailable ? '原消息已撤回或不可用' : quote.content || '[附件]');
    button.append(label);
    const cover = !quote.unavailable && quote.attachments?.find(a => a.kind === 'image' || a.mime_type?.startsWith('image/'))?.url;
    if (cover && (/^https?:\/\//i.test(cover) || /^\/(?!\/)/.test(cover))) {
      const image = document.createElement('img'); image.src = cover; image.alt = ''; image.loading = 'lazy'; image.referrerPolicy = 'no-referrer';
      image.style.cssText = 'width:40px;height:40px;flex:none;object-fit:cover;border-radius:3px';
      image.onerror = () => image.remove(); button.append(image);
    }
    button.onclick = () => {
      const original = Array.from(list.children).find(row => row.dataset.messageId === quote.message_id);
      if (original && !quote.unavailable) { original.scrollIntoView({ block: 'center', behavior: 'smooth' }); return; }
      const dialog = document.createElement('dialog'), text = document.createElement('p'), close = document.createElement('button');
      text.textContent = label.textContent; text.style.whiteSpace = 'pre-wrap'; close.textContent = '关闭'; close.onclick = () => dialog.close();
      dialog.append(text, close); dialog.onclose = () => dialog.remove(); document.body.append(dialog); dialog.showModal();
    };
    bubble.after(button);
  }
  function media(bubble, attachments) {
    (attachments || []).forEach(item => {
      const name = item.display_name || item.file_name || '附件';
      let url; try { url = new URL(item.url, location.origin); } catch { return; }
      if (!item.url || !['http:', 'https:'].includes(url.protocol)) return;
      if (location.protocol === 'https:' && url.hostname === location.hostname && /^\/api\/user\/[^/]+\/chat-attachments\//.test(url.pathname)) {
        url = new URL(url.pathname + url.search, location.origin);
      }
      const mime = item.mime_type || '', kind = item.kind || '';
      const box = document.createElement('div'); box.style.marginTop = '8px';
      const link = document.createElement('a'); link.href = url.href; link.textContent = '下载：' + name;
      link.target = '_blank'; link.rel = 'noopener noreferrer'; link.download = name;
      let player;
      if (kind === 'image' || mime.startsWith('image/')) {
        player = document.createElement('img'); player.alt = name; player.loading = 'lazy';
        player.style.cssText = 'max-width:100%;max-height:320px;display:block;border-radius:8px';
      } else if (['audio', 'voice'].includes(kind) || mime.startsWith('audio/')) {
        if (root.ElonSocialVoice) { root.ElonSocialVoice.mount(box, item, url.href, link); bubble.append(box); return; }
        player = document.createElement('audio'); player.controls = true; player.preload = 'none';
        player.style.cssText = 'max-width:100%;display:block';
        player.setAttribute('aria-label', '播放语音：' + name);
        player.onplay = () => document.querySelectorAll('audio').forEach(other => { if (other !== player) other.pause(); });
      } else if (kind === 'video' || mime.startsWith('video/')) {
        player = document.createElement('video'); player.controls = true; player.preload = 'none'; player.playsInline = true;
        player.style.cssText = 'max-width:100%;max-height:320px;display:block';
        player.setAttribute('aria-label', '播放视频：' + name);
      }
      if (player) {
        const retry = document.createElement('button'); retry.type = 'button'; retry.hidden = true;
        retry.textContent = '加载失败 · 点击重试';
        retry.onclick = () => { retry.hidden = true; player.src = url.href; if (player.load) player.load(); };
        player.onerror = () => { retry.hidden = false; };
        player.src = url.href; box.append(player, retry);
      }
      box.append(link); bubble.append(box);
      if (kind === 'image' || mime.startsWith('image/')) { root.ElonSourceLinks?.image(box, item, url.href); root.ElonSocialImageViewer?.bind(player, item, url.href); }
    });
  }
  function create(options) {
    let scope = '', nodes = new Map();
    return {
      render(messages, kind, contact, scroll) {
        const list = options.list, key = kind + ':' + contact.id;
        options.actions?.update(messages, kind, contact);
        const previousScroll = list.scrollTop, follow = scroll || list.scrollHeight - list.clientHeight - previousScroll < 70;
        if (scope !== key) { nodes.forEach(entry => entry.cleanup?.()); root.ElonSocialImageViewer?.close(); root.ElonSocialLinkViewer?.close(); root.ElonAiConversationShare?.reset(); list.replaceChildren(); nodes.clear(); scope = key; }
        if (kind === 'group') root.ElonAiConversationReader?.reconcile(contact.id, messages);
        options.resetTimeline();
        const next = new Map(); let cursor = list.firstChild;
        messages.forEach(msg => {
          const id = msg.id || msg.client_id;
          const senderId = msg.sender_user_id || '';
          const source = kind === 'friend' ? contact : contact.members?.find(member => member.id === senderId);
          const avatar = msg.sender_avatar_data_url || source?.avatar_data_url || source?.avatarDataUrl;
          const signature = JSON.stringify([msg, avatar]);
          let entry = nodes.get(id);
          if (!entry || entry.signature !== signature) {
            entry?.cleanup?.();
            const outgoing = !!msg.outgoing, recalled = !!(msg.recalled_at || msg.recalledAt);
            const text = recalled ? (outgoing ? '你撤回了一条消息' : (msg.sender_name || '对方') + ' 撤回了一条消息') : msg.content || '';
            const bubble = options.append(outgoing ? 'user' : senderId === 'usr_elon_ai' ? 'ai' : 'friend', text, null, null, null, {
              createdAtMs: options.time(msg.created_at) || Date.now(),
              mentionTarget: kind === 'group' && !outgoing && !recalled ? { id: senderId, display_name: senderId === 'usr_elon_ai' ? 'EL' : msg.sender_name } : null,
              senderName: msg.sender_name || (outgoing ? (options.user()?.nickname || options.user()?.account || '我') : source ? options.friendName(source) : '群成员'), avatarDataUrl: avatar,
              avatarFallback: outgoing ? (options.user()?.nickname || options.user()?.account || '我') : (msg.sender_name || options.friendName(contact)),
            });
            const compactLink = !recalled && !msg.attachments?.length && root.ElonSocialLinks?.prepareBubble(bubble, text);
            const recordOwner = options.user()?.id;
            const shared = !recalled && kind === 'group' && (root.ElonChatRecords?.mount(bubble, text, { api: options.api, group: contact.id, owner: recordOwner, current: () => scope === key && options.user()?.id === recordOwner }) || root.ElonAiConversationShare?.mount(bubble, msg, { api: options.api, groupId: contact.id, list, isCurrent: () => scope === key }));
            if (!recalled && !shared) { root.ElonArticles.mount(bubble, text, options.api, options.user()?.id); media(bubble, msg.attachments); if (!msg.attachments?.length) root.ElonSourceLinks?.text(bubble, text); }
            if (kind === 'group' && msg.id && !shared) root.ElonGroupMessageRevisions.mount(bubble, contact.id, msg, options.api, () => options.changed(contact.id));
            if (kind === 'group' && !recalled) root.ElonGroupAiReplyContext?.mount(bubble, msg, { api: options.api, group: contact.id, owner: options.user()?.id, list, current: () => scope === key && options.user()?.id === owner, changed: () => options.changed(contact.id) }, media);
            if (msg.send_status) { const status = document.createElement('small'); status.textContent = msg.send_status; status.style.display = 'block'; bubble.append(status); }
            const owner = options.user()?.id, cleanup = !recalled && !shared && root.ElonSocialLinks?.mount(bubble, text, { api: options.api, owner, compact: !!compactLink, pwaHandoff: true, isCurrent: () => scope === key && options.user()?.id === owner });
            if (!recalled) quotePreview(bubble, msg.quote, outgoing, list);
            const block = bubble.closest('.chat-message-block'), actionCleanup = options.actions?.bind(block, msg);
            entry = { signature, block, cleanup: () => { if (typeof cleanup === 'function') cleanup(); actionCleanup?.(); } };
            entry.block.dataset.messageId = id;
          }
          if (entry.block !== cursor) list.insertBefore(entry.block, cursor);
          cursor = entry.block.nextSibling; next.set(id, entry);
        });
        const keep = new Set(Array.from(next.values(), entry => entry.block));
        Array.from(list.children).forEach(node => { if (!keep.has(node)) node.remove(); });
        nodes.forEach((entry, id) => { if (!next.has(id)) entry.cleanup?.(); }); nodes = next; root.ElonAiConversationShare?.prune(); list.scrollTop = follow ? list.scrollHeight : previousScroll;
      },
      reset() { options.actions?.reset(); root.ElonSocialImageViewer?.close(); root.ElonSocialVoice?.stopWithin(options.list); nodes.forEach(entry => entry.cleanup?.()); root.ElonSocialLinkViewer?.close(); root.ElonAiConversationShare?.reset(); scope = ''; nodes.clear(); },
    };
  }
  root.ElonSocialChatView = { create };
})(globalThis);
