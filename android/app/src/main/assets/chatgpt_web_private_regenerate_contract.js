(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 9, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateRegenerateContract = api;
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const URLS = Object.freeze({
    shared: 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js',
    conversation: 'https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js',
    composer: 'https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js'
  });
  const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const SLUG = /^[a-z0-9][a-z0-9._-]{0,127}$/i;
  const models = page.__elonChatGptPrivateModelContract?.create(page);
  const ownerPath = page.__elonChatGptCommittedOwnerPath ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_committed_owner_path') : null);

  function ancestors(node) {
    if (!node?.isConnected) return [];
    const key = Object.keys(node).find(name => name.startsWith('__reactFiber$'));
    return ownerPath?.resolve(node[key])?.ancestors || [];
  }

  function retryMenu(turn) {
    if (!turn?.isConnected) return null;
    const nodes = Array.from(turn.querySelectorAll('button, [role="button"]'));
    if (nodes.length > 64) return null;
    const candidates = new Set();
    for (const node of nodes) {
      const chain = ancestors(node);
      const index = chain.findIndex(f => f.memoizedProps?.conversation &&
        f.memoizedProps?.lastMessage && typeof f.memoizedProps?.onModelSelect === 'function');
      if (index < 0) continue;
      const owner = chain[index].memoizedProps, seen = new Set();
      const queue = chain.slice(0, index + 1).map(f => ({ props: f.memoizedProps, depth: 0 }));
      // A closed Radix portal still has committed React element children. Do not
      // execute components/hooks or inspect memo-cache slot numbers to open it.
      for (let count = 0; queue.length && count < 256; count++) {
        const { props, depth } = queue.shift();
        if (!props || seen.has(props)) continue;
        seen.add(props);
        if (props.retryOption && props.conversation === owner.conversation &&
            props.lastMessage === owner.lastMessage && props.onModelSelect === owner.onModelSelect) candidates.add(props);
        if (depth >= 12) continue;
        const children = Array.isArray(props.children) ? props.children : [props.children];
        if (children.length > 64) return null;
        for (const child of children) {
          if (child?.$$typeof === Symbol.for('react.transitional.element') ||
              child?.$$typeof === Symbol.for('react.element')) queue.push({ props: child.props, depth: depth + 1 });
        }
      }
      if (queue.length) return null;
    }
    return candidates.size === 1 ? candidates.values().next().value : null;
  }

  function capture(turn, getModelTrigger) {
    if (!models || !Object.values(URLS).every(url =>
      page.__elonChatGptPrivateRuntimeBindings ? page.__elonChatGptPrivateRuntimeBindings.observed(url) :
      page.performance?.getEntriesByName?.(url, 'resource')?.length > 0 ||
      page.document.querySelector('link[rel="modulepreload"][href="' + url + '"]'))) return null;
    const model = models.capture(getModelTrigger), menu = retryMenu(turn);
    const message = menu?.lastMessage, option = menu?.retryOption;
    const cid = model?.conversation.serverId$();
    if (!model || !UUID.test(cid || '') || !menu || menu.conversation !== model.conversation ||
        menu.canRegenerateResponse !== true || menu.hasImageGenMessage !== false ||
        message?.author?.role !== 'assistant' || message?.content?.content_type !== 'text' ||
        !['finished_successfully', 'finished_partial_completion'].includes(message.status) ||
        message.metadata?.image_gen_async ||
        !UUID.test(message.id || '') || !SLUG.test(option?.value || '') ||
        option.shouldShowUpsell === true || option.disabled === true) return null;
    return { turn, getModelTrigger, model, conversation: menu.conversation, menu,
      message, callback: menu.onModelSelect, modelSlug: option.value, cid, token: model.token };
  }

  function current(binding) {
    if (binding.observationOwner && !models.ownerCurrent(binding.observationOwner)) return null;
    const now = capture(binding.turn, binding.getModelTrigger);
    return now && ['cid', 'token', 'conversation', 'message', 'callback', 'modelSlug'].every(key =>
      now[key] === binding[key]) && now.model.account === binding.model.account &&
      now.model.href === binding.model.href ? now : null;
  }

  function ownerCurrent(binding) {
    return models.ownerCurrent(binding.observationOwner || binding.model) && binding.model.conversation === binding.conversation;
  }

  function validate(modules) {
    return typeof modules?.shared?.XM === 'function' &&
      ['H3', 'F5', 'mq'].every(key => typeof modules.shared[key] === 'function') &&
      ['getCurrentLeafId', 'getParentPromptNode', 'getVariantIds', 'getNode'].every(key =>
        typeof modules.shared.HM?.[key] === 'function') && typeof modules?.conversation?.f8t === 'function';
  }

  function prepare(binding, modules) {
    const now = current(binding);
    if (!now || !validate(modules) || !now.model.menu.modelSwitcherDenialsBySlug ||
        modules.conversation.f8t({ modelSlug: now.modelSlug,
          modelSwitcherDenialsBySlug: now.model.menu.modelSwitcherDenialsBySlug }) !== true) return null;
    const s = modules.shared, tree = s.XM(now.conversation.id);
    const parent = s.HM.getParentPromptNode(tree, now.message.id);
    const variants = s.HM.getVariantIds(tree, now.message.id);
    if (s.HM.getCurrentLeafId(tree) !== now.message.id ||
        !UUID.test(parent?.id || '') || parent?.message?.author?.role !== 'user' ||
        !Array.isArray(variants) || variants.length > 1000 || !variants.includes(now.message.id) ||
        !variants.every(id => UUID.test(id))) return null;
    const observationOwner = models.withRuntimeIdentity(now.model, modules.shared);
    return observationOwner ? { ...now, observationOwner, parentId: parent.id, variants: new Set(variants) } : null;
  }

  function observation(binding, modules, stream) {
    const owner = models.ownerState(binding.observationOwner);
    if (owner !== 'owner_current') return owner;
    if (binding.model.conversation !== binding.conversation) return 'owner_object';
    if (!stream) return 'stream_missing';
    if (stream.conversationId !== binding.cid) return 'conversation_mismatch';
    if (!UUID.test(stream.id || '')) return 'message_invalid';
    if (binding.variants.has(stream.id)) return 'old_variant';
    if (!stream.text) return 'text_pending';
    if (!['streaming', 'completed'].includes(stream.state)) return 'state_unknown';
    try {
      const s = modules.shared, tree = s.XM(binding.conversation.id);
      if (s.HM.getCurrentLeafId(tree) !== stream.id) return 'leaf_pending';
      const node = s.HM.getNode(tree, stream.id);
      if (node?.message?.author?.role !== 'assistant') return 'node_pending';
      const parent = s.HM.getParentPromptNode(tree, stream.id);
      return parent?.id === binding.parentId && parent?.message?.author?.role === 'user'
        ? 'regenerate_observed' : 'parent_mismatch';
    } catch (_) { return 'tree_unavailable'; }
  }

  function observed(binding, modules, stream) {
    return observation(binding, modules, stream) === 'regenerate_observed';
  }

  return Object.freeze({ urls: URLS, capture, current, ownerCurrent, validate, prepare, observed, observation });
});
