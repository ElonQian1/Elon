/* Shared, read-only original-page adapter. No network, storage, credentials or page actions. */
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
    if (u.hostname === 'mp.weixin.qq.com' && /^\/s(?:\/[^/]+)?$/.test(p)) return 'wechat:' + p + (p === '/s' ? u.search : '');
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
  function read(original) {
    try {
      if (root.top !== root || document.visibilityState === 'hidden') return null;
      const id = identity(original); if (!id || identity(location.href) !== id) return null;
      const visible = node => !!node && !!node.getClientRects().length && getComputedStyle(node).visibility !== 'hidden';
      const txt = node => visible(node) ? clean(node.innerText || node.textContent) : '';
      const meta = name => document.querySelector('meta[property="' + name + '"],meta[name="' + name + '"]')?.content || '';
      const canonical = document.querySelector('link[rel="canonical"]')?.href || meta('og:url');
      if (canonical && identity(canonical) !== id) return null;
      let title = '', author = '', cover = '', description = '', content;
      if (id.startsWith('wechat:')) {
        content = document.querySelector('#js_content'); if (!visible(content)) return null;
        title = txt(document.querySelector('#activity-name')); author = txt(document.querySelector('#js_name')); cover = meta('og:image'); description = meta('og:description') || meta('description');
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
  const api = { identity, image, readingUrl, validate, read };
  root.ElonSocialReadAdapter = api;
  return api;
})(globalThis)
