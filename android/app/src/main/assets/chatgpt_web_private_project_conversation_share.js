(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateProjectConversationShare = api;
})(typeof window === 'object' ? window : null, function (page, contract) {
  'use strict';
  const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const PROJECT = /^g-p-[a-f0-9]{32}$/i;
  const USER = /^[A-Za-z0-9_-]{1,160}$/;
  const ROUTE = /^\/g\/(g-p-[a-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?\/c\/([^/]+)$/i;
  let last;

  function selected(modules, id, projectId) {
    const s = modules?.shared;
    if (typeof s?.H3 !== 'function' || s.H3() !== true || typeof s.mq !== 'function' ||
        typeof s.wV !== 'function' || typeof s.SV?.isPersonalWorkspace !== 'function' ||
        s.wV(s.SV.isPersonalWorkspace) !== true || typeof s.XM !== 'function' ||
        typeof s.HM?.getGizmoId !== 'function' || typeof s.HM.getCurrentLeafId !== 'function' ||
        typeof s.HM.hasNode !== 'function' || typeof s.nz !== 'function' || s.Pz?.Private !== 'private') return null;
    const account = s.mq(), thread = s.XM(id);
    if (typeof account?.isWorkspaceAccount !== 'function' || account.isWorkspaceAccount() !== false ||
        typeof account.isQuorum !== 'function' || account.isQuorum() !== false ||
        !USER.test(account.normalizedAccountUserId || '') || !thread || thread.isLoading !== false ||
        thread.is_do_not_remember !== false || s.HM.getGizmoId(thread) !== projectId ||
        thread.contextScopes != null && (!Array.isArray(thread.contextScopes) || thread.contextScopes.length)) return null;
    const owner = thread.sharedProjectConversationOwner?.id ?? account.normalizedAccountUserId;
    const inherited = thread.continuingFromSharedProjectConversationId ?? null;
    const leaf = s.HM.getCurrentLeafId(thread);
    if (!USER.test(owner) || inherited != null && !UUID.test(inherited) ||
        !UUID.test(leaf || '') || s.HM.hasNode(thread, leaf) !== true) return null;
    return { owner, leaf, inherited, user: account.normalizedAccountUserId };
  }

  function current(binding) {
    try {
      if (page.location.href !== binding.href || page.__elonChatGptDocumentToken !== binding.token ||
          contract.identity() !== binding.account || !contract.ready(binding)) return false;
      const now = selected(binding.modules, binding.id, binding.projectId);
      return now?.owner === binding.owner && now?.leaf === binding.leaf &&
        now?.inherited === binding.inherited && now?.user === binding.user;
    } catch (_) { return false; }
  }

  async function capture(path, readSnapshot) {
    const requested = ROUTE.exec(path || ''), url = new URL(page.location.href);
    if (!requested || !PROJECT.test(requested[1]) || !UUID.test(requested[2]) ||
        url.origin !== 'https://chatgpt.com' || url.search || url.hash || url.username || url.password) {
      throw new Error('share_project_scope_unconfirmed');
    }
    const projectId = requested[1], id = requested[2], visible = ROUTE.exec(url.pathname);
    if (url.pathname !== '/c/' + id && !(visible?.[1] === projectId && visible?.[2] === id)) {
      throw new Error('share_context_changed');
    }
    const account = contract.identity(), token = page.__elonChatGptDocumentToken;
    if (!account || !/^doc_[a-z0-9_]{3,80}$/.test(token || '')) throw new Error('share_auth_unavailable');
    const binding = { id, projectId, account, token, href: url.href, readSnapshot };
    if (!contract.ready(binding)) throw new Error('share_conversation_busy');
    const modules = await contract.load(), scope = selected(modules, id, projectId);
    if (!scope) throw new Error('share_project_scope_unconfirmed');
    Object.assign(binding, { modules, ...scope });
    if (!current(binding)) throw new Error('share_context_changed');
    return Object.freeze(binding);
  }

  function memberUrl(binding, payload) {
    const gizmo = payload?.gizmo, enums = binding.modules.shared.Pz;
    if (!gizmo || gizmo.id !== binding.projectId || gizmo.gizmo_snorlax_type === 'potion') return null;
    const recipient = gizmo.share_recipient;
    const shared = recipient == null ? Array.isArray(gizmo.sharing?.subjects) && gizmo.sharing.subjects.length > 0 :
      typeof recipient === 'string' && recipient !== enums.Private && Object.values(enums).includes(recipient);
    if (!shared) return null;
    const slug = gizmo.short_url ?? gizmo.id;
    if (typeof slug !== 'string' || !(slug === binding.projectId ||
        slug.startsWith(binding.projectId + '-') && /^[A-Za-z0-9_-]{1,124}$/.test(slug.slice(binding.projectId.length + 1)))) return null;
    // This is ShareProjectChatModal's existing-members URL, not share/create or a permission mutation.
    const url = binding.modules.shared.nz(binding.id, payload, binding.owner);
    const expected = 'https://chatgpt.com/g/' + slug + '/shared/c/' + binding.id + '?owner_user_id=' + binding.owner;
    return url === expected ? url : null;
  }

  async function resolve(path, readSnapshot) {
    const binding = await capture(path, readSnapshot);
    if (last && Date.now() - last.at >= 0 && Date.now() - last.at < 60000 &&
        binding.id === last.binding.id && binding.projectId === last.binding.projectId && current(last.binding)) return last.url;
    last = null;
    const headers = { Accept: 'application/json' };
    for (const [name, value] of Object.entries(page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders() || {})) {
      if (['authorization', 'chatgpt-account-id', 'oai-device-id', 'oai-language', 'oai-client-version',
        'oai-client-build-number'].includes(name.toLowerCase())) headers[name] = String(value);
    }
    if (!current(binding)) throw new Error('share_context_changed');
    const result = await page.__elonChatGptPrivateJsonRequest.request(page,
      '/backend-api/gizmos/' + encodeURIComponent(binding.projectId), {
        method: 'GET', headers, credentials: 'same-origin', cache: 'no-store', redirect: 'error',
        __elonPrivateTransport: 'project_conversation_share_v1',
      }, { timeoutMs: 7000, maxBytes: 1024 * 1024, mode: 'json' });
    if (!current(binding)) throw new Error('share_context_changed');
    const url = memberUrl(binding, result.payload);
    if (!url || !current(binding)) throw new Error('share_project_scope_unconfirmed');
    last = { binding, url, at: Date.now() };
    return url;
  }

  return Object.freeze({ version: 1, resolve });
});
