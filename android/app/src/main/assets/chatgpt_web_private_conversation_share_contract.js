(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateConversationShareContract = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const urls = {
    shared: 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js',
    conversation: 'https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js',
  };
  let runtime;

  function identity() {
    const headers = page.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    const normalized = {};
    for (const [key, value] of Object.entries(headers || {})) normalized[key.toLowerCase()] = value;
    if (!/^Bearer\s+\S{8,65536}$/.test(normalized.authorization || '')) return null;
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => normalized[key] || ''));
  }

  async function load() {
    if (!Object.values(urls).every(url => page.performance?.getEntriesByName?.(url, 'resource')?.length > 0 ||
        page.document?.querySelector?.('link[rel="modulepreload"][href="' + url + '"]'))) return null;
    if (!runtime) {
      const importer = options?.loadRuntime || (url => import(url));
      runtime = Promise.all(Object.entries(urls).map(async ([key, url]) => [key, await importer(url)]))
        .then(Object.fromEntries);
      runtime.catch(() => { runtime = null; });
    }
    let timer;
    try {
      return await Promise.race([runtime, new Promise((_, reject) => {
        timer = page.setTimeout(() => reject(new Error('share_runtime_timeout')), 1500);
      })]);
    } finally { page.clearTimeout(timer); }
  }

  function scope(modules, id) {
    const s = modules?.shared, c = modules?.conversation;
    if (typeof s?.H3 !== 'function' || s.H3() !== true || typeof s.mq !== 'function' ||
        typeof s.wV !== 'function' || typeof s.SV?.isPersonalWorkspace !== 'function' ||
        s.wV(s.SV.isPersonalWorkspace) !== true || typeof s.XM !== 'function' ||
        typeof c?.AGt !== 'function' || typeof s.HM?.getGizmoId !== 'function' ||
        typeof s.HM.getCurrentLeafId !== 'function' || typeof s.HM.hasNode !== 'function') return null;
    const account = s.mq();
    if (typeof account?.isQuorum !== 'function' || account.isQuorum() !== false ||
        typeof account.isWorkspaceAccount !== 'function' || account.isWorkspaceAccount() !== false) return null;
    const thread = s.XM(id);
    if (!thread || thread.isLoading !== false || thread.is_do_not_remember !== false ||
        s.HM.getGizmoId(thread) != null || thread.sharedProjectConversationOwner != null ||
        thread.continuingFromSharedProjectConversationId != null ||
        thread.contextScopes != null && (!Array.isArray(thread.contextScopes) || thread.contextScopes.length)) return null;
    const leaf = s.HM.getCurrentLeafId(thread), node = c.AGt(thread);
    if (!UUID.test(leaf || '') || !UUID.test(node || '') || s.HM.hasNode(thread, leaf) !== true ||
        s.HM.hasNode(thread, node) !== true) return null;
    return { leaf, node, title: typeof thread.title === 'string' ? thread.title : '' };
  }

  function ready(binding) {
    const snapshot = binding.readSnapshot?.();
    return snapshot?.url === binding.href && snapshot.composerReady === true && snapshot.streaming === false &&
      !snapshot.dictationActive && !snapshot.dictationCaptureActive && !snapshot.dictationCapturePending;
  }

  async function capture(path, readSnapshot) {
    const url = new URL(page.location.href), id = path?.startsWith('/c/') ? path.slice(3) : '';
    if (!UUID.test(id) || url.origin !== 'https://chatgpt.com' || url.pathname !== path ||
        url.search || url.hash || url.username || url.password) throw new Error('share_context_unavailable');
    const token = page.__elonChatGptDocumentToken, account = identity();
    if (!/^doc_[a-z0-9_]{3,80}$/.test(token || '') || !account) throw new Error('share_auth_unavailable');
    const binding = { id, token, account, href: url.href, readSnapshot };
    if (!ready(binding)) throw new Error('share_conversation_busy');
    const modules = await load(), selected = scope(modules, id);
    if (!selected) throw new Error('share_scope_unconfirmed');
    Object.assign(binding, { modules, ...selected });
    if (!current(binding)) throw new Error('share_context_changed');
    return Object.freeze(binding);
  }

  function current(binding) {
    try {
      if (page.location.href !== binding.href || page.__elonChatGptDocumentToken !== binding.token ||
          identity() !== binding.account || !ready(binding)) return false;
      const selected = scope(binding.modules, binding.id);
      return selected?.leaf === binding.leaf && selected?.node === binding.node && selected?.title === binding.title;
    } catch (_) { return false; }
  }

  function moderation(value) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return 'unknown';
    const keys = ['has_been_auto_blocked', 'has_been_auto_moderated', 'has_been_blocked'];
    if (keys.some(key => value[key] === true)) return 'blocked';
    return keys.every(key => value[key] === undefined || value[key] === false) ? 'allowed' : 'unknown';
  }

  function created(payload, node) {
    if (!payload || !UUID.test(payload.share_id || '') || payload.current_node_id != null &&
        !UUID.test(payload.current_node_id) || payload.is_visible !== true ||
        typeof payload.is_public !== 'boolean' || payload.is_anonymous !== true) return null;
    const expected = 'https://chatgpt.com/share/' + payload.share_id;
    if (payload.share_url !== expected) return null;
    return Object.freeze({ id: payload.share_id, url: expected, node,
      title: typeof payload.title === 'string' ? payload.title : undefined,
      highlighted: typeof payload.highlighted_message_id === 'string' ? payload.highlighted_message_id : undefined,
      moderation: payload.moderation_state });
  }

  return Object.freeze({ version: 1, capture, current, identity, created, moderation, urls });
});
