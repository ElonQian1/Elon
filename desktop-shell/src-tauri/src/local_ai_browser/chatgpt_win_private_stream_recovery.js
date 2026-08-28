(function () {
  'use strict';

  var VERSION = 8;
  var MAX_AGE_MS = 5 * 60 * 1000;
  var OFFICIAL_COMPLETION_SETTLE_MS = 3000;
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
  var completionOverride = null;
  var disposed = false;
  var baseUnsubscribe = null;
  var conversationGeneration = 0;
  // `base.reset()` records the conversation that must stay blocked after a
  // new-chat boundary. The first prompt in that new conversation must not call
  // `base.prepareSend()`, because the shared transport intentionally clears the
  // blocked conversation there. Later prompts are ordinary turns and must call
  // it so completed text, citations and charts cannot leak into the next answer.
  var newConversationBoundaryPending = false;

  function clearTurnRecovery() {
    conversationGeneration += 1;
    recovered = null;
    completionOverride = null;
  }

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

  function richTitle(part) {
    var rich = part && part.richContent;
    return cleanText(part && (part.text || rich && rich.payload && rich.payload.title), 240)
      .toLowerCase();
  }

  function genericPlaceholderReplacedBy(part, replacements) {
    if (!part || part.type !== 'interactive') return false;
    var kind = cleanText(part.kind, 40).toLowerCase();
    if (kind && kind !== 'interactive') return false;
    var title = richTitle(part);
    if (!title) return false;
    return replacements.some(function (replacement) {
      var replacementKind = cleanText(
        replacement && (replacement.kind || replacement.richContent && replacement.richContent.kind),
        40
      ).toLowerCase();
      return (replacementKind === 'finance' || replacementKind === 'chart') &&
        richTitle(replacement) === title;
    });
  }

  function validRichPart(part) {
    if (!part || typeof part !== 'object') return null;
    var rawKind = cleanText(part.kind, 32).toLowerCase();
    if (part.type === 'interactive' && rawKind === 'renderer_upgrade_required') {
      return {
        type: 'interactive',
        text: cleanText(part.text, 180) || '官网富内容已升级',
        kind: 'renderer_upgrade_required'
      };
    }
    if (part.type !== 'rich_card') return null;
    var rich = part.richContent;
    var kind = cleanText(rawKind || rich && rich.kind, 32).toLowerCase();
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

  function streamIdentity(value) {
    if (!value) return '';
    var conversation = cleanText(value.conversationId, 180);
    var turn = cleanText(value.turnId, 180);
    var message = cleanText(value.id || value.messageId, 180);
    if (!turn && !message) return '';
    return [conversation, turn, message].join(':');
  }

  function primaryMessageText(message) {
    var content = message && Array.isArray(message.content) ? message.content : [];
    return cleanText(content.filter(function (part) {
      return part && (part.type === 'markdown' || part.type === 'text');
    }).map(function (part) { return part.text || ''; }).join(' '), 8000);
  }

  function latestCompletedAssistant(messages) {
    var values = Array.isArray(messages) ? messages : [];
    for (var index = values.length - 1; index >= 0; index -= 1) {
      var message = values[index];
      if (!message || message.role !== 'assistant' || message.state === 'streaming') continue;
      if (primaryMessageText(message)) return message;
    }
    return null;
  }

  function completionTextMatches(active, official) {
    var privateText = cleanText(active && active.text, 8000);
    var officialText = primaryMessageText(official);
    if (!officialText) return false;
    if (!privateText) return Array.isArray(active && active.richParts) && active.richParts.length > 0;
    return privateText === officialText ||
      (Math.min(privateText.length, officialText.length) >= 12 &&
        (privateText.indexOf(officialText) === 0 || officialText.indexOf(privateText) === 0));
  }

  function completedActiveStream(active) {
    if (!active || active.state !== 'streaming' || !completionOverride) return active;
    if (streamIdentity(active) !== completionOverride.identity ||
        Number(active.updatedAt || 0) > completionOverride.streamUpdatedAt ||
        officialStreamingActive()) {
      completionOverride = null;
      return active;
    }
    return Object.assign({}, active, { state: 'completed' });
  }

  function observeOfficialCompletion(messages, pathname) {
    var active = currentBase(pathname);
    if (!active || active.state !== 'streaming') {
      completionOverride = null;
      return;
    }
    var identity = streamIdentity(active);
    var updatedAt = Number(active.updatedAt || 0);
    var official = latestCompletedAssistant(messages);
    if (!identity || !official || officialStreamingActive() ||
        !updatedAt || Date.now() - updatedAt < OFFICIAL_COMPLETION_SETTLE_MS ||
        !completionTextMatches(active, official)) return;
    var changed = !completionOverride || completionOverride.identity !== identity ||
      completionOverride.streamUpdatedAt !== updatedAt;
    completionOverride = { identity: identity, streamUpdatedAt: updatedAt };
    if (changed) notify();
  }

  function applyMergedCompletion(messages, pathname) {
    var active = completedActiveStream(currentBase(pathname));
    if (!active || active.state !== 'completed' || !completionOverride || !Array.isArray(messages)) {
      return messages;
    }
    var result = messages.slice();
    for (var index = result.length - 1; index >= 0; index -= 1) {
      if (!result[index] || result[index].role !== 'assistant') continue;
      result[index] = Object.assign({}, result[index], { state: 'completed' });
      break;
    }
    return result;
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
    var active = completedActiveStream(currentBase(pathname));
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
    if (!routeConversationId && !active && recovered.detached !== true) return null;
    if (!routeConversationId && active && !identityMatches(recovered, active)) return null;
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
    var suppliedGeneration = Number(snapshot.generation);
    var generationBound = Number.isSafeInteger(suppliedGeneration) &&
      suppliedGeneration === conversationGeneration;
    if (Number.isSafeInteger(suppliedGeneration) && !generationBound) return false;
    var candidate = {
      messageId: cleanText(snapshot.messageId || snapshot.id, 180),
      turnId: cleanText(snapshot.turnId, 180),
      conversationId: cleanText(snapshot.conversationId, 180),
      text: cleanText(snapshot.text, 8000),
      richParts: values,
      updatedAt: Date.now(),
      detached: false
    };
    var routeConversationId = activeConversationId(location.pathname);
    var active = currentBase(location.pathname);
    if (routeConversationId && candidate.conversationId &&
        routeConversationId !== candidate.conversationId) return false;
    if (!routeConversationId && !active) {
      if (!generationBound || !candidate.messageId || !candidate.conversationId || !candidate.text) {
        return false;
      }
      candidate.detached = true;
    }
    if (!routeConversationId && active && !identityMatches(candidate, active)) return false;
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
      if (genericPlaceholderReplacedBy(part, additions)) return false;
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
      if (messages[index] && messages[index].role === 'assistant' &&
          (recovery.detached !== true || completionTextMatches(recovery, messages[index]))) {
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
    prepareSend: function () {
      clearTurnRecovery();
      if (newConversationBoundaryPending) {
        newConversationBoundaryPending = false;
      } else {
        try { if (typeof base.prepareSend === 'function') base.prepareSend(); } catch (_) {}
      }
      notify();
    },
    mergeMessages: function (messages, pathname) {
      observeOfficialCompletion(messages, pathname);
      var merged = messages;
      try { merged = base.mergeMessages(messages, pathname); } catch (_) {}
      return applyMergedCompletion(enrichMessages(merged, pathname), pathname);
    },
    reset: function () {
      clearTurnRecovery();
      if (!newConversationBoundaryPending) {
        newConversationBoundaryPending = true;
        try { if (typeof base.reset === 'function') base.reset(); } catch (_) {}
      }
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
      clearTurnRecovery();
      newConversationBoundaryPending = false;
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
    generation: function () { return conversationGeneration; },
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
