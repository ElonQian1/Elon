/* Authenticated group snapshot cards and short-lived media; no provider session or URL. */
(function (root) {
  'use strict';
  const SCHEMA = 'elon.ai_conversation_share.v1', PREFIX = '【一龙AI对话】\n';
  const { el, button } = root.ElonAiShareRich;
  const cards = new Map();
  const opaque = id => typeof id === 'string' && /^[a-zA-Z0-9_.-]{1,160}$/.test(id) && !['.', '..'].includes(id);
  function reference(content, group) {
    try {
      if (typeof content !== 'string' || !content.startsWith(PREFIX) || content.length > 5000) return null;
      const ref = JSON.parse(content.slice(PREFIX.length));
      return ref.schema === SCHEMA && ref.provider === 'chatgpt' && opaque(ref.snapshot_id) && opaque(ref.group_id)
        && ref.group_id === group && typeof ref.title === 'string' && typeof ref.summary === 'string'
        && ref.title.length <= 240 && ref.summary.length <= 600 && Number.isSafeInteger(ref.message_count)
        && ref.message_count > 0 && ref.message_count <= 200 ? ref : null;
    } catch { return null; }
  }
  const path = ref => '/api/me/groups/' + encodeURIComponent(ref.group_id) + '/ai-snapshots/' + encodeURIComponent(ref.snapshot_id);
  const denied = status => [401, 403, 404, 410].includes(status);
  const errorText = status => ({
    401: '登录已失效，请重新登录一龙账号',
    403: '无权查看，仅当前群成员可读',
    404: '分享已撤销、撤回或不存在',
    410: '分享已撤销',
  }[status] || '读取失败，请检查网络后重试');
  async function read(api, ref, signal) {
    const response = await api(path(ref), { signal, cache: 'no-store', redirect: 'error' });
    if (!response.ok) throw Object.assign(new Error(errorText(response.status)), { status: response.status });
    const view = await response.json(), doc = view.document;
    if (view.snapshot_id !== ref.snapshot_id || view.group_id !== ref.group_id || doc?.schema !== SCHEMA
      || doc.provider !== 'chatgpt' || !Array.isArray(doc.messages) || doc.messages.length > 200
      || doc.messages.some(m => !['user', 'assistant'].includes(m.role) || typeof m.content !== 'string' || !Array.isArray(m.parts))) {
      throw new Error('分享格式不受支持，请更新后重试');
    }
    return view;
  }
  function mediaScope(api, ref, onDenied) {
    let disposed = false;
    const controllers = new Set(), urls = new Set(), figures = new Set();
    function image(part, preview = false) {
      const figure = el('figure', null, preview ? 'ai-share-cover' : 'ai-share-image'); figures.add(figure);
      const img = el('img'), status = el('figcaption', '正在加载图片…');
      img.alt = part.caption || part.label || '分享图片'; img.decoding = 'async'; img.referrerPolicy = 'no-referrer'; img.hidden = true;
      const retry = button('重试图片', () => void load()); retry.hidden = true;
      const expand = button('查看图片', () => figure.classList.toggle('expanded')); expand.className = 'ai-share-image-button';
      expand.replaceChildren(img); expand.disabled = true;
      figure.append(preview ? img : expand, status, retry);
      let busy = false, objectUrl;
      function release() { if (objectUrl) { URL.revokeObjectURL(objectUrl); urls.delete(objectUrl); objectUrl = null; } img.removeAttribute('src'); }
      async function load() {
        if (disposed || busy || !opaque(part.asset_id)) { if (!opaque(part.asset_id)) status.textContent = '图片未包含在分享中'; return; }
        const controller = new AbortController(); controllers.add(controller); busy = true; retry.hidden = true;
        release(); status.textContent = '正在加载图片…'; img.hidden = true;
        const timeout = setTimeout(() => controller.abort(), 20000);
        try {
          const response = await api(path(ref) + '/assets/' + encodeURIComponent(part.asset_id), { signal: controller.signal, cache: 'no-store', redirect: 'error' });
          if (!response.ok) throw Object.assign(new Error(errorText(response.status)), { status: response.status });
          const type = (response.headers.get('content-type') || '').split(';')[0].trim().toLowerCase();
          if (!['image/png', 'image/jpeg', 'image/webp'].includes(type)) throw Error('图片格式不受支持');
          const blob = await response.blob();
          if (blob.size > 12 * 1024 * 1024 || disposed || controller.signal.aborted) throw Error('图片无法加载');
          objectUrl = URL.createObjectURL(blob); urls.add(objectUrl); img.src = objectUrl;
          await img.decode();
          if (disposed || controller.signal.aborted) return;
          img.hidden = false; expand.disabled = false; status.textContent = preview ? '' : (part.caption || '');
        } catch (error) {
          if (disposed) return;
          release(); status.textContent = errorText(error.status);
          retry.hidden = denied(error.status); if (denied(error.status)) onDenied?.(error.status);
        } finally { clearTimeout(timeout); busy = false; controllers.delete(controller); }
      }
      void load(); return figure;
    }
    return { image, dispose() {
      disposed = true; controllers.forEach(c => c.abort()); controllers.clear();
      urls.forEach(url => URL.revokeObjectURL(url)); urls.clear();
      figures.forEach(f => f.remove()); figures.clear();
    } };
  }
  function invalidate(ref, status) {
    cards.forEach(entry => {
      if (entry.ref.group_id !== ref.group_id || entry.ref.snapshot_id !== ref.snapshot_id) return;
      entry.controller?.abort(); entry.media?.dispose(); entry.preview?.remove();
      entry.info.textContent = errorText(status); entry.card.dataset.state = 'denied';
      entry.title.textContent = 'AI 对话片段'; entry.excerpt.textContent = '';
    });
  }
  function prune() {
    cards.forEach((entry, bubble) => { if (!bubble.isConnected) { entry.dispose(); cards.delete(bubble); } });
  }
  function mount(bubble, message, options) {
    const ref = reference(message.content, options.groupId);
    if (!ref || message.recalled_at || message.recalledAt) return false;
    prune(); cards.get(bubble)?.dispose();
    const sender = typeof ref.sender_name === 'string' && ref.sender_name ? ref.sender_name : message.sender_name || '分享者';
    const entry = { ref }, card = button('', () => root.ElonAiConversationReader.open({ ...options, ref, messageId: message.id, senderName: sender }));
    card.className = 'ai-share-card'; card.setAttribute('aria-label', '阅读 AI 对话片段：' + ref.title);
    const title = el('strong', ref.title || 'AI 对话片段'), excerpt = el('span', ref.summary, 'ai-share-excerpt');
    const info = el('small', ref.message_count + ' 条消息 · ChatGPT · ' + sender);
    card.append(title, excerpt, info); bubble.replaceChildren(card); bubble.classList.add('ai-share-card-bubble');
    Object.assign(entry, { card, title, excerpt, info });
    let observer, disposed = false;
    entry.dispose = () => { disposed = true; observer?.disconnect(); entry.controller?.abort(); entry.media?.dispose(); };
    cards.set(bubble, entry);
    // The marker contains no media URL: preview images must come from a fresh authorized snapshot.
    async function preview() {
      observer?.disconnect(); if (disposed) return;
      const controller = new AbortController(); entry.controller = controller;
      const timeout = setTimeout(() => controller.abort(), 20000);
      try {
        let first;
        if (Object.hasOwn(ref, 'cover_asset_id')) {
          if (opaque(ref.cover_asset_id)) first = { asset_id: ref.cover_asset_id, label: '分享封面' };
        } else {
          const view = await read(options.api, ref, controller.signal);
          const imageParts = view.document.messages.flatMap(m => m.parts).filter(p => p.type === 'image' && opaque(p.asset_id));
          first = imageParts.find(p => p.asset_id === view.document.cover_asset_id) || imageParts[0];
        }
        if (disposed || controller.signal.aborted || !bubble.isConnected) return;
        if (!first) return;
        entry.media = mediaScope(options.api, ref, status => invalidate(ref, status));
        // Keep the preview outside the card button so media retries never nest interactive controls.
        entry.preview = entry.media.image(first, true); bubble.append(entry.preview);
        entry.preview.addEventListener('click', event => { if (!event.target.closest('button')) card.click(); });
      } catch (error) { if (!disposed && denied(error.status)) invalidate(ref, error.status); }
      finally { clearTimeout(timeout); }
    }
    if (root.IntersectionObserver) {
      observer = new IntersectionObserver(rows => { if (rows.some(row => row.isIntersecting)) void preview(); }); observer.observe(card);
    } else void preview();
    return true;
  }
  function reset() {
    root.ElonAiConversationReader?.close(); cards.forEach(entry => entry.dispose()); cards.clear();
  }
  root.ElonAiConversationShare = { reference, mount, prune, reset, path, read, mediaScope, errorText, denied, invalidate };
})(globalThis);
