(function (root) {
  'use strict';
  const sites = {
    'mp.weixin.qq.com': '微信公众号', 'douyin.com': '抖音', 'www.douyin.com': '抖音', 'v.douyin.com': '抖音',
    'www.iesdouyin.com': '抖音', 'xiaohongshu.com': '小红书', 'www.xiaohongshu.com': '小红书', 'xhslink.com': '小红书', 'www.xhslink.com': '小红书',
    'bilibili.com': '哔哩哔哩', 'www.bilibili.com': '哔哩哔哩', 'm.bilibili.com': '哔哩哔哩', 'b23.tv': '哔哩哔哩',
    'binance.com': '币安广场', 'www.binance.com': '币安广场', 'app.binance.com': '币安广场', 'x.com': 'X', 'www.x.com': 'X', 'twitter.com': 'X', 'www.twitter.com': 'X', 'mobile.twitter.com': 'X', 't.co': 'X',
  };
  const labels = { '视频号': ['视频号', '视频号'], '微信公众号': ['文', '微信公众号文章'], '小红书': ['小红书', '小红书笔记'], '抖音': ['抖', '抖音视频'], '哔哩哔哩': ['B站', '哔哩哔哩视频'], '币安广场': ['币安', '币安广场帖子'], 'X': ['X', 'X 帖子'] };
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
    const origin = value.source === 'member' ? ' · 成员回填' : '';
    return { badge, title: value.title || fallback, summary: value.description || '', source: value.site + (author && author !== value.site ? ' · ' + author : '') + origin, time: seconds !== null && /^\d+$/.test(seconds) && n <= 604800 ? `从 ${stamp} 开始` : '' };
  }
  const cache = new Map();
  // A poster layout does not imply that the provider permits embedded playback.
  function mediaPresentation(value) {
    if (channelsId(value.url)) return { kind: 'channels', ratio: 3 / 4, action: '打开视频号', play: true };
    const url = safeUrl(value.url), path = url?.pathname || '';
    if (value.site === '哔哩哔哩' && (value.embed?.kind === 'bilibili' || url?.hostname === 'b23.tv' && path !== '/')) return { kind: 'bilibili', ratio: 16 / 9, action: '打开哔哩哔哩视频', play: true };
    if (value.site === '抖音' && (value.embed?.kind === 'douyin' || url?.hostname === 'v.douyin.com' && path !== '/')) return { kind: 'douyin', ratio: 3 / 4, action: '打开抖音视频', play: true };
    if (value.site === '小红书' && (/\/(?:explore|discovery\/item)\/[A-Za-z0-9]+/.test(path) || /^(?:www\.)?xhslink\.com$/.test(url?.hostname || '') && path !== '/')) return { kind: 'note', ratio: 3 / 4, action: '查看小红书笔记', play: false };
    return null;
  }
  const readers = new Set();
  function remember(owner, value, expires = Date.now() + 24 * 3600000) {
    if (!Number.isFinite(expires) || expires <= Date.now()) return;
    const item = links(value?.url || '')[0]; if (!item) return;
    const clean = sanitize(value, item); if (clean.status !== 'ready') return;
    const key = String(owner || '') + '\n' + item.url;
    while (cache.size >= 128) cache.delete(cache.keys().next().value);
    cache.set(key, { expires: Math.min(expires, Date.now() + 24 * 3600000), promise: Promise.resolve(clean), read: clean });
    readers.forEach(notify => notify(key, clean));
  }
  function genericTitle(title, site) {
    const clean = String(title || '').replace(/\s+/g, '').toLowerCase();
    return !clean || clean === site.toLowerCase() || ['小红书-你的生活兴趣社区', '小红书–你的生活兴趣社区', '微信公众平台', '微信公众号', '环境异常', '安全验证', '访问验证', '抖音-记录美好生活'].includes(clean);
  }
  function shareTitle(raw, site, nearby = '') {
    if (site === '哔哩哔哩') raw = biliShareGroups(nearby)?.find(t => !t.startsWith('精准空降')) || raw;
    if (site === '抖音') {
      const match = nearby.match(/看看【([^】]+)的作品】\s*(\S[\s\S]*)$/u);
      if (match) return { title: match[2].trim().slice(0, 160), author: match[1].slice(0, 80) };
    }
    if (site !== '小红书') return { title: raw.slice(0, 160), author: '' };
    const parts = raw.split(' | 小红书')[0].split(' - ');
    return parts.length > 1 ? { title: parts.slice(0, -1).join(' - ').slice(0, 160), author: parts.at(-1).slice(0, 80) } : { title: raw.slice(0, 160), author: '' };
  }
  // Bilibili titles can contain nested brackets; only collapse complete share wrappers.
  function biliShareGroups(text) {
    if (!text || text.length > 300) return null;
    const groups = []; let depth = 0, start = 0;
    for (let i = 0; i < text.length; i++) {
      const ch = text[i];
      if (ch === '【') { if (depth++ === 0) start = i + 1; }
      else if (ch === '】') {
        if (!depth) return null;
        if (--depth === 0) { const title = text.slice(start, i).trim(); if (!title || groups.length === 2) return null; groups.push(title); }
      } else if (!depth && !/\s/.test(ch)) return null;
    }
    return !depth && groups.length ? groups : null;
  }
  function compact(text) {
    const items = links(text); if (items.length !== 1) return false;
    const item = items[0], value = text.trim();
    if (value === item.url || value === `[${item.url}](${item.url})`) return true;
    const index = value.indexOf(item.url); if (index < 0) return false;
    const before = value.slice(0, index).trim(), after = value.slice(index + item.url.length).trim();
    if (item.site === '小红书') return !after && /^\d{1,3}\s+【[^】]+】\s+.{1,8}\s+[A-Za-z0-9]{6,32}\s+.{1,8}$/u.test(before);
    if (item.site === '抖音') return /^\d+(?:\.\d+)?\s+复制打开抖音[，,].+$/u.test(before) && /^[A-Za-z0-9]{3}:\/\s+[A-Za-z0-9@.]+\s+:[A-Za-z0-9]+\s+\d{2}\/\d{2}$/.test(after);
    return item.site === '哔哩哔哩' && !after && !!biliShareGroups(before);
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
  // Provider name, or the bare host for any other public page (generic Open Graph preview).
  function siteOf(u) { return sites[u.hostname] || u.hostname.replace(/^www\./, ''); }
  function channelsId(value) {
    const u = safeUrl(value); if (!u || u.hash) return null;
    if (u.hostname === 'weixin.qq.com' && /^\/sph\/[A-Za-z0-9_-]{1,128}$/.test(u.pathname)) return u.pathname.slice(5);
    if (u.hostname === 'channels.weixin.qq.com' && u.pathname === '/finder-preview/pages/sph' && u.searchParams.getAll('id').length === 1) {
      const id = u.searchParams.get('id'); return /^[A-Za-z0-9_-]{1,128}$/.test(id || '') ? id : null;
    }
    return null;
  }
  function links(text) {
    if (!text || /^【一龙(?:文章|项目|AI)/.test(text)) return [];
    const result = [], seen = new Set();
    for (const match of text.slice(0, 50000).matchAll(/https:\/\/[^\s<>"'\]\)]+/g)) {
      const raw = match[0].replace(/[，。！？；：、）】》”’.,!;]+$/g, '');
      const u = safeUrl(raw); if (!u || !u.hostname.includes('.') || /^\d+\.\d+\.\d+\.\d+$/.test(u.hostname) || seen.has(u.href)) continue;
      seen.add(u.href);
      const site = channelsId(u.href) ? '视频号' : siteOf(u);
      const nearby = text.slice(Math.max(0, match.index - 300), match.index);
      const title = [...nearby.matchAll(/【([^】]+)】/g)].map(m => m[1]).find(t => !t.startsWith('精准空降')) || '';
      result.push({ schema: 1, url: u.href, site, ...shareTitle(title, site, nearby), description: '', image: null, embed: embed(u.href), status: 'unavailable', source: 'server' });
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
  // Server-copied thumbnails avoid CDN referrer checks; bounded so a response cannot bloat memory.
  function inlineCover(value) { return typeof value === 'string' && value.length <= 98304 && /^data:image\/(?:jpeg|png|webp);base64,[A-Za-z0-9+/]+=*$/.test(value) ? value : null; }
  function sanitize(value, fallback) {
    if (!value || value.schema !== 1 || value.url !== fallback.url) return fallback;
    const title = typeof value.title === 'string' && !genericTitle(value.title, fallback.site) ? value.title.slice(0, 160) : fallback.title;
    return { ...fallback, title,
      description: typeof value.description === 'string' ? value.description.slice(0, 300) : '',
      author: typeof value.author === 'string' && value.author.trim() ? value.author.slice(0, 80) : fallback.author,
      author_avatar_data_url: typeof value.author_avatar_data_url === 'string' && value.author_avatar_data_url.length <= 12288 ? inlineCover(value.author_avatar_data_url) : null,
      image: inlineCover(value.cover_data_url) || safeUrl(value.image)?.href || null, embed: trustedEmbed(value.embed) || fallback.embed,
      status: value.status === 'ready' && title ? 'ready' : 'unavailable', source: value.source === 'member' ? 'member' : 'server' };
  }
  async function preview(item, options, refresh) {
    const key = String(options.owner || '') + '\n' + item.url;
    const entry = cache.get(key);
    if (entry && entry.expires > Date.now() && (!refresh || entry.read)) return entry.promise;
    while (cache.size >= 128) cache.delete(cache.keys().next().value);
    const controller = new AbortController(); const timer = setTimeout(() => controller.abort(), 15000);
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
      const channels = !!channelsId(item.url);
      const format = mediaPresentation(item);
      if (format) { button.classList.add('social-link-poster'); button.setAttribute('data-kind', format.kind); }
      if (channels) { wrap.classList.add('social-link-channels-wrap'); button.classList.add('social-link-channels'); }
      button.target = '_blank'; button.rel = 'noopener noreferrer';
      const copy = document.createElement('span'); copy.className = 'social-link-copy';
      const title = document.createElement('strong'); title.className = 'social-link-title';
      const summary = document.createElement('span'); summary.className = 'social-link-summary'; summary.hidden = true;
      const source = document.createElement('span'); source.className = 'social-link-source';
      const time = document.createElement('span'); time.className = 'social-link-time';
      const media = document.createElement('span'); media.className = 'social-link-media'; media.setAttribute('aria-hidden', 'true');
      const badge = document.createElement('span'); badge.className = 'social-link-badge';
      const cover = document.createElement('img'); cover.className = 'social-link-cover'; cover.alt = ''; cover.hidden = true; cover.referrerPolicy = 'no-referrer'; cover.loading = 'lazy';
      function posterState(loaded) {
        button.setAttribute('data-cover', loaded ? 'ready' : 'missing');
        if (format && !channels) media.style.aspectRatio = String(loaded ? Math.max(2 / 3, Math.min(16 / 9, cover.naturalWidth / cover.naturalHeight)) : 16 / 9);
      }
      cover.onload = () => { if (cover.naturalWidth > 0) posterState(true); };
      cover.onerror = () => { cover.hidden = true; cover.removeAttribute('src'); posterState(false); };
      const retry = document.createElement('button'); retry.type = 'button'; retry.className = 'social-link-retry'; retry.textContent = '更新预览'; retry.hidden = true;
      copy.append(title, summary, source, time); media.append(badge, cover); button.append(copy, media); wrap.append(button, retry); host.append(wrap);
      let avatar, initial;
      if (format) {
        const play = document.createElement('span'); play.className = 'social-link-play'; play.textContent = format.play ? '\u25b6' : '↗'; play.setAttribute('aria-hidden', 'true');
        media.append(play);
        const footer = document.createElement('span'); footer.className = 'social-link-video-footer';
        const identity = document.createElement('span'); identity.className = 'social-link-avatar'; identity.setAttribute('aria-hidden', 'true');
        initial = document.createElement('span'); identity.append(initial);
        avatar = document.createElement('img'); avatar.alt = ''; avatar.hidden = true; avatar.loading = 'lazy';
        avatar.onerror = () => { avatar.hidden = true; avatar.removeAttribute('src'); initial.hidden = false; }; identity.append(avatar);
        const author = document.createElement('span'); author.className = 'social-link-video-author';
        const action = document.createElement('span'); action.className = 'social-link-video-action'; action.textContent = channels && options.channelsHandoff ? '在微信中观看' : format.action;
        source.remove(); author.append(source, action); footer.append(identity, author); button.append(footer);
      }
      if (channels && options.openOriginal) {
        const original = document.createElement('button'); original.type = 'button'; original.className = 'social-link-original-action'; original.textContent = '查看原网页';
        original.onclick = () => options.openOriginal(current); wrap.append(original);
      }
      function draw(value) {
        current = value; const view = presentation(value);
        title.textContent = view.title; button.title = view.title;
        summary.textContent = view.summary; summary.hidden = !view.summary || options.compact === true;
        source.textContent = view.source; source.title = view.source;
        if (format) {
          source.textContent = channels ? value.author || '视频号作者' : view.source; source.title = source.textContent;
          initial.textContent = Array.from(value.author || view.badge)[0];
          const picture = value.author_avatar_data_url;
          avatar.hidden = !picture; initial.hidden = !!picture;
          if (picture) avatar.src = picture; else avatar.removeAttribute('src');
        }
        time.textContent = view.time; time.hidden = !view.time;
        badge.textContent = view.badge; media.setAttribute('data-site', value.site);
        button.setAttribute('aria-label', `${channels && options.channelsHandoff ? '在微信打开' : '打开'}${view.title}（${view.source}${view.time ? '，' + view.time : ''}）`);
        if (value.image) {
          if (cover.getAttribute('src') !== value.image) { posterState(false); cover.hidden = false; cover.src = value.image; }
        } else { cover.hidden = true; cover.removeAttribute('src'); posterState(false); }
      }
      async function load(refresh = false) {
        if (busy || !valid()) return; busy = true; retry.disabled = true;
        const value = await preview(item, options, refresh);
        busy = false;
        const observed = cache.get(String(options.owner || '') + '\n' + item.url);
        const latest = observed?.read && observed.expires > Date.now() ? observed.read : value;
        if (valid()) { draw(latest); retry.disabled = false; retry.hidden = latest.status === 'ready' || options.compact === true; }
      }
      button.onclick = event => {
        if (event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
        event.preventDefault(); (options.open || root.ElonSocialLinkViewer?.open || (p => root.open(p.url, '_blank', 'noopener,noreferrer')))(current);
      };
      retry.onclick = () => load(true); draw(item);
      const notify = (key, value) => { if (valid() && key === String(options.owner || '') + '\n' + item.url) draw(value); };
      readers.add(notify); cleanups.push(() => readers.delete(notify));
      if (typeof IntersectionObserver === 'function') {
        const observer = new IntersectionObserver(entries => {
          if (entries.some(e => e.isIntersecting)) { observer.disconnect(); load(); }
        }, { rootMargin: '150px' });
        observer.observe(wrap); cleanups.push(() => observer.disconnect());
      } else { queueMicrotask(() => load()); }
    }
    return () => { active = false; cleanups.forEach(fn => fn()); host.remove(); };
  }
  root.ElonSocialLinks = { safeUrl, channelsId, embed, trustedEmbed, links, sanitize, compact, prepareBubble, presentation, mediaPresentation, mount, remember };
})(globalThis);
