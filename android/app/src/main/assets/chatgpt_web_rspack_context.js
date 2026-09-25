(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
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
  function capture(node, reading = false) {
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
    if (pathId ? serverId !== pathId : serverId !== null || !id.startsWith('local-chatgpt:')) return fail('conversation_mismatch');
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
    if (!Array.isArray(uploads) || uploads.length || !Array.isArray(hints) || hints.length) return fail('attachments_or_tools_present');
    if (typeof draft !== 'string' || typeof model?.slug !== 'string' || !model.slug ||
        model.thinkingEffort != null && typeof model.thinkingEffort !== 'string') return fail('composer_state_pending');
    code = 'ready';
    return { ...binding, draft, model: { ...model } };
  }
  function current(binding) {
    try {
      const next = capture(binding.node);
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
  return Object.freeze({ capture: node => capture(node), read: node => capture(node, true),
    current, owns, state: () => ({ code }) });
});
