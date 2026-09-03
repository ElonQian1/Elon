(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  if (root && !root.__elonChatGptProjectChoiceReveal) {
    root.__elonChatGptProjectChoiceReveal = api;
  }
})(typeof window !== 'undefined' ? window : globalThis, function () {
  'use strict';

  const CHOICE_ROLES = new Set(['button', 'menuitem', 'menuitemradio', 'option', 'radio']);
  const MAX_SCROLL_STEPS = 24;
  const SCROLL_SETTLE_MS = 120;

  function normalizedLabel(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim().toLowerCase();
  }

  function scrollPositions(container) {
    const maximum = Math.max(0, Number(container.scrollHeight) - Number(container.clientHeight));
    if (maximum <= 1) return [];
    const step = Math.max(80, Math.floor(Number(container.clientHeight) * 0.72));
    const positions = [0];
    for (let value = step; value < maximum && positions.length < MAX_SCROLL_STEPS - 1; value += step) {
      positions.push(Math.min(maximum, value));
    }
    if (positions[positions.length - 1] !== maximum) positions.push(maximum);
    return positions;
  }

  function isScrollable(node, isVisible) {
    return !!node && node.nodeType === 1 && node.tagName !== 'BODY' && node.tagName !== 'HTML' &&
      isVisible(node) && Number(node.scrollHeight) > Number(node.clientHeight) + 8;
  }

  function create(options) {
    const actionableNodes = options.actionableNodes;
    const visibleOverlayRoots = options.visibleOverlayRoots;
    const isVisible = options.isVisible;
    const labelOf = options.labelOf;
    const roleOf = options.roleOf;
    const setTimer = options.setTimeout || ((callback, delay) => setTimeout(callback, delay));
    const createScrollEvent = options.createScrollEvent || (() => new Event('scroll', { bubbles: true }));

    function choices(rootNode) {
      return actionableNodes(rootNode).filter((node) => CHOICE_ROLES.has(roleOf(node)));
    }

    function exactChoice(expected) {
      const matches = visibleOverlayRoots().flatMap(choices).filter((node) =>
        normalizedLabel(labelOf(node, '')) === expected
      );
      return matches.length === 1 ? matches[0] : null;
    }

    function findScrollContainer() {
      const roots = visibleOverlayRoots();
      const projectNodes = roots.flatMap(choices);
      const candidates = new Map();
      projectNodes.forEach((node) => {
        let current = node.parentElement;
        for (let depth = 0; current && depth < 10; depth += 1, current = current.parentElement) {
          if (isScrollable(current, isVisible)) {
            candidates.set(current, (candidates.get(current) || 0) + 1);
          }
          if (current.tagName === 'BODY' || current.tagName === 'HTML') break;
        }
      });
      roots.forEach((overlay) => {
        if (isScrollable(overlay, isVisible) && !candidates.has(overlay)) candidates.set(overlay, 0);
        Array.from(overlay.querySelectorAll('*')).forEach((node) => {
          if (isScrollable(node, isVisible) && !candidates.has(node)) candidates.set(node, 0);
        });
      });
      return Array.from(candidates.entries()).sort((left, right) => {
        if (right[1] !== left[1]) return right[1] - left[1];
        const leftRange = Number(left[0].scrollHeight) - Number(left[0].clientHeight);
        const rightRange = Number(right[0].scrollHeight) - Number(right[0].clientHeight);
        return rightRange - leftRange;
      })[0]?.[0] || null;
    }

    function applyScroll(container, position) {
      if (typeof container.scrollTo === 'function') container.scrollTo(0, position);
      else container.scrollTop = position;
      try { container.dispatchEvent(createScrollEvent()); } catch {}
    }

    function reveal(label, onChanged, done) {
      const expected = normalizedLabel(label);
      if (!expected) return done(false, 'invalid_project_choice');
      const current = exactChoice(expected);
      if (current) {
        current.scrollIntoView({ block: 'nearest', inline: 'nearest' });
        onChanged();
        return done(true, 'project_choice_revealed');
      }
      const container = findScrollContainer();
      if (!container) return done(false, 'project_choice_scroll_unavailable');
      const originalPosition = Number(container.scrollTop) || 0;
      const positions = scrollPositions(container);
      let index = 0;

      function next() {
        if (!container.isConnected || index >= positions.length) {
          if (container.isConnected) applyScroll(container, originalPosition);
          onChanged();
          done(false, 'project_choice_not_rendered');
          return;
        }
        applyScroll(container, positions[index]);
        index += 1;
        setTimer(() => {
          const match = exactChoice(expected);
          if (match) {
            match.scrollIntoView({ block: 'nearest', inline: 'nearest' });
            onChanged();
            done(true, 'project_choice_revealed');
            return;
          }
          next();
        }, SCROLL_SETTLE_MS);
      }

      next();
    }

    return Object.freeze({ reveal });
  }

  return Object.freeze({ create, normalizedLabel, scrollPositions });
});
