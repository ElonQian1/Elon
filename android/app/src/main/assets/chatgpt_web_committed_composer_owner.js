(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptCommittedComposerOwner = api;
})(typeof window === 'object' ? window : null, function (page, readStores, ownerPath) {
  'use strict';
  let cache = null;

  function roots() {
    const document = page.document, body = document?.body;
    const children = Array.from(body?.children || []);
    if (children.length > 64) return null;
    const result = new Map();
    for (const container of new Set([document, document?.documentElement, body, ...children])) {
      if (!container) continue;
      for (const key of Object.keys(container).filter(key => key.startsWith('__reactContainer$'))) {
        const host = Object.getOwnPropertyDescriptor(container, key)?.value;
        const state = host?.stateNode, current = state?.current;
        // React's container field can retain the initial HostRoot after commits.
        if (!current || state.containerInfo !== container || current.stateNode !== state || current.return) return null;
        result.set(state, current);
        if (result.size > 4) return null;
      }
    }
    return result.size ? result : null;
  }

  function sameRoots(a, b) {
    return a && b && a.size === b.size && [...a].every(([state, current]) => b.get(state) === current);
  }

  function candidates(currentRoots) {
    if (cache?.document === page.document && sameRoots(cache.roots, currentRoots)) return cache.candidates;
    const result = [], visited = new Set(), stack = [...currentRoots.values()];
    while (stack.length) {
      const fiber = stack.pop();
      if (visited.has(fiber) || visited.size >= 8192) return null;
      visited.add(fiber);
      const props = fiber.memoizedProps;
      if (props?.conversation && props.composerController?.conversation === props.conversation) {
        if (result.length >= 64) return null;
        result.push(fiber);
      }
      // Follow only committed child/sibling membership, not alternates or arbitrary object graphs.
      const siblings = new Set();
      for (let child = fiber.child; child; child = child.sibling) {
        if (siblings.has(child) || siblings.size >= 512) return null;
        siblings.add(child); stack.push(child);
      }
    }
    cache = { document: page.document, roots: currentRoots, candidates: result };
    return result;
  }

  function locate(matches, acceptsFiber) {
    try {
      const currentRoots = roots();
      if (!currentRoots) { cache = null; return null; }
      const found = candidates(currentRoots);
      if (!found) { cache = null; return null; }
      let selected = null;
      for (const fiber of found) {
        if (acceptsFiber && !acceptsFiber(fiber)) continue;
        const props = fiber.memoizedProps;
        if (!matches(props.conversation)) continue;
        const path = ownerPath?.resolve(fiber);
        if (!path?.ancestors?.length || path.code) return null;
        const host = path.ancestors[path.ancestors.length - 1];
        if (currentRoots.get(host.stateNode) !== host) return null;
        const context = readStores(path.ancestors), live = context?.shared.getSharedProps();
        if (!context || live?.conversation !== props.conversation ||
            live.composerController !== props.composerController) return null;
        if (selected && (acceptsFiber || ['shared', 'files'].some(key => selected[key] !== context[key]))) return null;
        selected = { ...context, fiber, conversation: props.conversation, controller: props.composerController };
      }
      if (!selected || !sameRoots(currentRoots, roots())) return null;
      const document = page.document;
      return { ...selected, current: () => {
        if (document !== page.document) return false;
        const next = locate(matches, acceptsFiber);
        return next !== null && ['shared', 'files', 'conversation', 'controller'].every(key => next[key] === selected[key]);
      } };
    } catch (_) { cache = null; return null; }
  }

  return Object.freeze({ locate });
});
