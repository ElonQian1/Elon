(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (!root) return;
  const installed = root.__elonWinChatGptCapturedFinanceRecovery;
  if (installed && Number(installed.version || 0) >= api.version) return;
  root.__elonWinChatGptCapturedFinanceRecovery = Object.freeze({
    version: api.version,
    decodeWidget(widget) { return api.decodeWidget(root, widget); },
    recover(body, format, generation) {
      return api.recover(root, body, format, generation);
    },
    sameResponse: api.sameResponse,
  });
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const VERSION = 3;
  const MAX_WIDGETS = 4;
  const MAX_PACKED_BYTES = 1024 * 1024;
  const MAX_UNPACKED_BYTES = 2 * 1024 * 1024;

  function text(value, limit) {
    return String(value || '').replace(/\s+/g, ' ').trim().slice(0, limit || 180);
  }

  function identity(value) {
    const source = value && typeof value === 'object' ? value : {};
    return {
      messageId: text(source.messageId || source.id, 180),
      turnId: text(source.turnId, 180),
      conversationId: text(source.conversationId, 180),
    };
  }

  function sameResponse(left, right) {
    const a = identity(left);
    const b = identity(right);
    let compared = false;
    if (a.conversationId && b.conversationId) {
      compared = true;
      if (a.conversationId !== b.conversationId) return false;
    }
    if (a.turnId && b.turnId) {
      compared = true;
      if (a.turnId !== b.turnId) return false;
    }
    if (a.messageId && b.messageId) {
      compared = true;
      if (a.messageId !== b.messageId && !(a.turnId && b.turnId && a.turnId === b.turnId)) {
        return false;
      }
    }
    return compared;
  }

  function supportedPart(part) {
    return Boolean(part && part.type === 'rich_card' && (
      part.kind === 'finance' || part.kind === 'chart' ||
      part.richContent && (part.richContent.kind === 'finance' ||
        part.richContent.kind === 'chart')
    ));
  }

  function richPartKey(part) {
    const content = part && part.richContent && typeof part.richContent === 'object'
      ? part.richContent
      : {};
    return [text(part && (part.kind || content.kind), 48), text(part && part.text, 240)].join(':');
  }

  function chartPointCount(chart) {
    if (!chart || typeof chart !== 'object') return 0;
    const points = Array.isArray(chart.points) ? chart.points.length : 0;
    const candles = Array.isArray(chart.candles) ? chart.candles.length : 0;
    const series = Array.isArray(chart.series)
      ? chart.series.reduce(function (total, item) {
          return total + (Array.isArray(item && item.points) ? item.points.length : 0);
        }, 0)
      : 0;
    return points + candles + series;
  }

  function richPartQuality(part) {
    const payload = part && part.richContent && part.richContent.payload;
    if (!payload || typeof payload !== 'object') return 0;
    const periodViews = Array.isArray(payload.periodViews) ? payload.periodViews : [];
    const periodPoints = periodViews.reduce(function (total, view) {
      return total + chartPointCount(view && view.chart);
    }, 0);
    const metrics = Array.isArray(payload.metrics) ? payload.metrics.length : 0;
    return periodViews.length * 10000 + periodPoints * 10 +
      chartPointCount(payload.chart) * 10 + metrics;
  }

  function addOrUpgradePart(parts, part) {
    if (!supportedPart(part)) return false;
    const key = richPartKey(part);
    const existingIndex = parts.findIndex(function (candidate) {
      return richPartKey(candidate) === key;
    });
    if (existingIndex < 0) {
      if (parts.length >= MAX_WIDGETS) return false;
      parts.push(part);
      return true;
    }
    if (richPartQuality(part) <= richPartQuality(parts[existingIndex])) return false;
    parts[existingIndex] = part;
    return true;
  }

  function base64Bytes(rootValue, encoded) {
    const value = String(encoded || '');
    if (!value || value.length > MAX_PACKED_BYTES || !/^[A-Za-z0-9_-]+$/.test(value)) return null;
    const padding = (4 - value.length % 4) % 4;
    const binary = rootValue.atob(value.replace(/-/g, '+').replace(/_/g, '/') + '='.repeat(padding));
    if (binary.length > MAX_PACKED_BYTES) return null;
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
    return bytes;
  }

  async function decodeWidget(rootValue, widget) {
    if (!widget || widget.encoding !== 'gzip-json-base64url-v1' ||
        typeof rootValue.atob !== 'function' || typeof rootValue.Blob !== 'function' ||
        typeof rootValue.DecompressionStream !== 'function') return null;
    let bytes;
    try { bytes = base64Bytes(rootValue, widget.compressed); } catch (_) { return null; }
    if (!bytes) return null;
    try {
      const stream = new rootValue.Blob([bytes]).stream()
        .pipeThrough(new rootValue.DecompressionStream('gzip'));
      const reader = stream.getReader();
      const chunks = [];
      let total = 0;
      while (true) {
        const next = await reader.read();
        if (next.done) break;
        const chunk = next.value || new Uint8Array(0);
        total += chunk.byteLength;
        if (total > MAX_UNPACKED_BYTES) {
          try { await reader.cancel(); } catch (_) {}
          return null;
        }
        chunks.push(chunk);
      }
      if (!total) return null;
      const buffer = new Uint8Array(total);
      let offset = 0;
      chunks.forEach(function (chunk) {
        buffer.set(chunk, offset);
        offset += chunk.byteLength;
      });
      const Decoder = rootValue.TextDecoder ||
        (typeof TextDecoder === 'function' ? TextDecoder : null);
      if (!Decoder) return null;
      const decoded = new Decoder().decode(buffer);
      const value = JSON.parse(decoded);
      return value && typeof value === 'object' && !Array.isArray(value) ? value : null;
    } catch (_) {
      return null;
    }
  }

  function widgetKey(widget) {
    const compressed = String(widget && widget.compressed || '');
    return [
      text(widget && widget.messageId, 180),
      text(widget && widget.widgetId, 180),
      compressed.length,
      compressed.slice(0, 16),
      compressed.slice(-16),
    ].join(':').slice(0, 460);
  }

  function collect(rootValue, body) {
    const policy = rootValue.__elonChatGptPrivateStreamPolicy;
    if (!policy || typeof policy.createSession !== 'function' ||
        typeof policy.createSseDecoder !== 'function' ||
        typeof policy.financePartFromWidget !== 'function') return null;
    const session = policy.createSession({ now: function () { return Date.now(); } });
    const widgets = [];
    const keys = new Set();
    function addWidgets(values) {
      (Array.isArray(values) ? values : []).some(function (widget) {
        const key = widgetKey(widget);
        if (!key || keys.has(key)) return false;
        keys.add(key);
        widgets.push(widget);
        return widgets.length >= MAX_WIDGETS;
      });
    }
    try {
      session.begin();
      const decoder = policy.createSseDecoder(function (payload) {
        session.accept(payload);
        if (typeof policy.packedFinanceWidgets === 'function') {
          addWidgets(policy.packedFinanceWidgets(payload));
        }
      }, function () { session.finish(); });
      decoder.push(String(body || ''));
      decoder.finish();
      if (typeof session.packedWidgets === 'function') addWidgets(session.packedWidgets());
      const pathname = rootValue.location && rootValue.location.pathname || '';
      const snapshot = session.current(pathname) || session.current('');
      return snapshot ? { policy, snapshot, widgets } : null;
    } catch (_) {
      return null;
    }
  }

  function liveRichParts(rootValue, snapshot) {
    const transport = rootValue.__elonChatGptPrivateStreamTransport;
    if (!transport || typeof transport.current !== 'function') return [];
    let active = null;
    try {
      active = transport.current(rootValue.location && rootValue.location.pathname || '');
    } catch (_) {}
    if (!active || !sameResponse(active, snapshot) || !Array.isArray(active.richParts)) return [];
    return active.richParts.filter(supportedPart);
  }

  function improvesLiveRichParts(rootValue, snapshot, candidates) {
    const live = liveRichParts(rootValue, snapshot);
    return (Array.isArray(candidates) ? candidates : []).some(function (candidate) {
      if (!supportedPart(candidate)) return false;
      const key = richPartKey(candidate);
      const current = live.find(function (part) { return richPartKey(part) === key; });
      return !current || richPartQuality(candidate) > richPartQuality(current);
    });
  }

  async function recover(rootValue, body, format, generation) {
    if (format !== 'sse' || typeof body !== 'string' || !body) return false;
    const recovery = rootValue.__elonWinChatGptPrivateStreamRecovery;
    if (!recovery || typeof recovery.accept !== 'function') return false;
    const result = collect(rootValue, body);
    if (!result) return false;
    const parts = [];
    (Array.isArray(result.snapshot.richParts) ? result.snapshot.richParts : [])
      .filter(supportedPart).slice(0, MAX_WIDGETS).forEach(function (part) {
        addOrUpgradePart(parts, part);
      });
    for (let index = 0; index < result.widgets.length; index += 1) {
      const value = await decodeWidget(rootValue, result.widgets[index]);
      const part = value && result.policy.financePartFromWidget(value);
      if (part) addOrUpgradePart(parts, part);
    }
    if (!parts.length || !improvesLiveRichParts(rootValue, result.snapshot, parts)) return false;
    return Boolean(recovery.accept({
      messageId: result.snapshot.id || '',
      turnId: result.snapshot.turnId || '',
      conversationId: result.snapshot.conversationId || '',
      text: result.snapshot.text || '',
      generation,
      richParts: parts,
    }));
  }

  return Object.freeze({ version: VERSION, decodeWidget, recover, sameResponse });
});
