(function (root) {
  'use strict';
  function media(bubble, attachments) {
    (attachments || []).forEach(item => {
      const name = item.display_name || item.file_name || '附件';
      let url; try { url = new URL(item.url, location.origin); } catch { return; }
      if (!item.url || !['http:', 'https:'].includes(url.protocol)) return;
      const mime = item.mime_type || '', kind = item.kind || '';
      const box = document.createElement('div'); box.style.marginTop = '8px';
      const link = document.createElement('a'); link.href = url.href; link.textContent = '下载：' + name;
      link.target = '_blank'; link.rel = 'noopener noreferrer'; link.download = name;
      let player;
      if (kind === 'image' || mime.startsWith('image/')) {
        player = document.createElement('img'); player.alt = name; player.loading = 'lazy';
        player.style.cssText = 'max-width:100%;max-height:320px;display:block;border-radius:8px';
      } else if (['audio', 'voice'].includes(kind) || mime.startsWith('audio/')) {
        player = document.createElement('audio'); player.controls = true; player.preload = 'none';
        player.style.cssText = 'max-width:100%;display:block';
        player.setAttribute('aria-label', '播放语音：' + name);
        player.onplay = () => document.querySelectorAll('audio').forEach(other => { if (other !== player) other.pause(); });
      }
      if (player) {
        const retry = document.createElement('button'); retry.type = 'button'; retry.hidden = true;
        retry.textContent = '加载失败 · 点击重试';
        retry.onclick = () => { retry.hidden = true; player.src = url.href; if (player.load) player.load(); };
        player.onerror = () => { retry.hidden = false; };
        player.src = url.href; box.append(player, retry);
      }
      box.append(link); bubble.append(box);
      if (kind === 'image' || mime.startsWith('image/')) root.ElonSourceLinks?.image(box, item, url.href);
    });
  }
  function create(options) {
    let scope = '', nodes = new Map();
    return {
      render(messages, kind, contact, scroll) {
        const list = options.list, key = kind + ':' + contact.id;
        const previousScroll = list.scrollTop, follow = scroll || list.scrollHeight - list.clientHeight - previousScroll < 70;
        if (scope !== key) { nodes.forEach(entry => entry.cleanup?.()); root.ElonSocialLinkViewer?.close(); root.ElonAiConversationShare?.reset(); list.replaceChildren(); nodes.clear(); scope = key; }
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
            const shared = !recalled && kind === 'group' && root.ElonAiConversationShare?.mount(bubble, msg, { api: options.api, groupId: contact.id, list, isCurrent: () => scope === key });
            if (!recalled && !shared) { root.ElonArticles.mount(bubble, text, options.api, options.user()?.id); media(bubble, msg.attachments); if (!msg.attachments?.length) root.ElonSourceLinks?.text(bubble, text); }
            if (kind === 'group' && msg.id && !shared) root.ElonGroupMessageRevisions.mount(bubble, contact.id, msg, options.api, () => options.changed(contact.id));
            if (kind === 'group' && !recalled) root.ElonGroupAiReplyContext?.mount(bubble, msg, { api: options.api, group: contact.id, owner: options.user()?.id, list, current: () => scope === key && options.user()?.id === owner, changed: () => options.changed(contact.id) }, media);
            if (msg.send_status) { const status = document.createElement('small'); status.textContent = msg.send_status; status.style.display = 'block'; bubble.append(status); }
            const owner = options.user()?.id, cleanup = !recalled && !shared && root.ElonSocialLinks?.mount(bubble, text, { api: options.api, owner, compact: !!compactLink, isCurrent: () => scope === key && options.user()?.id === owner });
            entry = { signature, block: bubble.closest('.chat-message-block'), cleanup: typeof cleanup === 'function' ? cleanup : undefined };
          }
          if (entry.block !== cursor) list.insertBefore(entry.block, cursor);
          cursor = entry.block.nextSibling; next.set(id, entry);
        });
        const keep = new Set(Array.from(next.values(), entry => entry.block));
        Array.from(list.children).forEach(node => { if (!keep.has(node)) node.remove(); });
        nodes.forEach((entry, id) => { if (!next.has(id)) entry.cleanup?.(); }); nodes = next; root.ElonAiConversationShare?.prune(); list.scrollTop = follow ? list.scrollHeight : previousScroll;
      },
      reset() { nodes.forEach(entry => entry.cleanup?.()); root.ElonSocialLinkViewer?.close(); root.ElonAiConversationShare?.reset(); scope = ''; nodes.clear(); },
    };
  }
  root.ElonSocialChatView = { create };
})(globalThis);
