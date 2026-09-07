(function (root, resolve) {
  'use strict';
  const api = Object.freeze({ version: 1, resolve });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptCommittedOwnerPath = api;
})(typeof window === 'object' ? window : null, function (start) {
  'use strict';
  const MAX_DEPTH = 512, MAX_VISITS = 1024;
  const paths = new Map(), children = new Map(), visiting = new Set();
  let fault = null, visited = 0, inspected = 0;
  function contains(parent, fiber) {
    if (!children.has(parent)) {
      const set = new Set();
      for (let child = parent.child; child; child = child.sibling) {
        if (set.has(child)) { fault = 'react_owner_cycle'; return false; }
        if (set.size >= 512 || ++inspected > 4096) { fault = 'react_owner_child_limit'; return false; }
        set.add(child);
      }
      children.set(parent, set);
    }
    return children.get(parent).has(fiber);
  }
  function resolve(fiber, depth) {
    if (!fiber || typeof fiber !== 'object' || fault) return null;
    if (depth >= MAX_DEPTH) { fault = 'react_owner_depth_limit'; return null; }
    if (visiting.has(fiber)) { fault = 'react_owner_cycle'; return null; }
    if (paths.has(fiber)) {
      const cached = paths.get(fiber);
      if (cached && depth + cached.length > MAX_DEPTH) { fault = 'react_owner_depth_limit'; return null; }
      return cached;
    }
    if (++visited > MAX_VISITS) { fault = 'react_owner_visit_limit'; return null; }
    if (!fiber.return) {
      const path = fiber.stateNode?.current === fiber ? { fiber, next: null, length: 1 } : null;
      paths.set(fiber, path);
      return path;
    }
    visiting.add(fiber);
    let path = null;
    // Bailouts may reuse a child whose return pointer still names the old parent.
    // Prove actual child membership through each branch, up to root.current.
    for (const parent of new Set([fiber.return, fiber.return.alternate])) {
      if (!parent || !contains(parent, fiber)) continue;
      const tail = resolve(parent, depth + 1);
      if (!tail) continue;
      if (path) { fault = 'react_owner_ambiguous'; break; }
      path = { fiber, next: tail, length: tail.length + 1 };
    }
    visiting.delete(fiber);
    paths.set(fiber, path);
    return path;
  }
  let path = null;
  for (const candidate of new Set([start, start?.alternate])) {
    const found = resolve(candidate, 0);
    if (!found) continue;
    if (path) { fault = 'react_owner_ambiguous'; break; }
    path = found;
  }
  if (fault || !path) return { ancestors: [], code: fault || 'react_owner_uncommitted' };
  // Linked memoized records keep deep trees linear in allocation size.
  const ancestors = [];
  for (let link = path; link; link = link.next) ancestors.push(link.fiber);
  return { ancestors, code: '' };
});
