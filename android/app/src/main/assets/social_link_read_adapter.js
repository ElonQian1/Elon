/* Shared original-page metadata adapter. Passive observation only; no additional requests. */
(function (root) {
  'use strict';
  function safe(value) {
    try {
      if (typeof value !== 'string' || value.length > 4096 || /[\u0000-\u001f\u007f]/.test(value)) return null;
      const u = new URL(value);
      return u.protocol === 'https:' && !u.username && !u.password && (!u.port || u.port === '443') ? u : null;
    } catch { return null; }
  }
  function identity(value) {
    const u = safe(value); if (!u) return null;
    const p = u.pathname.replace(/\/$/, ''); let m;
    if (['www.douyin.com', 'douyin.com', 'www.iesdouyin.com'].includes(u.hostname)) {
      m = p.match(/^\/(?:share\/)?video\/(\d{5,24})$/); if (m) return 'douyin:' + m[1];
    }
    if (['www.xiaohongshu.com', 'xiaohongshu.com'].includes(u.hostname)) {
      m = p.match(/^\/(?:explore|discovery\/item)\/([a-f0-9]{24})$/); if (m) return 'xiaohongshu:' + m[1];
    }
    if (u.hostname === 'mp.weixin.qq.com' && /^\/s(?:\/[^/]+)?$/.test(p)) return 'wechat:' + p + (p === '/s' ? u.search : '');
    if (['bilibili.com', 'www.bilibili.com', 'm.bilibili.com'].includes(u.hostname)) {
      m = p.match(/^\/video\/(BV[A-Za-z0-9]{10})$/); if (m) return 'bilibili:' + m[1];
    }
    if (['binance.com', 'www.binance.com', 'app.binance.com'].includes(u.hostname)) {
      m = p.match(/^(?:\/[a-z]{2}(?:-[A-Z]{2})?)?\/square\/(?:post|article)\/(\d{5,24})$/) || p.match(/^\/uni-qr\/cpos\/(\d{5,24})$/);
      if (m) return 'binance:' + m[1];
    }
    if (['x.com', 'www.x.com', 'twitter.com', 'www.twitter.com', 'mobile.twitter.com'].includes(u.hostname)) {
      m = p.match(/^\/(?:[A-Za-z0-9_]{1,15}|i\/web)\/status\/(\d{5,24})$/);
      if (m) return 'x-post:' + m[1];
      m = p.match(/^\/i\/article\/(\d{5,24})$/); if (m) return 'x-article:' + m[1];
    }
    return null;
  }
  function image(value, kind) {
    const u = safe(value); if (!u) return null;
    if (kind === 'wechat' && (u.hostname === 'qpic.cn' || u.hostname.endsWith('.qpic.cn'))) return u.href;
    if (kind === 'bilibili' && (u.hostname === 'hdslb.com' || u.hostname.endsWith('.hdslb.com')) && /^\/bfs\/archive\//.test(u.pathname)) return u.href;
    if (kind === 'douyin' && ['douyinpic.com', 'byteimg.com'].some(h => u.hostname === h || u.hostname.endsWith('.' + h)) && !/avatar|logo|icon|gaosi/i.test(u.pathname)) return u.href;
    if (kind === 'xiaohongshu' && u.hostname.endsWith('.xhscdn.com') && !/avatar|logo|icon|fe-platform/i.test(u.pathname)) return u.href;
    if (kind === 'binance' && (u.hostname === 'bnbstatic.com' || u.hostname.endsWith('.bnbstatic.com')) && !/logo|avatar|icon/i.test(u.pathname)) return u.href;
    if (kind.startsWith('x-') && u.hostname === 'pbs.twimg.com' && /^\/(?:media|card_img|amplify_video_thumb|ext_tw_video_thumb|tweet_video_thumb)\//.test(u.pathname)) return u.href;
    return null;
  }
  function readingUrl(original) {
    const id = identity(original), u = safe(original);
    if (u?.hostname !== 'app.binance.com' || !id?.startsWith('binance:')) return original;
    const locale = u.searchParams.get('l');
    const language = locale && /^[a-z]{2}(?:-[A-Z]{2})?$/.test(locale) ? locale : 'en';
    return `https://www.binance.com/${language}/square/post/${id.split(':')[1]}${u.search}`;
  }
  function clean(value, max = 160) { return String(value || '').replace(/[\u0000-\u001f\u007f]/g, ' ').replace(/\s+/g, ' ').trim().slice(0, max); }
  function meaningful(title) {
    return title && !/^(?:X|Twitter|Home\s*\/\s*X|Binance(?: Square)?|币安(?:广场)?|微信公众号(?:文章)?|微信公众平台|环境异常|安全验证|访问验证|Just a moment\.*|Access Denied|Log in to X|Sign in to X|登录|登入)$/i.test(title)
      && !/^(?:Binance -|Binance Square -|Log in|Sign in|Page not found|This (?:post|page) (?:is|does)|Something went wrong|此[帖页].*(?:不存在|删除)|内容已删除|该内容已)/i.test(title);
  }
  function validate(original, value) {
    const id = identity(original);
    if (!id || !value || value.schema !== 1 || value.original !== original || identity(value.url) !== id || value.article !== true) return null;
    const title = clean(value.title); if (!meaningful(title)) return null;
    return { schema: 1, original, url: value.url, article: true, title, author: clean(value.author, 80), description: clean(value.description, 300), image: image(value.image, id.split(':')[0]) };
  }
  function readSource(original, player) {
    const u = safe(original), p = safe(player);
    if (u && ['douyin.com', 'www.douyin.com', 'www.iesdouyin.com', 'v.douyin.com'].includes(u.hostname)) {
      const id = p?.hostname === 'open.douyin.com' && p.pathname === '/player/video' ? p.searchParams.get('vid') : identity(original)?.split(':')[1];
      if (/^\d{5,24}$/.test(id || '') && (!identity(original) || identity(original) === 'douyin:' + id)) return 'https://www.douyin.com/video/' + id;
    }
    return original;
  }
  function cacheAlias(original, source) {
    if (original === source || identity(original) && identity(original) === identity(source)) return true;
    const u = safe(original);
    return u?.hostname === 'v.douyin.com' && /^\/[A-Za-z0-9_-]+\/?$/.test(u.pathname) && !!identity(source)?.startsWith('douyin:');
  }
  function findMedia(value, id) {
    const queue = [value]; let visited = 0;
    while (queue.length && ++visited < 6000) {
      const item = queue.shift(); if (!item || typeof item !== 'object') continue;
      if (String(item.aweme_id || item.awemeId || item.noteId || item.note_id || '') === id && (item.video || item.imageList)) return item;
      if (queue.length < 12000) queue.push(...Object.values(item).filter(v => v && typeof v === 'object').slice(0, 128));
    }
    return null;
  }
  function mediaFields(item, kind) {
    if (!item) return null;
    const video = item.video || {};
    const title = clean(kind === 'douyin' ? item.desc || item.item_title : item.title || item.desc);
    const first = item.imageList?.[0];
    const covers = kind === 'douyin' ? [...(video.origin_cover?.url_list || []), ...(video.cover?.url_list || [])]
      : [first?.urlDefault, first?.url, first?.infoList?.find(v => v.imageScene === 'WB_DFT')?.url];
    const author = clean(kind === 'douyin' ? item.author?.nickname : item.user?.nickname, 80);
    const poster = covers.slice(0, 8).map(c => image(typeof c === 'string' ? c.replace(/^http:/, 'https:') : c, kind)).find(Boolean);
    return meaningful(title) && poster ? { title, author, image: poster, description: '' } : null;
  }
  // Installed at document start. Keep one matching poster, never request URLs, tokens or bodies.
  function observeMedia() {
    if (root.top !== root || !identity(location.href)?.startsWith('douyin:') || root.__elonSocialMediaPreviewV1) return;
    const state = root.__elonSocialMediaPreviewV1 = { id: '', value: null };
    function target(raw, method) {
      try {
        const u = new URL(raw, location.href), id = u.searchParams.get('aweme_id');
        return String(method || 'GET').toUpperCase() === 'GET' && u.origin === location.origin &&
          u.pathname === '/aweme/v1/web/aweme/detail/' && identity(location.href) === 'douyin:' + id ? id : null;
      } catch { return null; }
    }
    function accept(id, value) {
      if (identity(location.href) !== 'douyin:' + id || value?.status_code !== 0 || String(value.aweme_detail?.aweme_id) !== id) return;
      const fields = mediaFields(value.aweme_detail, 'douyin');
      if (fields) Object.assign(state, { id, value: fields });
    }
    const nativeFetch = root.fetch;
    if (nativeFetch) root.fetch = function(input, init) {
      const pending = Reflect.apply(nativeFetch, this, arguments);
      const id = target(typeof input === 'string' || input instanceof URL ? String(input) : input?.url, init?.method || input?.method);
      if (id) pending.then(async response => {
        if (!response.ok || !response.body) return;
        const reader = response.clone().body.getReader(), chunks = []; let size = 0;
        const timer = setTimeout(() => { void reader.cancel().catch(() => {}); }, 5000);
        try {
          for (;;) { const { done, value } = await reader.read(); if (done) break;
            size += value.length; if (size > 1024 * 1024) { void reader.cancel().catch(() => {}); return; } chunks.push(value); }
          const bytes = new Uint8Array(size); let at = 0;
          for (const chunk of chunks) { bytes.set(chunk, at); at += chunk.length; }
          accept(id, JSON.parse(new TextDecoder().decode(bytes)));
        } finally { clearTimeout(timer); reader.releaseLock(); }
      }).catch(() => {});
      return pending;
    };
    const proto = root.XMLHttpRequest?.prototype, requests = new WeakMap();
    if (proto) {
      const open = proto.open, send = proto.send;
      proto.open = function(method, url) { requests.set(this, target(url, method)); return Reflect.apply(open, this, arguments); };
      proto.send = function() {
        const id = requests.get(this);
        if (id) this.addEventListener('load', () => {
          try { if (this.status !== 200) return;
            if (this.responseType === 'json') accept(id, this.response);
            else if ((!this.responseType || this.responseType === 'text') && this.responseText.length <= 1024 * 1024) accept(id, JSON.parse(this.responseText));
          } catch { /* non-JSON or rejected response */ }
        }, { once: true });
        return Reflect.apply(send, this, arguments);
      };
    }
  }
  function readMedia(id, visible, txt) {
    const [kind, key] = id.split(':'); let item;
    if (kind === 'xiaohongshu') {
      const state = root.__INITIAL_STATE__?.note;
      item = state?.noteDetailMap?.[key]?.note;
      if (!item) return null;
      // Map keys alone are not proof of the selected note's identity.
      if (String(item.noteId || item.note_id) !== key) return null;
      const content = document.querySelector('.note-content, #detail-desc');
      const title = txt(document.querySelector('#detail-title')) || txt(content);
      const result = mediaFields(item, kind);
      return visible(content) && result && title.includes(result.title.slice(0, 24)) ? result : null;
    }
    const content = document.querySelector('[data-e2e="detail-video-info"]');
    if (!visible(content) || !document.querySelector('.video_' + key)) return null;
    try {
      const raw = document.getElementById('RENDER_DATA')?.textContent || '';
      if (raw.length <= 1024 * 1024) item = findMedia(JSON.parse(decodeURIComponent(raw)), key);
    } catch { /* the current page may load its detail asynchronously */ }
    const cached = root.__elonSocialMediaPreviewV1;
    const result = mediaFields(item, kind) || (cached?.id === key ? cached.value : null);
    return result && txt(content).includes(result.title.slice(0, 24)) ? result : null;
  }
  function read(original) {
    try {
      if (root.top !== root || document.visibilityState === 'hidden') return null;
      const id = identity(original); if (!id || identity(location.href) !== id) return null;
      const visible = node => !!node && !!node.getClientRects().length && getComputedStyle(node).visibility !== 'hidden';
      const txt = node => visible(node) ? clean(node.innerText || node.textContent) : '';
      const meta = name => document.querySelector('meta[property="' + name + '"],meta[name="' + name + '"]')?.content || '';
      const canonical = document.querySelector('link[rel="canonical"]')?.href || meta('og:url');
      if (canonical && identity(canonical) !== id) return null;
      if (id.startsWith('douyin:') || id.startsWith('xiaohongshu:')) {
        const media = readMedia(id, visible, txt);
        return media ? validate(original, { schema: 1, original, url: location.href, article: true, ...media }) : null;
      }
      let title = '', author = '', cover = '', description = '', content;
      if (id.startsWith('wechat:')) {
        content = document.querySelector('#js_content'); if (!visible(content)) return null;
        title = txt(document.querySelector('#activity-name')); author = txt(document.querySelector('#js_name')); cover = meta('og:image'); description = meta('og:description') || meta('description');
      } else if (id.startsWith('bilibili:')) {
        const heading = document.querySelector('h1.video-title, h1.video-info-title, h1');
        title = txt(heading); if (!title) return null;
        const ogTitle = clean(meta('og:title')).replace(/[_-]哔哩哔哩.*$/, '');
        if (!ogTitle || title !== ogTitle) return null;
        cover = meta('og:image'); author = meta('author'); description = meta('og:description');
      } else if (id.startsWith('x-post:')) {
        // Match the permalink's own article, never a quoted post or a recommendation.
        for (const time of document.querySelectorAll('article time')) {
          const link = time.closest('a'); if (identity(link?.href) !== id) continue;
          content = time.closest('article'); if (!visible(content)) continue;
          const own = selector => [...content.querySelectorAll(selector)].filter(n => n.closest('article') === content && !n.closest('[data-testid="quoteTweet"], [role="link"]'));
          title = txt(own('[data-testid="tweetText"]')[0]);
          author = txt(own('[data-testid="User-Name"]')[0]);
          const photo = own('[data-testid="tweetPhoto"] img').find(n => visible(n));
          cover = photo?.currentSrc || photo?.src || own('video')[0]?.poster || '';
          // Long-form posts may present a linked article heading in the primary post.
          title ||= txt(own('[data-testid="twitter-article-title"]')[0]);
          if (!title && image(cover, 'x-post')) title = 'X 图片帖子';
          break;
        }
        if (!title) return null;
      } else {
        content = document.querySelector('article') || document.querySelector('main');
        if (!visible(content)) return null;
        title = txt(content.querySelector('h1'));
        const ogTitle = clean(meta('og:title')).replace(/\s*[|–-]\s*(?:Binance Square|币安广场|X|Twitter)\s*$/i, '');
        // Only accept page metadata backed by the actual rendered content, not a loading shell.
        const rendered = clean(content.innerText || content.textContent, 32000);
        if (!title && meaningful(ogTitle) && rendered.includes(ogTitle.slice(0, 32))) title = ogTitle;
        const summary = clean(meta('og:description') || meta('description'));
        if (!title && meaningful(summary) && rendered.includes(summary.slice(0, 32))) title = summary;
        else if (summary !== title) description = summary;
        author = meta('author') || meta('article:author'); cover = meta('og:image') || meta('twitter:image');
      }
      if (cover.startsWith('http:')) cover = 'https:' + cover.slice(5);
      return validate(original, { schema: 1, original, url: location.href, article: true, title, author, description, image: cover });
    } catch { return null; }
  }
  const api = { identity, image, readingUrl, readSource, cacheAlias, validate, read };
  try { observeMedia(); } catch { /* metadata is optional; never prevent the original page loading */ }
  root.ElonSocialReadAdapter = api;
  return api;
})(globalThis)
