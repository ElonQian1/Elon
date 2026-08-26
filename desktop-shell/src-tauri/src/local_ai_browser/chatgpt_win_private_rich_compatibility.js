(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (!root) return;
  const current = root.__elonChatGptPrivateStreamPolicy;
  if (!current || typeof current.createSession !== 'function') return;
  const installed = root.__elonWinChatGptPrivateRichCompatibility;
  if (installed && Number(installed.version || 0) >= api.version && installed.policy === current) return;
  const policy = current.__elonWinPrivateRichCompatibilityWrapped === true && installed &&
    installed.basePolicy && typeof installed.basePolicy.createSession === 'function'
    ? installed.basePolicy
    : current;
  if (policy.__elonWinPrivateRichCompatibilityWrapped === true) return;
  const enhanced = api.enhancePolicy(policy);
  root.__elonChatGptPrivateStreamPolicy = Object.freeze(enhanced);
  root.__elonWinChatGptPrivateRichCompatibility = Object.freeze({
    version: api.version,
    basePolicy: policy,
    current: enhanced.richCompatibility,
    policy: enhanced,
  });
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const VERSION = 4;
  const MAX_WIDGET_KEYS = 32;

  function packedWidgetKey(widget) {
    if (!widget || typeof widget !== 'object' || Array.isArray(widget)) return '';
    const compressed = String(widget && widget.compressed || '');
    if (!compressed) return '';
    return [
      String(widget && widget.messageId || ''),
      String(widget && widget.widgetId || ''),
      compressed.length,
      compressed.slice(0, 16),
      compressed.slice(-16),
    ].join(':').slice(0, 460);
  }

  function isSupportedRichPart(part) {
    return Boolean(part && part.type === 'rich_card' && (
      part.kind === 'finance' ||
      part.kind === 'chart' ||
      part.richContent && (
        part.richContent.kind === 'finance' || part.richContent.kind === 'chart'
      )
    ));
  }

  function comparisonText(value) {
    return String(value || '')
      .normalize('NFKC')
      .replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
      .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
      .replace(/<[^>]{1,200}>/g, ' ')
      .replace(/[`*_~>#|\[\]{}-]/g, '')
      .replace(/\s+/g, '')
      .toLowerCase()
      .slice(0, 16000);
  }

  function sameRenderedReply(left, right) {
    const first = comparisonText(left);
    const second = comparisonText(right);
    if (!first || !second) return false;
    if (first === second) return true;
    const shorter = first.length <= second.length ? first : second;
    const longer = first.length > second.length ? first : second;
    return shorter.length >= 24 && longer.includes(shorter) &&
      shorter.length / longer.length >= 0.78;
  }

  function primaryReplyText(message) {
    return (Array.isArray(message && message.content) ? message.content : [])
      .filter((part) => part && (part.type === 'markdown' || part.type === 'text'))
      .map((part) => String(part.text || ''))
      .join('\n');
  }

  function mergeRenderedReply(policy, values, stream) {
    const messages = Array.isArray(values) ? values : [];
    const merged = policy.mergeMessages(messages, stream);
    if (!stream || !stream.text || !Array.isArray(merged) || merged.length <= messages.length) {
      return merged;
    }
    let latestUser = -1;
    for (let index = messages.length - 1; index >= 0; index -= 1) {
      if (messages[index] && messages[index].role === 'user') {
        latestUser = index;
        break;
      }
    }
    let assistantIndex = -1;
    for (let index = messages.length - 1; index > latestUser; index -= 1) {
      const message = messages[index];
      if (message && message.role === 'assistant' &&
          sameRenderedReply(primaryReplyText(message), stream.text)) {
        assistantIndex = index;
        break;
      }
    }
    if (assistantIndex < 0) return merged;

    // Reuse the shared merge implementation after supplying a temporary
    // identity match. This keeps citations and rich cards in the official DOM
    // message instead of appending a second private-stream answer.
    const forcedId = String(stream.id || 'win-rendered-reply');
    const originalId = messages[assistantIndex].id;
    const forcedMessages = messages.slice();
    forcedMessages[assistantIndex] = Object.assign({}, messages[assistantIndex], { id: forcedId });
    const forced = policy.mergeMessages(
      forcedMessages,
      Object.assign({}, stream, { id: forcedId }),
    );
    if (!Array.isArray(forced) || forced.length !== messages.length) return merged;
    const restored = Object.assign({}, forced[assistantIndex]);
    if (originalId === undefined) delete restored.id;
    else restored.id = originalId;
    forced[assistantIndex] = restored;
    return forced;
  }

  function rendererUpgradePart() {
    return {
      type: 'interactive',
      text: '官网富内容已升级',
      kind: 'renderer_upgrade_required',
    };
  }

  function createTracker() {
    const widgetKeys = new Set();
    let convertedCount = 0;

    function observeWidgets(values) {
      (Array.isArray(values) ? values : []).forEach((widget) => {
        const key = packedWidgetKey(widget);
        if (!key || widgetKeys.has(key)) return;
        if (widgetKeys.size >= MAX_WIDGET_KEYS) widgetKeys.clear();
        widgetKeys.add(key);
      });
      return Array.isArray(values) ? values : [];
    }

    function converted(part) {
      if (isSupportedRichPart(part)) convertedCount += 1;
      return part;
    }

    function reset() {
      widgetKeys.clear();
      convertedCount = 0;
    }

    function snapshot() {
      return Object.freeze({
        packedWidgetCount: widgetKeys.size,
        convertedWidgetCount: convertedCount,
        rendererUpgradeRequired: widgetKeys.size > 0 && convertedCount === 0,
      });
    }

    return Object.freeze({ converted, observeWidgets, reset, snapshot });
  }

  function withCompatibility(active, tracker) {
    if (!active || active.state !== 'completed') return active;
    const richParts = Array.isArray(active.richParts) ? active.richParts : [];
    if (richParts.some(isSupportedRichPart)) return active;
    const status = tracker.snapshot();
    if (!status.rendererUpgradeRequired) return active;
    if (richParts.some((part) => (
      part && part.type === 'interactive' && part.kind === 'renderer_upgrade_required'
    ))) return active;
    return Object.assign({}, active, {
      richParts: richParts.concat([rendererUpgradePart()]),
    });
  }

  function enhanceSession(policy, session, tracker, publishCompatibility) {
    function publish() {
      publishCompatibility(tracker.snapshot());
    }

    function accept(payload) {
      const accepted = session.accept(payload);
      let widgets = [];

      // The official SSE can deliver a packed interactive widget in its own
      // frame, after the assistant text frame. The shared session parser
      // intentionally rejects that frame because it has no message body, but
      // the transport only attempts widget decoding after accept() succeeds.
      // Keep this Win-only compatibility bridge narrow: only release frames
      // that the upstream policy positively identifies as finance widgets.
      try {
        widgets = typeof policy.packedFinanceWidgets === 'function'
          ? policy.packedFinanceWidgets(payload)
          : [];
      } catch (_) {
        widgets = [];
      }
      tracker.observeWidgets(widgets);
      publish();
      return accepted || widgets.length > 0;
    }

    function begin() {
      tracker.reset();
      const result = session.begin();
      publish();
      return result;
    }

    function reset() {
      tracker.reset();
      const result = session.reset();
      publish();
      return result;
    }

    function acceptRichParts(parts, identity) {
      tracker.observeWidgets(identity ? [identity] : []);
      const accepted = typeof session.acceptRichParts === 'function'
        ? session.acceptRichParts(parts, identity)
        : false;
      if (accepted) (Array.isArray(parts) ? parts : []).forEach(tracker.converted);
      publish();
      return accepted;
    }

    function packedWidgets() {
      const values = typeof session.packedWidgets === 'function' ? session.packedWidgets() : [];
      const observed = tracker.observeWidgets(values);
      publish();
      return observed;
    }

    function current(pathname) {
      const active = withCompatibility(session.current(pathname), tracker);
      publish();
      return active;
    }

    function merge(values, pathname) {
      const active = current(pathname);
      return active ? mergeRenderedReply(policy, values, active) : values;
    }

    return Object.freeze(Object.assign({}, session, {
      accept,
      acceptRichParts,
      begin,
      current,
      merge,
      packedWidgets,
      reset,
    }));
  }

  function enhancePolicy(policy) {
    if (!policy || typeof policy.createSession !== 'function' ||
        typeof policy.mergeMessages !== 'function') return policy;
    if (policy.__elonWinPrivateRichCompatibilityWrapped === true) return policy;
    let latestCompatibility = createTracker().snapshot();

    return Object.assign({}, policy, {
      __elonWinPrivateRichCompatibilityWrapped: true,
      createSession(options) {
        const tracker = createTracker();
        latestCompatibility = tracker.snapshot();
        return enhanceSession(
          policy,
          policy.createSession(options || {}),
          tracker,
          (snapshot) => { latestCompatibility = snapshot; },
        );
      },
      richCompatibility: () => latestCompatibility,
    });
  }

  return Object.freeze({
    version: VERSION,
    enhancePolicy,
    mergeRenderedReply,
    packedWidgetKey,
    rendererUpgradePart,
    sameRenderedReply,
  });
});
