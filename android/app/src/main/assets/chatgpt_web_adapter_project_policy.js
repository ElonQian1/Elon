(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptProjectPolicy = Object.freeze(policy);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  const PROJECT_ID = /(?:^|[^A-Za-z0-9_-])(g-p-[A-Za-z0-9_-]{1,160})(?:[^A-Za-z0-9_-]|$)/;
  const PROJECT_PATH = /(?:^|https:\/\/chatgpt\.com)?\/g\/(g-p-[A-Za-z0-9_-]{1,160})(?:\/project)?(?:[/?#]|$)/i;
  const PROJECT_OPTIONS = /(?:project|\u9879\u76ee).*(?:options?|menu|\u9009\u9879|\u64cd\u4f5c|\u83dc\u5355)|(?:open|\u6253\u5f00).*(?:options?|\u9009\u9879)/i;
  const RESERVED_TITLE = /^(?:projects?|\u9879\u76ee|new project|create project|\u65b0\u5efa\u9879\u76ee|\u65b0\u9879\u76ee|view more|\u67e5\u770b\u66f4\u591a)$/i;

  function clean(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function canonicalPath(id) {
    return id ? '/g/' + id + '/project' : '';
  }

  function projectId(value) {
    const text = String(value || '');
    const route = text.match(PROJECT_PATH);
    if (route) return route[1];
    const direct = text.match(PROJECT_ID);
    return direct ? direct[1] : '';
  }

  function attributeValues(node) {
    if (!node || typeof node.getAttributeNames !== 'function') return [];
    return node.getAttributeNames().slice(0, 40)
      .map((name) => node.getAttribute(name)).filter(Boolean);
  }

  function runtimeProjectId(node) {
    if (!node || typeof node !== 'object') return '';
    const roots = Object.getOwnPropertyNames(node).filter((name) =>
      /^__(?:react|next|remix)/i.test(name)
    ).slice(0, 12).map((name) => {
      const descriptor = Object.getOwnPropertyDescriptor(node, name);
      return descriptor && 'value' in descriptor ? descriptor.value : null;
    }).filter(Boolean);
    const queue = roots.map((value) => ({ value, depth: 0 }));
    const seen = new Set();
    let visited = 0;
    while (queue.length && visited < 240) {
      const entry = queue.shift();
      const value = entry.value;
      if (typeof value === 'string') {
        const id = projectId(value);
        if (id) return id;
        continue;
      }
      if (!value || typeof value !== 'object' || seen.has(value) || entry.depth >= 6) continue;
      seen.add(value);
      visited += 1;
      Object.getOwnPropertyNames(value).slice(0, 50).forEach((name) => {
        const keyId = projectId(name);
        if (keyId) queue.unshift({ value: keyId, depth: entry.depth + 1 });
        const descriptor = Object.getOwnPropertyDescriptor(value, name);
        if (descriptor && 'value' in descriptor) {
          queue.push({ value: descriptor.value, depth: entry.depth + 1 });
        }
      });
    }
    return '';
  }

  function projectIdForNode(node) {
    const candidates = [];
    let current = node;
    for (let depth = 0; current && depth < 7; depth += 1, current = current.parentElement) {
      candidates.push.apply(candidates, attributeValues(current));
    }
    if (node && typeof node.querySelectorAll === 'function') {
      Array.from(node.querySelectorAll('[href], [data-project-id], [data-project-gizmo-id]'))
        .slice(0, 40).forEach((child) => candidates.push.apply(candidates, attributeValues(child)));
    }
    return candidates.map(projectId).find(Boolean) || runtimeProjectId(node);
  }

  function referencedTitle(value) {
    const label = clean(value);
    if (!PROJECT_OPTIONS.test(label)) return '';
    return label
      .replace(/^(?:open|\u6253\u5f00)\s*[\u201c\u201d"']?\s*/i, '')
      .replace(/\s*[\u201c\u201d"']?\s*\u7684?\s*(?:project|\u9879\u76ee)?\s*(?:options?|menu|\u9009\u9879|\u64cd\u4f5c|\u83dc\u5355)$/i, '')
      .replace(/\s*(?:project\s*)?(?:options?|menu)$/i, '')
      .trim();
  }

  function read(document, isVisible, labelOf) {
    if (!document || typeof document.querySelectorAll !== 'function') return [];
    const label = typeof labelOf === 'function' ? labelOf : (node) => clean(node && node.textContent);
    const visible = typeof isVisible === 'function' ? isVisible : () => true;
    const actionable = Array.from(document.querySelectorAll(
      'a[href], button, [role="button"], [role="menuitem"], [role="treeitem"]'
    )).filter(visible);
    const titles = new Map();
    actionable.forEach((node) => {
      const value = clean(label(node));
      if (!value || RESERVED_TITLE.test(value) || PROJECT_OPTIONS.test(value)) return;
      if (!titles.has(value)) titles.set(value, []);
      titles.get(value).push(node);
    });

    const optionsTitles = new Set(actionable.map((node) => referencedTitle(label(node))).filter(Boolean));
    const candidates = [];
    actionable.forEach((node) => {
      const title = clean(label(node));
      const id = projectIdForNode(node);
      if (id && title && !RESERVED_TITLE.test(title) && !PROJECT_OPTIONS.test(title)) {
        candidates.push({ node, id, title });
      }
    });
    Array.from(document.querySelectorAll('*')).slice(0, 6000).forEach((node) => {
      const id = attributeValues(node).map(projectId).find(Boolean) || runtimeProjectId(node);
      if (!id) return;
      const nearby = actionable.find((candidate) => candidate === node || candidate.contains(node) || node.contains(candidate));
      if (!nearby) return;
      const title = clean(label(nearby));
      if (title && !RESERVED_TITLE.test(title) && !PROJECT_OPTIONS.test(title)) {
        candidates.push({ node: nearby, id, title });
      }
    });
    optionsTitles.forEach((title) => {
      (titles.get(title) || []).forEach((node) => {
        const id = projectIdForNode(node);
        if (id) candidates.push({ node, id, title });
      });
    });

    const seen = new Set();
    return candidates.filter((item) => {
      if (!item.id || !item.title || seen.has(item.id)) return false;
      seen.add(item.id);
      return true;
    }).map((item) => ({
      id: item.id,
      title: item.title.slice(0, 160),
      path: canonicalPath(item.id),
      node: item.node
    }));
  }

  function findNode(document, path, isVisible, labelOf) {
    const id = projectId(path);
    if (!id) return null;
    return read(document, isVisible, labelOf).find((project) => project.id === id)?.node || null;
  }

  function unresolved(document, isVisible, labelOf) {
    if (!document || typeof document.querySelectorAll !== 'function') return [];
    const label = typeof labelOf === 'function' ? labelOf : (node) => clean(node && node.textContent);
    const visible = typeof isVisible === 'function' ? isVisible : () => true;
    const actionable = Array.from(document.querySelectorAll('button, [role="button"]')).filter(visible);
    const optionTitles = new Set(actionable.map((node) => referencedTitle(label(node))).filter(Boolean));
    const seen = new Set();
    return actionable.filter((node) => {
      if (!visible(node) || node.getAttribute('aria-expanded') !== null) return false;
      const title = clean(label(node));
      if (!title || !optionTitles.has(title) || RESERVED_TITLE.test(title) || PROJECT_OPTIONS.test(title)) return false;
      if (projectIdForNode(node) || seen.has(title)) return false;
      seen.add(title);
      return true;
    }).map((node) => ({ node, title: clean(label(node)).slice(0, 160) })).slice(0, 20);
  }

  return Object.freeze({
    canonicalPath, findNode, projectId, read, referencedTitle, runtimeProjectId, unresolved
  });
});
