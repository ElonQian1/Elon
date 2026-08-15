(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  if (root && root.location && root.location.origin === 'https://chatgpt.com') {
    root.__elonChatGptConversationHistory = Object.freeze(api);
  }
})(typeof window !== 'undefined' ? window : globalThis, function () {
  'use strict';

  function boundedInteger(value, fallback, minimum, maximum) {
    const parsed = Number(value);
    if (!Number.isFinite(parsed)) return fallback;
    return Math.min(maximum, Math.max(minimum, Math.floor(parsed)));
  }

  function conversationKey(conversation) {
    if (!conversation) return '';
    const id = typeof conversation.id === 'string' ? conversation.id.trim() : '';
    if (id) return id;
    const path = typeof conversation.path === 'string' ? conversation.path : '';
    return path.split('/').filter(Boolean).pop() || '';
  }

  function mergeConversation(previous, conversation) {
    const nextHasProject = !!conversation.projectId;
    const previousHasProject = !!previous.projectId;
    return Object.assign({}, previous, conversation, {
      path: nextHasProject || !previousHasProject ? conversation.path : previous.path,
      projectId: conversation.projectId || previous.projectId || null,
      projectTitle: conversation.projectTitle || previous.projectTitle || null,
      projectPath: conversation.projectPath || previous.projectPath || null,
      active: !!previous.active || !!conversation.active,
      activityDates: Array.from(new Set(
        [].concat(previous.activityDates || [], conversation.activityDates || [])
      ))
    });
  }

  function mergeConversations(target, conversations, maximum) {
    (Array.isArray(conversations) ? conversations : []).some((conversation) => {
      if (!conversation || typeof conversation.path !== 'string') {
        return false;
      }
      const key = conversationKey(conversation);
      if (!key) return false;
      const previous = target.get(key);
      if (previous) {
        target.set(key, mergeConversation(previous, conversation));
        return false;
      }
      target.set(key, conversation);
      return target.size >= maximum;
    });
  }

  function dispatchScroll(scroller) {
    if (!scroller || typeof scroller.dispatchEvent !== 'function') return;
    try {
      scroller.dispatchEvent(new Event('scroll', { bubbles: true }));
    } catch {
      // Some test and older WebView environments do not expose an Event constructor.
    }
  }

  function collect(options, onDone) {
    const read = typeof options.read === 'function' ? options.read : () => [];
    const schedule = typeof options.schedule === 'function'
      ? options.schedule
      : (callback, delayMs) => setTimeout(callback, delayMs);
    const now = typeof options.now === 'function' ? options.now : Date.now;
    const maximum = boundedInteger(options.maximum, 100, 1, 100);
    const stallTimeoutMs = boundedInteger(options.timeoutMs, 5000, 500, 30000);
    const absoluteTimeoutMs = boundedInteger(
      options.absoluteTimeoutMs,
      30000,
      stallTimeoutMs,
      60000
    );
    const delayMs = boundedInteger(options.delayMs, 180, 20, 1000);
    const maxSteps = boundedInteger(options.maxSteps, 40, 1, 80);
    const stablePassesRequired = boundedInteger(options.stablePasses, 3, 1, 8);
    const collected = new Map();
    const scroller = typeof options.findScroller === 'function' ? options.findScroller() : null;
    const originalScrollTop = scroller && Number.isFinite(Number(scroller.scrollTop))
      ? Number(scroller.scrollTop)
      : 0;
    const startedAt = now();
    let lastProgressAt = startedAt;
    let steps = 0;
    let stablePasses = 0;
    let previousCount = 0;
    let previousMaxTop = -1;
    let scrolled = false;
    let finished = false;

    mergeConversations(collected, options.initial, maximum);

    function finish(reachedEnd, timedOut) {
      if (finished) return;
      finished = true;
      let scrollRestored = !scroller;
      if (scroller) {
        try {
          scroller.scrollTop = originalScrollTop;
          dispatchScroll(scroller);
          scrollRestored = Math.abs(Number(scroller.scrollTop) - originalScrollTop) <= 1;
        } catch {
          scrollRestored = false;
        }
      }
      const conversations = Array.from(collected.values()).slice(0, maximum);
      onDone({
        conversations,
        collection: {
          scrollerFound: !!scroller,
          scrolled,
          scrollRestored,
          reachedEnd: !!reachedEnd,
          truncated: collected.size >= maximum,
          timedOut: !!timedOut,
          observedCount: conversations.length,
          steps
        }
      });
    }

    function tick() {
      if (finished) return;
      mergeConversations(collected, read(), maximum);
      if (collected.size >= maximum) return finish(false, false);
      if (!scroller) return finish(false, false);

      const scrollTop = Math.max(0, Number(scroller.scrollTop) || 0);
      const clientHeight = Math.max(0, Number(scroller.clientHeight) || 0);
      const scrollHeight = Math.max(clientHeight, Number(scroller.scrollHeight) || 0);
      const maxTop = Math.max(0, scrollHeight - clientHeight);
      const grew = collected.size > previousCount || maxTop > previousMaxTop + 1;
      const atEnd = maxTop <= 1 || scrollTop >= maxTop - 2;
      const observedAt = now();

      stablePasses = atEnd && !grew ? stablePasses + 1 : 0;
      if (grew) lastProgressAt = observedAt;
      previousCount = collected.size;
      previousMaxTop = maxTop;

      if (atEnd && stablePasses >= stablePassesRequired) return finish(true, false);
      if (
        observedAt - lastProgressAt >= stallTimeoutMs ||
        observedAt - startedAt >= absoluteTimeoutMs
      ) return finish(false, true);
      if (steps >= maxSteps) return finish(false, false);

      if (!atEnd) {
        const distance = Math.max(240, Math.floor(clientHeight * 0.8));
        const nextTop = Math.min(maxTop, scrollTop + distance);
        scroller.scrollTop = nextTop;
        scrolled = scrolled || nextTop > originalScrollTop + 1;
      }
      dispatchScroll(scroller);
      steps += 1;
      schedule(tick, delayMs);
    }

    tick();
  }

  return Object.freeze({ collect, mergeConversations });
});
