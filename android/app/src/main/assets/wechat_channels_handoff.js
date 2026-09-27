/* Read-only Channels preview -> official app handoff. Never emulates WeixinJSBridge. */
(function () {
  'use strict';
  if (window.ElonWechatChannelsHandoff) return window.ElonWechatChannelsHandoff;
  let state = null;
  let controller = null;
  function identity(value) {
    try {
      const u = new URL(value);
      if (u.protocol !== 'https:' || u.username || u.password || u.port || u.hash) return null;
      if (u.hostname === 'weixin.qq.com' && /^\/sph\/[A-Za-z0-9_-]{1,128}$/.test(u.pathname)) return u.pathname.slice(5);
      if (u.hostname === 'channels.weixin.qq.com' && u.pathname === '/finder-preview/pages/sph' && u.searchParams.getAll('id').length === 1) {
        const id = u.searchParams.get('id');
        return /^[A-Za-z0-9_-]{1,128}$/.test(id) ? id : null;
      }
    } catch (_) { /* invalid input */ }
    return null;
  }
  function buildScene(scene, now) {
    if (!scene || typeof scene.dynamicExportId !== 'string' || !/^export\/[A-Za-z0-9_-]{8,2048}$/.test(scene.dynamicExportId)) throw new Error('invalid_scene');
    if (!Number.isSafeInteger(scene.expiredTime) || scene.expiredTime * 1000 <= now + 5000) throw new Error('expired_scene');
    const pairs = [`exportId=${scene.dynamicExportId}`, 'actionType=0'];
    for (const key of ['commentScene', 'entryScene', 'entryCardType', 'requestScene']) {
      if (scene[key] === undefined) continue;
      if (!Number.isSafeInteger(scene[key]) || scene[key] < 0 || scene[key] > 1000000) throw new Error('invalid_scene');
      pairs.push(`${key}=${scene[key]}`);
    }
    return 'weixin://biz/finder/openFinderFeed/' + encodeURIComponent(pairs.join('&'));
  }
  function read(nonce) {
    if (!state || state.nonce !== nonce || state.source !== location.href) return { status: 'stale' };
    return { ...state };
  }
  function pageToken() {
    const query = new URL(location.href).searchParams.get('token');
    if (query) return query;
    const item = String(document.cookie || '').split(';').map(s => s.trim()).find(s => s.startsWith('token='));
    if (!item) return '';
    try { return decodeURIComponent(item.slice(6)); } catch (_) { return item.slice(6); }
  }
  function start(source, nonce) {
    if (state?.nonce === nonce) return read(nonce);
    controller?.abort();
    const id = identity(source);
    const next = { schema: 1, nonce, source, status: 'pending' };
    state = next;
    if (!id || source !== location.href || location.origin !== 'https://channels.weixin.qq.com') {
      next.status = 'failed'; next.error = 'page_not_ready'; return read(nonce);
    }
    controller = new AbortController();
    const owned = controller;
    const timer = setTimeout(() => owned.abort(), 6000);
    // Same-origin preview read, identical to the observed official request. No credentials leave the page.
    const endpoint = '/finder-preview/api/feed/get_feed_info?_pageUrl=' + encodeURIComponent(location.origin + location.pathname);
    fetch(endpoint, {
      method: 'POST', credentials: 'same-origin', redirect: 'error', signal: owned.signal,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ baseReq: { generalToken: pageToken() }, shortUri: id }),
    }).then(async response => {
      if (!response.ok) throw new Error('preview_unavailable');
      const body = await response.text();
      if (body.length > 65536) throw new Error('invalid_response');
      const value = JSON.parse(body);
      if (value.errCode !== 0) throw new Error('preview_unavailable');
      const url = buildScene(value.data?.sceneInfo, Date.now());
      if (state !== next || source !== location.href) return;
      Object.assign(next, { status: 'ready', url, expiresAt: value.data.sceneInfo.expiredTime * 1000 });
    }).catch(error => {
      if (state !== next) return;
      next.status = 'failed';
      next.error = ['expired_scene', 'invalid_scene', 'preview_unavailable', 'invalid_response'].includes(error.message) ? error.message : 'request_failed';
    }).finally(() => clearTimeout(timer));
    return read(nonce);
  }
  return window.ElonWechatChannelsHandoff = Object.freeze({ identity, buildScene, start, read });
})()
