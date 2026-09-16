(function (root) {
  'use strict';
  const sites = {
    'mp.weixin.qq.com': '微信公众号', 'douyin.com': '抖音', 'www.douyin.com': '抖音', 'v.douyin.com': '抖音',
    'www.iesdouyin.com': '抖音', 'xiaohongshu.com': '小红书', 'www.xiaohongshu.com': '小红书', 'xhslink.com': '小红书', 'www.xhslink.com': '小红书',
    'bilibili.com': '哔哩哔哩', 'www.bilibili.com': '哔哩哔哩', 'm.bilibili.com': '哔哩哔哩', 'b23.tv': '哔哩哔哩',
    'binance.com': '币安广场', 'www.binance.com': '币安广场', 'app.binance.com': '币安广场', 'x.com': 'X', 'www.x.com': 'X', 'twitter.com': 'X', 'www.twitter.com': 'X', 'mobile.twitter.com': 'X', 't.co': 'X',
  };
  const labels = { '微信公众号': ['文', '微信公众号文章'], '小红书': ['小红书', '小红书笔记'], '抖音': ['抖', '抖音视频'], '哔哩哔哩': ['B站', '哔哩哔哩视频'], '币安广场': ['币安', '币安广场帖子'], 'X': ['X', 'X 帖子'] };
  function presentation(value) {
    const [badge, fallback] = labels[value.site] || ['↗', '网页链接'];
    let author = value.author?.trim() || '';
    if (!author && value.site === 'X') {
      const parts = safeUrl(value.url)?.pathname.split('/').filter(Boolean) || [];
      if (parts[1] === 'status' && parts[0] !== 'i' && /^[A-Za-z0-9_]{1,15}$/.test(parts[0])) author = '@' + parts[0];
    }
    const seconds = value.embed?.kind === 'bilibili' ? new URL(value.embed.url).searchParams.get('t') : null;
    const n = Number(seconds);
    const stamp = n >= 3600 ? `${Math.floor(n / 3600)}:${String(Math.floor(n / 60) % 60).padStart(2, '0')}:${String(n % 60).padStart(2, '0')}` : `${String(Math.floor(n / 60)).padStart(2, '0')}:${String(n % 60).padStart(2, '0')}`;
    return { badge, title: value.title || fallback, source: value.site + (author && author !== value.site ? ' · ' + author : ''), time: seconds !== null && /^\d+$/.test(seconds) && n <= 604800 ? `从 ${stamp} 开始` : '' };
  }
  const cache = new Map();
  function genericTitle(title, site) {
    const clean = String(title || '').replace(/\s+/g, '').toLowerCase();
    return !clean || clean === site.toLowerCase() || ['小红书-你的生活兴趣社区', '小红书–你的生活兴趣社区', '微信公众平台', '微信公众号', '环境异常', '安全验证', '访问验证', '抖音-记录美好生活'].includes(clean);
  }
  function shareTitle(raw, site, nearby = '') {
    if (site === '抖音') {
      const match = nearby.match(/看看【([^】]+)的作品】\s*(\S[\s\S]*)$/u);
      if (match) return { title: match[2].trim().slice(0, 160), author: match[1].slice(0, 80) };
    }
    if (site !== '小红书') return { title: raw.slice(0, 160), author: '' };
    const parts = raw.split(' | 小红书')[0].split(' - ');
    return parts.length > 1 ? { title: parts.slice(0, -1).join(' - ').slice(0, 160), author: parts.at(-1).slice(0, 80) } : { title: raw.slice(0, 160), author: '' };
  }
  function compact(text) {
    const items = links(text); if (items.length !== 1) return false;
    const item = items[0], value = text.trim();
    if (value === item.url || value === `[${item.url}](${item.url})`) return true;
    const index = value.indexOf(item.url); if (index < 0) return false;
    const before = value.slice(0, index).trim(), after = value.slice(index + item.url.length).trim();
    if (item.site === '小红书') return !after && /^\d{1,3}\s+【[^】]+】\s+.{1,8}\s+[A-Za-z0-9]{6,32}\s+.{1,8}$/u.test(before);
    if (item.site === '抖音') return /^\d+(?:\.\d+)?\s+复制打开抖音[，,].+$/u.test(before) && /^[A-Za-z0-9]{3}:\/\s+[A-Za-z0-9@.]+\s+:[A-Za-z0-9]+\s+\d{2}\/\d{2}$/.test(after);
    return item.site === '哔哩哔哩' && !after && /^(?:【[^】]+】\s*){1,2}$/.test(before);
  }
  function prepareBubble(bubble, text) {
    if (!compact(text)) return false;
    const original = document.createElement('span'); original.className = 'social-link-original'; original.hidden = true;
    while (bubble.firstChild) original.append(bubble.firstChild);
    bubble.append(original); bubble.classList.add('social-link-only'); return true;
  }
  function safeUrl(value) {
    try {
      if (typeof value !== 'string' || value.length > 4096 || /[\u0000-\u001f\u007f]/.test(value)) return null;
      const u = new URL(value);
      return u.protocol === 'https:' && !u.username && !u.password && (!u.port || u.port === '443') ? u : null;
    } catch { return null; }
  }
  function embed(value) {
    const u = safeUrl(value); if (!u) return null;
    const site = sites[u.hostname], parts = u.pathname.split('/').filter(Boolean);
    if (site === '哔哩哔哩') {
      if (!parts.includes('video')) return null;
      const id = parts[parts.indexOf('video') + 1];
      if (!/^BV[A-Za-z0-9]{10}$/.test(id || '')) return null;
      const player = new URL('https://player.bilibili.com/player.html');
      player.searchParams.set('bvid', id); player.searchParams.set('autoplay', '0'); player.searchParams.set('poster', '1');
      for (const key of ['t', 'p']) {
        const v = u.searchParams.get(key);
        if (v !== null && /^\d+$/.test(v) && Number(v) <= (key === 't' ? 604800 : 10000) && (key !== 'p' || Number(v) > 0)) player.searchParams.set(key, v);
      }
      return { kind: 'bilibili', id, url: player.href };
    }
    const segment = site === '抖音' ? 'video' : site === 'X' ? 'status' : '';
    if (!segment || !parts.includes(segment)) return null;
    const id = parts[parts.indexOf(segment) + 1]; if (!/^\d{5,24}$/.test(id || '')) return null;
    return site === 'X' ? { kind: 'x', id, url: `https://x.com/i/status/${id}` }
      : { kind: 'douyin', id, url: `https://open.douyin.com/player/video?vid=${id}&autoplay=0` };
  }
  function links(text) {
    if (!text || /^【一龙(?:文章|项目|AI)/.test(text)) return [];
    const result = [], seen = new Set();
    for (const match of text.slice(0, 50000).matchAll(/https:\/\/[^\s<>"'\]\)]+/g)) {
      const raw = match[0].replace(/[，。！？；：、）】》”’.,!;]+$/g, '');
      const u = safeUrl(raw); if (!u || !sites[u.hostname] || seen.has(u.href)) continue;
      seen.add(u.href);
      const nearby = text.slice(Math.max(0, match.index - 300), match.index);
      const title = [...nearby.matchAll(/【([^】]+)】/g)].map(m => m[1]).find(t => !t.startsWith('精准空降')) || '';
      result.push({ schema: 1, url: u.href, site: sites[u.hostname], ...shareTitle(title, sites[u.hostname], nearby), image: null, embed: embed(u.href), status: 'unavailable' });
      if (result.length === 2) break;
    }
    return result;
  }
  function trustedEmbed(value) {
    if (!value || !['x', 'douyin', 'bilibili'].includes(value.kind)) return null;
    if (value.kind === 'x') return /^\d{5,24}$/.test(value.id) ? embed(`https://x.com/i/status/${value.id}`) : null;
    if (value.kind === 'douyin') return /^\d{5,24}$/.test(value.id) ? embed(`https://www.douyin.com/video/${value.id}`) : null;
    const url = safeUrl(value.url);
    if (!url || url.hostname !== 'player.bilibili.com' || url.pathname !== '/player.html' || !/^BV[A-Za-z0-9]{10}$/.test(value.id)) return null;
    const source = new URL(`https://www.bilibili.com/video/${value.id}`);
    for (const key of ['t', 'p']) { if (url.searchParams.has(key)) source.searchParams.set(key, url.searchParams.get(key)); }
    return embed(source.href);
  }
  function sanitize(value, fallback) {
    if (!value || value.schema !== 1 || value.url !== fallback.url) return fallback;
    const title = typeof value.title === 'string' && !genericTitle(value.title, fallback.site) ? value.title.slice(0, 160) : fallback.title;
    return { ...fallback, title,
      author: typeof value.author === 'string' && value.author.trim() ? value.author.slice(0, 80) : fallback.author,
      image: safeUrl(value.image)?.href || null, embed: trustedEmbed(value.embed) || fallback.embed,
      status: value.status === 'ready' && title ? 'ready' : 'unavailable' };
  }
  async function preview(item, options, refresh) {
    const key = String(options.owner || '') + '\n' + item.url;
    const entry = cache.get(key);
    if (!refresh && entry && entry.expires > Date.now()) return entry.promise;
    while (cache.size >= 128) cache.delete(cache.keys().next().value);
    const controller = new AbortController(); const timer = setTimeout(() => controller.abort(), 12000);
    const record = { expires: Date.now() + 30000, promise: null };
    record.promise = Promise.resolve().then(() => options.api('/api/me/link-preview', {
      method: 'POST', body: JSON.stringify({ url: item.url }), signal: controller.signal,
    })).then(value => {
      if (typeof value?.json === 'function') { if (!value.ok) throw new Error('preview unavailable'); return value.json(); }
      return value;
    }).then(value => sanitize(value, item), () => item).then(value => {
      record.expires = Date.now() + (value.status === 'ready' ? 3600000 : 30000); return value;
    }).finally(() => clearTimeout(timer));
    cache.set(key, record); return record.promise;
  }
  function mount(container, text, options) {
    const items = links(text); if (!items.length) return () => {};
    let active = true; const cleanups = [];
    const host = document.createElement('div'); host.className = 'social-link-cards' + (options.compact ? ' social-link-cards-compact' : '') + (options.desktop ? ' social-link-cards-desktop' : ''); container.append(host);
    const valid = () => active && host.parentNode === container && (!options.isCurrent || options.isCurrent());
    for (const item of items) {
      let current = item, busy = false;
      const wrap = document.createElement('div'); wrap.className = 'social-link-wrap';
      const button = document.createElement('a'); button.className = 'social-link-card'; button.href = item.url;
      button.target = '_blank'; button.rel = 'noopener noreferrer';
      const copy = document.createElement('span'); copy.className = 'social-link-copy';
      const title = document.createElement('strong'); title.className = 'social-link-title';
      const source = document.createElement('span'); source.className = 'social-link-source';
      const time = document.createElement('span'); time.className = 'social-link-time';
      const media = document.createElement('span'); media.className = 'social-link-media'; media.setAttribute('aria-hidden', 'true');
      const badge = document.createElement('span'); badge.className = 'social-link-badge';
      const cover = document.createElement('img'); cover.className = 'social-link-cover'; cover.alt = ''; cover.hidden = true; cover.referrerPolicy = 'no-referrer'; cover.loading = 'lazy';
      cover.onerror = () => { cover.hidden = true; cover.removeAttribute('src'); };
      const retry = document.createElement('button'); retry.type = 'button'; retry.className = 'social-link-retry'; retry.textContent = '更新预览'; retry.hidden = true;
      copy.append(title, source, time); media.append(badge, cover); button.append(copy, media); wrap.append(button, retry); host.append(wrap);
      function draw(value) {
        current = value; const view = presentation(value);
        title.textContent = view.title; button.title = view.title;
        source.textContent = view.source; source.title = view.source;
        time.textContent = view.time; time.hidden = !view.time;
        badge.textContent = view.badge; media.setAttribute('data-site', value.site);
        button.setAttribute('aria-label', `打开${view.title}（${view.source}${view.time ? '，' + view.time : ''}）`);
        if (value.image) { cover.hidden = false; cover.src = value.image; } else { cover.hidden = true; cover.removeAttribute('src'); }
      }
      async function load(refresh = false) {
        if (busy || !valid()) return; busy = true; retry.disabled = true;
        const value = await preview(item, options, refresh);
        busy = false;
        if (valid()) { draw(value); retry.disabled = false; retry.hidden = value.status === 'ready' || options.compact === true; }
      }
      button.onclick = event => {
        if (event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
        event.preventDefault(); (options.open || root.ElonSocialLinkViewer?.open || (p => root.open(p.url, '_blank', 'noopener,noreferrer')))(current);
      };
      retry.onclick = () => load(true); draw(item);
      if (typeof IntersectionObserver === 'function') {
        const observer = new IntersectionObserver(entries => {
          if (entries.some(e => e.isIntersecting)) { observer.disconnect(); load(); }
        }, { rootMargin: '150px' });
        observer.observe(wrap); cleanups.push(() => observer.disconnect());
      } else { queueMicrotask(() => load()); }
    }
    return () => { active = false; cleanups.forEach(fn => fn()); host.remove(); };
  }
  root.ElonSocialLinks = { safeUrl, embed, trustedEmbed, links, sanitize, compact, prepareBubble, presentation, mount };
})(globalThis);
