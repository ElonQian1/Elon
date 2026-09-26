(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 8, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptRspackContext = api;
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const UUID = '[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}';
  const idPattern = new RegExp('^(?:local-chatgpt:)?' + UUID + '$', 'i');
  const conversationPath = new RegExp('^/c/(' + UUID + ')$', 'i');
  const ownerPath = page.__elonChatGptCommittedOwnerPath ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_committed_owner_path') : null);
  let code = 'not_observed';
  const fail = reason => { code = reason; return null; };

  function ancestors(node) {
    for (let host = node, depth = 0; host?.isConnected && depth < 12; host = host.parentElement, depth++) {
      if (host === page.document.body || host === page.document.documentElement) break;
      const key = Object.keys(host).find(name => name.startsWith('__reactFiber$'));
      if (key) return ownerPath?.resolve(host[key])?.ancestors || [];
    }
    return [];
  }
  function owners(node, runtime) {
    const scopes = new Set(), ids = new Set();
    for (const fiber of ancestors(node)) {
      const props = fiber.memoizedProps;
      if (idPattern.test(props?.conversationId || '')) ids.add(props.conversationId);
      // X9.z retains the live AppScope in a useRef. Never invoke a hook or component.
      let hook = fiber.memoizedState;
      const seen = new Set();
      for (let count = 0; hook && count < 256 && !seen.has(hook); count++, hook = hook.next) {
        seen.add(hook);
        const scope = hook.memoizedState?.current;
        if (scope?.scope !== runtime.scope.a || scope.node?.token !== runtime.scope.a ||
            scope.chain?.get?.(runtime.scope.a.id) !== scope.node ||
            !['get', 'set', 'watch', 'when'].every(key => typeof scope[key] === 'function')) continue;
        scopes.add(scope);
      }
    }
    // Multiple useScope refs may share the same committed scope node and chain.
    const nodes = new Set([...scopes].map(scope => scope.node));
    if (nodes.size !== 1 || ids.size !== 1) return fail(nodes.size > 1 || ids.size > 1 ? 'owner_ambiguous' : 'owner_pending');
    return { scope: scopes.values().next().value, id: ids.values().next().value };
  }
  function capture(node, reading = false, expectedUploads = []) {
    const runtime = page.__elonChatGptRspackRuntime?.peek();
    if (!runtime) return fail('runtime_pending');
    if (!node?.isConnected) return fail('composer_detached');
    const url = new URL(page.location.href), pathId = conversationPath.exec(url.pathname)?.[1] || null;
    if (url.origin !== 'https://chatgpt.com' || url.hash || url.username || url.password ||
        url.pathname !== '/' && !pathId || url.search && url.search !== '?temporary-chat=true') return fail('route_unsupported');
    const token = page.__elonChatGptDocumentToken;
    if (!/^doc_[a-z0-9_]{3,80}$/.test(token || '')) return fail('document_unavailable');
    const owner = owners(node, runtime);
    if (!owner) return null;
    const { scope, id } = owner, auth = runtime.auth;
    if (auth.isBrowserWorkspaceSwitchPending() || auth.isBrowserAccountSwitchLoading()) return fail('identity_pending');
    const identity = auth.getBrowserChatGptAuthSnapshot();
    if (!identity?.accountId || !identity.userId || !identity.accessToken ||
        scope.get(runtime.identity.d) !== identity.accountId || scope.get(runtime.identity.i) !== identity.userId ||
        scope.get(runtime.identity.j)?.status !== 'allowed') return fail('identity_unavailable');
    const serverId = scope.get(runtime.conversation.i, id) || null;
    // A temporary/new local thread acquires a server ID before its URL changes.
    // Its committed owner remains authoritative; never accept a malformed server ID.
    const committedServerOwner = reading && serverId !== null && id === serverId;
    if (pathId ? serverId !== pathId : !id.startsWith('local-chatgpt:') && !committedServerOwner ||
        serverId !== null && !new RegExp('^' + UUID + '$', 'i').test(serverId)) return fail('conversation_mismatch');
    const binding = { scope, id, node, runtime, token, href: url.href, serverId,
      accountId: identity.accountId, userId: identity.userId, generation: auth.getBrowserChatGptAuthGeneration(),
      temporary: url.search === '?temporary-chat=true', parent: scope.get(runtime.conversation.z, id) || null };
    // Reading the active branch must not wait for an idle or empty composer.
    // The send entry continues to apply every write-admission guard below.
    if (reading) { code = 'ready'; return binding; }
    // Work, custom GPTs and project sends have additional context contracts.
    if (scope.get(runtime.conversation.K, id) != null || scope.get(runtime.conversation.w, id) != null) return fail('mode_unsupported');
    if (scope.get(runtime.conversation.T, id) !== 'idle' || scope.get(runtime.composer.h, id)) return fail('busy');
    const uploads = scope.get(runtime.composer.f, id), hints = scope.get(runtime.composer.u, id);
    const draft = scope.get(runtime.composer.r, id), model = scope.get(runtime.composer.s, id);
    if (!Array.isArray(uploads) || uploads.length !== expectedUploads.length ||
        !uploads.every((item, index) => item === expectedUploads[index]) ||
        !Array.isArray(hints) || hints.length) return fail('attachments_or_tools_present');
    if (typeof draft !== 'string' || typeof model?.slug !== 'string' || !model.slug ||
        model.thinkingEffort != null && typeof model.thinkingEffort !== 'string') return fail('composer_state_pending');
    code = 'ready';
    return { ...binding, draft, model: { ...model }, uploads: [...expectedUploads] };
  }
  function current(binding) {
    try {
      const next = capture(binding.node, false, binding.uploads);
      return !!next && ['id', 'token', 'href', 'serverId', 'draft', 'accountId', 'userId', 'generation', 'parent']
        .every(key => next[key] === binding[key]) && next.scope.node === binding.scope.node &&
        next.model.slug === binding.model.slug && next.model.thinkingEffort === binding.model.thinkingEffort;
    } catch (_) { return false; }
  }
  function owns(binding) {
    try {
      const owner = owners(binding.node, binding.runtime), auth = binding.runtime.auth;
      const identity = auth.getBrowserChatGptAuthSnapshot();
      return binding.node.isConnected && page.__elonChatGptDocumentToken === binding.token &&
        page.location.href === binding.href && owner?.id === binding.id && owner.scope.node === binding.scope.node &&
        !auth.isBrowserWorkspaceSwitchPending() && !auth.isBrowserAccountSwitchLoading() &&
        auth.getBrowserChatGptAuthGeneration() === binding.generation &&
        identity?.accountId === binding.accountId && identity?.userId === binding.userId &&
        binding.scope.get(binding.runtime.identity.d) === binding.accountId &&
        binding.scope.get(binding.runtime.identity.i) === binding.userId;
    } catch (_) { return false; }
  }
  function visibleComposers() {
    const selectors = ['#prompt-textarea', '[data-testid="prompt-textarea"]', 'form [contenteditable="true"]',
      'form textarea', 'main [contenteditable="true"]', 'textarea[placeholder]'];
    const candidates = new Set(selectors.flatMap(selector => Array.from(page.document.querySelectorAll(selector))));
    return [...candidates].filter(node => {
      if (!node.isConnected) return false;
      const rect = node.getBoundingClientRect(), style = page.getComputedStyle(node);
      return !!rect.width && !!rect.height && style.display !== 'none' && style.visibility !== 'hidden';
    });
  }
  function requestCurrent(binding, serverId) {
    try {
      if (!new RegExp('^' + UUID + '$', 'i').test(serverId || '') ||
          binding.scope.get(binding.runtime.conversation.i, binding.id) !== serverId) return false;
      const allowedUrl = new URL(binding.href);
      allowedUrl.pathname = '/c/' + serverId;
      // The official server-ID callback may remount the editor and change its
      // committed owner. Only that exact alias, scope and account may continue.
      const next = visibleComposers().map(node => capture(node, true)).filter(Boolean);
      return next.length === 1 && ['token', 'accountId', 'userId', 'generation'].every(key => next[0][key] === binding[key]) &&
        next[0].scope.node === binding.scope.node && next[0].serverId === serverId &&
        [binding.id, serverId].includes(next[0].id) &&
        [binding.href, allowedUrl.href].includes(next[0].href);
    } catch (_) { return false; }
  }
  function refreshOwner(binding) {
    try {
      const next = visibleComposers().map(node => capture(node, true)).filter(Boolean);
      if (next.length !== 1 || next[0].scope.node !== binding.scope.node ||
          !['id', 'token', 'href', 'serverId', 'accountId', 'userId', 'generation']
            .every(key => next[0][key] === binding[key])) return null;
      return { ...binding, node: next[0].node };
    } catch (_) { return null; }
  }
  function ownedRequestCurrent(binding, serverId) {
    try {
      const auth = binding.runtime.auth, identity = auth.getBrowserChatGptAuthSnapshot();
      const alias = binding.scope.get(binding.runtime.conversation.i, binding.id) || null;
      if (serverId && (alias !== serverId || !new RegExp('^' + UUID + '$', 'i').test(serverId))) return false;
      // Exclusive group hosts own a submitted request, not an editor node.
      // This is never used for write admission or a shared personal host.
      // A token refresh may advance auth generation without changing its owner.
      return page.__elonChatGptGroupRequestOwnershipEnabled === true &&
        page.__elonChatGptDocumentToken === binding.token && page.location.href === binding.href &&
        !auth.isBrowserWorkspaceSwitchPending() && !auth.isBrowserAccountSwitchLoading() &&
        identity?.accountId === binding.accountId && identity?.userId === binding.userId &&
        binding.scope.get(binding.runtime.identity.d) === binding.accountId &&
        binding.scope.get(binding.runtime.identity.i) === binding.userId &&
        binding.scope.get(binding.runtime.identity.j)?.status === 'allowed';
    } catch (_) { return false; }
  }
  function find(uploads) {
    const bindings = [];
    code = 'composer_detached';
    for (const node of visibleComposers()) {
      // Selector matches alone are not ownership: every candidate must belong
      // to the current committed account and conversation before any upload.
      const binding = capture(node, false, uploads);
      if (binding) bindings.push(binding);
    }
    if (bindings.length > 1) return fail('composer_ambiguous');
    if (!bindings.length) return null;
    code = 'ready'; return bindings[0];
  }
  return Object.freeze({ find, capture: (node, uploads) => capture(node, false, uploads), read: node => capture(node, true),
    current, owns, requestCurrent, ownedRequestCurrent, refreshOwner, state: () => ({ code }) });
});
