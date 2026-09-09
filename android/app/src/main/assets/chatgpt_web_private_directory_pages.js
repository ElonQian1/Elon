(function (root) {
  'use strict';
  // Inspected web_20260909_b: apt, getProjectSidebar, iJ/getProjectConversations.
  const PROJECT_ID = /^g-p-[A-Za-z0-9_-]{1,160}$/;
  const ID = /^[A-Za-z0-9_-]{1,160}$/;
  function initial(scope) {
    if (scope === 'conversations') return { scope, token: 0, maximum: 200 };
    if (scope === 'projects') return { scope, token: null, maximum: 40 };
    if (PROJECT_ID.test(scope || '')) return { scope, token: '0', maximum: 200 };
    throw new Error('directory_scope_invalid');
  }
  function path(page) {
    if (page.scope === 'conversations') {
      return '/backend-api/conversations?offset=' + page.token + '&limit=28&order=updated&is_archived=false';
    }
    const cursor = page.token === null ? '' : '&cursor=' + encodeURIComponent(page.token);
    if (page.scope === 'projects') {
      return '/backend-api/gizmos/snorlax/sidebar?conversations_per_gizmo=0&owned_only=true&limit=20' + cursor;
    }
    return '/backend-api/gizmos/' + page.scope + '/conversations?cursor=' + encodeURIComponent(page.token);
  }
  function decode(page, text) {
    const payload = JSON.parse(String(text).replace(/^\)\]\}'\s*/, ''));
    if (!payload || !Array.isArray(payload.items)) throw new Error('directory_response_invalid');
    let items = payload.items;
    if (page.scope === 'projects') {
      // Project metadata comes from its resource, never an embedded conversation title.
      items = items.filter(item => item && Object.hasOwn(item, 'conversations')).map(item => {
        const value = item.gizmo?.gizmo;
        if (!PROJECT_ID.test(value?.id || '') || typeof value?.display?.name !== 'string' || !value.display.name.trim()) {
          throw new Error('directory_response_invalid');
        }
        return { id: value.id, title: value.display.name };
      });
    } else if (items.some(item => !item || !ID.test(item.id || '') || typeof item.title !== 'string' || !item.title.trim() ||
        (page.scope !== 'conversations' && !Object.hasOwn(item, 'owner')))) {
      throw new Error('directory_response_invalid');
    }
    let next = null, knownEnd = false;
    if (page.scope === 'conversations') {
      const { offset, limit, total } = payload;
      if ([offset, limit, total].every(Number.isSafeInteger) && offset === page.token &&
          offset >= 0 && limit > 0 && limit <= 200 && total >= 0 && items.length <= limit) {
        next = offset + limit < total ? offset + limit : null;
        knownEnd = next === null;
      }
    } else if (Object.hasOwn(payload, 'cursor')) {
      if (payload.cursor === null) knownEnd = true;
      else if (typeof payload.cursor === 'string' && payload.cursor.length > 0 &&
          payload.cursor.length <= 4096 && !/[\u0000-\u001f\u007f]/.test(payload.cursor)) next = payload.cursor;
    }
    return { items, next, knownEnd };
  }
  const exported = Object.freeze({ version: 1, initial, path, decode });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateDirectoryPages = exported;
})(typeof window === 'object' ? window : null);
