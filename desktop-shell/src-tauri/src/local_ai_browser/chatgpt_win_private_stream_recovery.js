(function () {
  'use strict';

  var VERSION = 2;
  var MAX_AGE_MS = 5 * 60 * 1000;
  var MAX_RICH_PARTS = 4;
  var MAX_SERIALIZED_BYTES = 512 * 1024;
  var base = window.__elonChatGptPrivateStreamTransport;
  if (!base || typeof base.mergeMessages !== 'function') return;

  var previous = window.__elonWinChatGptPrivateStreamRecovery;
  if (previous && Number(previous.version) >= VERSION && previous.transport === base) return;
  if (base.__elonWinRichRecoveryWrapped === true) {
    if (!previous || !previous.baseTransport) return;
    base = previous.baseTransport;
  }
  if (previous && typeof previous.detach === 'function') {
    try { previous.detach(); } catch (_) {}
  }

  var listeners = new Set();
  var recovered = null;
  var disposed = false;
  var baseUnsubscribe = null;

  function cleanText(value, limit) {
    return String(value || '').replace(/\s+/g, ' ').trim().slice(0, limit || 240);
  }

  function activeConversationId(pathname) {
    var match = String(pathname || '').match(/^\/c\/([^/?#]+)/);
    if (!match) return '';
    try { return decodeURIComponent(match[1]).slice(0, 180); }
    catch (_) { return String(match[1] || '').slice(0, 180); }
  }

  function cloneJson(value) {
    try {
      var serialized = JSON.stringify(value);
      if (!serialized || serialized.length > MAX_SERIALIZED_BYTES) return null;
      return JSON.parse(serialized);
    } catch (_) {
      return null;
    }
  }

  function richKey(part) {
    var rich = part && part.richContent;
    return cleanText(part && (part.kind || rich && rich.kind), 32).toLowerCase() + ':' +
      cleanText(part && (part.text || rich && rich.payload && rich.payload.title), 240).toLowerCase();
  }

  function validRichPart(part) {
    if (!part || part.type !== 'rich_card') return null;
    var rich = part.richContent;
    var kind = cleanText(part.kind || rich && rich.kind, 32).toLowerCase();
    if (kind !== 'finance' && kind !== 'chart') return null;
    if (!rich || rich.schema !== 'yilong.rich-content.v1' || rich.kind !== kind ||
        rich.source !== 'private_response' || !rich.payload || typeof rich.payload !== 'object') return null;
    var value = cloneJson(part);
    if (!value) return null;
    value.type = 'rich_card';
    value.kind = kind;
    value.text = cleanText(value.text || value.richContent.payload.title ||
      (kind === 'finance' ? '市场行情' : '图表'), 240);
    value.richContent.schema = 'yilong.rich-content.v1';
    value.richContent.kind = kind;
    value.richContent.source = 'private_response';
    return value;
  }

  function sanitizeRichParts(parts) {
    var values = [];
    var keys = new Set();
    (Array.isArray(parts) ? parts : []).some(function (part) {
      var value = validRichPart(part);
      if (!value) return false;
      var key = richKey(value);
      if (!key || keys.has(key)) return false;
      keys.add(key);
      values.push(value);
      return values.length >= MAX_RICH_PARTS;
    });
    return values;
  }

  function currentBase(pathname) {
    try { return typeof base.current === 'function' ? base.current(pathname) : null; }
    catch (_) { return null; }
  }

  function visibleNode(node) {
    if (!node) return false;
    try {
      var rect = node.getBoundingClientRect();
      var style = window.getComputedStyle(node);
      return rect.width > 0 && rect.height > 0 &&
        style.display !== 'none' && style.visibility !== 'hidden';
    } catch (_) {
      return false;
    }
  }

  function safeProgressLabel(value) {
    var label = cleanText(value, 220).replace(/^[\s🌐🔍]+/u, '');
    if (!label) return '';
    return /^(?:正在(?:搜索|查询|浏览)|searching|browsing|looking up)\b/i.test(label) ||
      /^(?:正在(?:搜索|查询|浏览))/.test(label)
      ? label
      : '';
  }

  function officialStreamingActive() {
    try {
      var direct = document.querySelector(
        '[data-testid="stop-button"], button[data-testid*="stop" i], ' +
        'main [data-is-streaming="true"], main [data-streaming="true"], main .result-streaming'
      );
      return visibleNode(direct);
    } catch (_) {
      return false;
    }
  }

  function domProgressLabel() {
    var candidates = [];
    try {
      candidates = Array.from(document.querySelectorAll(
        'main [role="status"], main [aria-live], main [data-testid*="reason" i], ' +
        'main [data-testid*="search" i]'
      )).slice(-80);
      var main = document.querySelector('main');
      var turns = main ? Array.from(main.querySelectorAll('[data-testid^="conversation-turn-"]')) : [];
      var latest = turns.length ? turns[turns.length - 1] : null;
      if (latest) {
        candidates = candidates.concat(Array.from(latest.querySelectorAll('div, p, span')).slice(-160));
      }
    } catch (_) {
      return '';
    }
    for (var index = candidates.length - 1; index >= 0; index -= 1) {
      var node = candidates[index];
      if (!visibleNode(node)) continue;
      var label = safeProgressLabel(node.innerText || node.textContent);
      if (label) return label;
    }
    return '';
  }

  function currentWithProgress(pathname) {
    var active = currentBase(pathname);
    if (active && (active.state !== 'streaming' || cleanText(active.progressLabel, 220))) return active;
    var label = domProgressLabel();
    if (!label) return active;
    if (active) return Object.assign({}, active, { progressLabel: label });
    if (!officialStreamingActive()) return null;
    return {
      id: '',
      turnId: '',
      conversationId: activeConversationId(pathname),
      text: '',
      progressLabel: label,
      state: 'streaming',
      richParts: [],
      __elonWinProgressOnly: true,
      updatedAt: Date.now()
    };
  }

  function identityMatches(candidate, reference) {
    if (!candidate || !reference) return false;
    var compared = false;
    if (candidate.conversationId && reference.conversationId) {
      compared = true;
      if (candidate.conversationId !== reference.conversationId) return false;
    }
    if (candidate.turnId && reference.turnId) {
      compared = true;
      if (candidate.turnId !== reference.turnId) return false;
    }
    if (candidate.messageId && reference.id) {
      compared = true;
    }
    if (candidate.messageId && reference.id && candidate.messageId !== reference.id) {
      var sameTurn = candidate.turnId && reference.turnId && candidate.turnId === reference.turnId;
      if (!sameTurn) return false;
    }
    return compared;
  }

  function usable(pathname) {
    if (!recovered || Date.now() - recovered.updatedAt > MAX_AGE_MS) {
      recovered = null;
      return null;
    }
    var routeConversationId = activeConversationId(pathname);
    if (routeConversationId && recovered.conversationId &&
        routeConversationId !== recovered.conversationId) return null;
    var active = currentBase(pathname);
    if (!routeConversationId && !identityMatches(recovered, active)) return null;
    if (active && !identityMatches(recovered, active)) return null;
    return recovered;
  }

  function notify() {
    listeners.forEach(function (listener) {
      try { listener(); } catch (_) {}
    });
  }

  function accept(snapshot) {
    if (disposed || !snapshot || typeof snapshot !== 'object') return false;
    var values = sanitizeRichParts(snapshot.richParts);
    if (!values.length) return false;
    var candidate = {
      messageId: cleanText(snapshot.messageId || snapshot.id, 180),
      turnId: cleanText(snapshot.turnId, 180),
      conversationId: cleanText(snapshot.conversationId, 180),
      richParts: values,
      updatedAt: Date.now()
    };
    var routeConversationId = activeConversationId(location.pathname);
    var active = currentBase(location.pathname);
    if (routeConversationId && candidate.conversationId &&
        routeConversationId !== candidate.conversationId) return false;
    if (!routeConversationId && !identityMatches(candidate, active)) return false;
    if (active && !identityMatches(candidate, active)) return false;
    recovered = candidate;
    notify();
    return true;
  }

  function mergeRichParts(content, recovery) {
    var values = Array.isArray(content) ? content.slice() : [];
    var additions = recovery.richParts.slice();
    var recoveredKeys = new Set(additions.map(richKey));
    var privateKeys = new Set();

    values.forEach(function (part) {
      if (part && part.type === 'rich_card' && part.richContent &&
          part.richContent.source === 'private_response') privateKeys.add(richKey(part));
    });

    values = values.filter(function (part) {
      if (!part) return false;
      var type = cleanText(part.type, 40).toLowerCase();
      var key = richKey(part);
      if (type === 'rich_card' && recoveredKeys.has(key)) {
        return part.richContent && part.richContent.source === 'private_response';
      }
      if (type !== 'interactive' && type !== 'artifact' && type !== 'chart') return true;
      var kind = cleanText(part.kind || part.richContent && part.richContent.kind, 40).toLowerCase();
      var renderer = cleanText(part.renderer || part.rendererKind || part.reason, 80).toLowerCase();
      return !(['finance', 'chart', 'renderer_upgrade_required'].includes(kind) ||
        renderer.indexOf('renderer_upgrade_required') >= 0 || recoveredKeys.has(key));
    });

    additions.forEach(function (part) {
      var key = richKey(part);
      if (!privateKeys.has(key)) values.push(cloneJson(part));
    });
    return values;
  }

  function enrichMessages(messages, pathname) {
    var recovery = usable(pathname);
    if (!recovery || !Array.isArray(messages)) return messages;
    var assistantIndex = -1;
    for (var index = messages.length - 1; index >= 0; index -= 1) {
      if (messages[index] && messages[index].role === 'assistant') {
        assistantIndex = index;
        break;
      }
    }
    if (assistantIndex < 0) return messages;
    var result = messages.slice();
    var assistant = Object.assign({}, result[assistantIndex]);
    assistant.content = mergeRichParts(assistant.content, recovery);
    result[assistantIndex] = assistant;
    return result;
  }

  var transport = Object.freeze({
    version: Number(base.version || 0),
    enabled: base.enabled !== false,
    __elonWinRichRecoveryWrapped: true,
    current: function (pathname) {
      var active = currentWithProgress(pathname);
      if (active && active.__elonWinProgressOnly === true) return active;
      var recovery = usable(pathname);
      if (!active || !recovery) return active;
      var value = Object.assign({}, active);
      value.richParts = mergeRichParts(active.richParts || [], recovery);
      return value;
    },
    access: function () {
      try { return typeof base.access === 'function' ? base.access() : null; }
      catch (_) { return null; }
    },
    mergeMessages: function (messages, pathname) {
      var merged = messages;
      try { merged = base.mergeMessages(messages, pathname); } catch (_) {}
      return enrichMessages(merged, pathname);
    },
    reset: function () {
      recovered = null;
      try { if (typeof base.reset === 'function') base.reset(); } catch (_) {}
      notify();
    },
    subscribe: function (listener) {
      if (typeof listener !== 'function') return function () {};
      listeners.add(listener);
      return function () { listeners.delete(listener); };
    },
    dispose: function () {
      if (disposed) return;
      disposed = true;
      recovered = null;
      listeners.clear();
      if (typeof baseUnsubscribe === 'function') baseUnsubscribe();
      baseUnsubscribe = null;
      try { if (typeof base.dispose === 'function') base.dispose(); } catch (_) {}
    }
  });

  if (typeof base.subscribe === 'function') {
    try { baseUnsubscribe = base.subscribe(notify); } catch (_) {}
  }
  var recoveryApi = Object.freeze({
    version: VERSION,
    transport: transport,
    baseTransport: base,
    accept: accept,
    detach: function () {
      if (typeof baseUnsubscribe === 'function') baseUnsubscribe();
      baseUnsubscribe = null;
      listeners.clear();
    }
  });
  window.__elonChatGptPrivateStreamTransport = transport;
  window.__elonWinChatGptPrivateStreamRecovery = recoveryApi;
})();
