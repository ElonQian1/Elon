(function () {
  'use strict';

  var VERSION = 1;
  var MAX_ENTRIES = 24;
  var MAX_RICH_PARTS = 6;
  var MAX_PACKED_WIDGET_BYTES = 768 * 1024;
  var MAX_UNPACKED_WIDGET_BYTES = 2 * 1024 * 1024;
  var MAX_RICH_DECODE_WAIT_MS = 180;
  var policy = window.__elonChatGptPrivateStreamPolicy;
  var delegateFetch = typeof window.fetch === 'function' ? window.fetch.bind(window) : null;
  if (location.origin !== 'https://chatgpt.com' || !policy || !delegateFetch) return;

  var previous = window.__elonWinChatGptConversationRichCache;
  if (previous && Number(previous.version) >= VERSION) return;
  var entries = new Map();

  function cleanText(value, limit) {
    return String(value || '').replace(/\s+/g, ' ').trim().slice(0, limit || 240);
  }

  function cloneJson(value) {
    try { return JSON.parse(JSON.stringify(value)); }
    catch (_) { return null; }
  }

  function requestTarget(input, init) {
    if (!init || init.__elonPrivateTransport !== 'conversation_prefetch') return null;
    try {
      var url = new URL(typeof input === 'string' ? input : input && input.url, location.href);
      var match = url.origin === location.origin
        ? url.pathname.match(/^\/backend-api\/conversations\/([A-Za-z0-9_-]{1,160})$/)
        : null;
      return match ? { conversationId: match[1] } : null;
    } catch (_) {
      return null;
    }
  }

  function rawMessages(payload) {
    var value = payload && typeof payload === 'object' ? payload : {};
    var mapping = value.mapping && typeof value.mapping === 'object' ? value.mapping : null;
    if (mapping) {
      return Object.values(mapping).map(function (node) {
        return node && (node.message || node);
      }).filter(Boolean);
    }
    var arrays = [
      value.messages,
      value.linear_conversation,
      value.items,
      value.data && value.data.messages,
      value.data && value.data.linear_conversation,
      value.result && value.result.messages,
    ];
    return arrays.find(Array.isArray) || [];
  }

  function richKey(part) {
    var rich = part && part.richContent;
    return cleanText(part && (part.kind || rich && rich.kind), 40).toLowerCase() + ':' +
      cleanText(part && (part.text || rich && rich.payload && rich.payload.title), 180).toLowerCase();
  }

  function uniqueParts(parts) {
    var result = [];
    var keys = new Set();
    (Array.isArray(parts) ? parts : []).some(function (part) {
      if (!part || !part.type) return false;
      var key = part.type === 'rich_card' ? richKey(part) :
        cleanText(part.type, 40) + ':' + cleanText(part.url || part.text, 240);
      if (!key || keys.has(key)) return false;
      var copy = cloneJson(part);
      if (!copy) return false;
      keys.add(key);
      result.push(copy);
      return result.length >= MAX_RICH_PARTS + 32;
    });
    return result;
  }

  function remember(messageId, conversationId, parts) {
    var id = cleanText(messageId, 180);
    if (!id) return;
    var previousEntry = entries.get(id);
    var value = {
      conversationId: cleanText(conversationId || previousEntry && previousEntry.conversationId, 180),
      parts: uniqueParts((previousEntry && previousEntry.parts || []).concat(parts || [])),
      updatedAt: Date.now(),
    };
    entries.delete(id);
    entries.set(id, value);
    while (entries.size > MAX_ENTRIES) entries.delete(entries.keys().next().value);
  }

  function base64UrlBytes(value) {
    var source = String(value || '');
    if (!source || source.length > 1024 * 1024 || !/^[A-Za-z0-9_-]+$/.test(source) ||
        typeof atob !== 'function') return null;
    var padding = source.length % 4 ? '='.repeat(4 - source.length % 4) : '';
    var decoded;
    try { decoded = atob(source.replace(/-/g, '+').replace(/_/g, '/') + padding); }
    catch (_) { return null; }
    if (!decoded || decoded.length > MAX_PACKED_WIDGET_BYTES) return null;
    var bytes = new Uint8Array(decoded.length);
    for (var index = 0; index < decoded.length; index += 1) bytes[index] = decoded.charCodeAt(index);
    return bytes;
  }

  async function decodePackedFinance(widget) {
    if (!widget || widget.encoding !== 'gzip-json-base64url-v1' ||
        typeof DecompressionStream !== 'function' || typeof Blob !== 'function' ||
        typeof TextDecoder !== 'function') return null;
    var compressed = base64UrlBytes(widget.compressed);
    if (!compressed) return null;
    var reader;
    try {
      reader = new Blob([compressed]).stream()
        .pipeThrough(new DecompressionStream('gzip')).getReader();
      var decoder = new TextDecoder();
      var json = '';
      var total = 0;
      while (true) {
        var chunk = await reader.read();
        if (chunk.done) break;
        total += Number(chunk.value && chunk.value.byteLength || 0);
        if (total > MAX_UNPACKED_WIDGET_BYTES) {
          try { await reader.cancel(); } catch (_) {}
          return null;
        }
        json += decoder.decode(chunk.value, { stream: true });
      }
      json += decoder.decode();
      return typeof policy.financePartFromWidget === 'function'
        ? policy.financePartFromWidget(JSON.parse(json))
        : null;
    } catch (_) {
      return null;
    } finally {
      if (reader) {
        try { reader.releaseLock(); } catch (_) {}
      }
    }
  }

  async function observePayload(payload, conversationId) {
    var tasks = [];
    rawMessages(payload).slice(-80).forEach(function (message) {
      if (!message || !message.author || message.author.role !== 'assistant') return;
      var id = cleanText(message.id, 180);
      if (!id) return;
      var envelope = { message: message, conversation_id: conversationId };
      var frame = typeof policy.assistantFrame === 'function' ? policy.assistantFrame(envelope) : null;
      var parts = [];
      if (frame && frame.text) parts.push({ type: 'markdown', text: frame.text });
      if (frame && Array.isArray(frame.citations)) parts = parts.concat(frame.citations);
      var chart = typeof policy.clientChartPartFromMetadata === 'function'
        ? policy.clientChartPartFromMetadata(message.metadata)
        : null;
      if (chart) parts.push(chart);
      if (typeof policy.financePartsFromMetadata === 'function') {
        parts = parts.concat(policy.financePartsFromMetadata(message.metadata));
      }
      remember(id, conversationId, parts);
      var packed = typeof policy.packedFinanceWidgets === 'function'
        ? policy.packedFinanceWidgets(envelope).slice(0, MAX_RICH_PARTS)
        : [];
      packed.forEach(function (widget) {
        tasks.push(Promise.resolve(decodePackedFinance(widget)).then(function (part) {
          if (!part) return;
          remember(id, conversationId, [part]);
          var recovery = window.__elonWinChatGptPrivateStreamRecovery;
          if (recovery && typeof recovery.accept === 'function') {
            try { recovery.accept({ messageId: id, conversationId: conversationId, richParts: [part] }); }
            catch (_) {}
          }
        }));
      });
    });
    if (!tasks.length) return;
    await Promise.race([
      Promise.all(tasks),
      new Promise(function (resolve) { window.setTimeout(resolve, MAX_RICH_DECODE_WAIT_MS); }),
    ]);
  }

  function enrichMessage(message, requestedPath) {
    if (!message || message.role !== 'assistant') return message;
    var entry = entries.get(cleanText(message.id, 180));
    if (!entry) return message;
    var pathMatch = String(requestedPath || '').match(/(?:^|\/)c\/([A-Za-z0-9_-]{1,160})$/);
    if (pathMatch && entry.conversationId && pathMatch[1] !== entry.conversationId) return message;
    var content = uniqueParts(entry.parts);
    if (!content.length) return message;
    return Object.assign({}, message, { content: content });
  }

  window.fetch = function () {
    var args = Array.from(arguments);
    var target = requestTarget(args[0], args[1] || {});
    var result = delegateFetch.apply(window, args);
    if (!target) return result;
    return Promise.resolve(result).then(function (response) {
      if (!response || !response.ok || typeof response.json !== 'function') return response;
      var originalJson = response.json.bind(response);
      var payloadPromise = null;
      var replacement = function () {
        if (!payloadPromise) {
          payloadPromise = Promise.resolve(originalJson()).then(async function (payload) {
            await observePayload(payload, target.conversationId);
            return payload;
          });
        }
        return payloadPromise;
      };
      try { Object.defineProperty(response, 'json', { configurable: true, value: replacement }); }
      catch (_) {
        try { response.json = replacement; } catch (_) {}
      }
      return response;
    });
  };

  window.__elonWinChatGptConversationRichCache = Object.freeze({
    version: VERSION,
    enrichMessage: enrichMessage,
    size: function () { return entries.size; },
  });
})();
