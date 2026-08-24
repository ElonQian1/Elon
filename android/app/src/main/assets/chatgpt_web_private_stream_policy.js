(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptPrivateStreamPolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const MAX_TEXT_LENGTH = 40000;
  const MAX_BUFFER_LENGTH = 2 * 1024 * 1024;
  const MAX_PATCH_TEXT_LENGTH = 524288;
  const MAX_AGE_MS = 5 * 60 * 1000;
  const MAX_PROGRESS_LENGTH = 220;
  const MAX_PATCH_ARRAY_LENGTH = 128;
  const MAX_FINANCE_WIDGETS = 4;
  const MAX_PACKED_WIDGET_LENGTH = 1024 * 1024;
  const MAX_FINANCE_POINTS = 256;
  const MAX_CHART_SERIES = 4;
  const MAX_CHART_POINTS = 256;

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\n{3,}/g, '\n\n').trim();
  }

  function compactEnvelope(payload) {
    if (!payload || typeof payload !== 'object') return payload;
    return Number.isFinite(payload.c) && payload.v && typeof payload.v === 'object'
      ? payload.v
      : payload;
  }

  function assistantEnvelope(payload) {
    const envelope = compactEnvelope(payload);
    if (!envelope || typeof envelope !== 'object') return null;
    const message = envelope.message || envelope.data && envelope.data.message;
    if (!message || typeof message !== 'object' ||
        !message.author || message.author.role !== 'assistant') return null;
    return { envelope, message };
  }

  function safeProgressLabel(value) {
    const label = cleanText(value).replace(/[\u0000-\u001f\u007f]/g, ' ').slice(0, MAX_PROGRESS_LENGTH);
    if (!label) return '';
    return /^(?:正在(?:搜索|查询|浏览)|searching|browsing|looking up)\b/i.test(label) ||
      /^(?:正在(?:搜索|查询|浏览))/.test(label)
      ? label
      : '';
  }

  function progressFrame(payload) {
    const envelope = compactEnvelope(payload);
    if (!envelope || typeof envelope !== 'object') return null;
    const message = envelope.message || envelope.data && envelope.data.message;
    const metadata = message && message.metadata;
    const label = safeProgressLabel(metadata && metadata.reasoning_title);
    if (!label) return null;
    return {
      conversationId: String(envelope.conversation_id || envelope.conversationId || '').slice(0, 180),
      progressLabel: label,
      state: 'streaming'
    };
  }

  function publicSourceUrl(value) {
    try {
      const url = new URL(String(value || ''));
      if (!/^https?:$/.test(url.protocol) || url.username || url.password) return '';
      url.search = '';
      url.hash = '';
      return url.toString();
    } catch (_) {
      return '';
    }
  }

  function sourceHost(value) {
    try { return new URL(value).hostname.toLowerCase().replace(/^www\./, ''); }
    catch (_) { return ''; }
  }

  function sourceLabel(item) {
    return cleanText(item && (item.attribution || item.title)).replace(/[\[\]()]/g, '').slice(0, 80);
  }

  function safeToken(value, limit) {
    return cleanText(value).replace(/[^A-Za-z0-9._-]/g, '').slice(0, limit || 24);
  }

  function boundedSamples(values, limit) {
    const source = Array.isArray(values) ? values : [];
    if (source.length <= limit) return source.slice();
    return Array.from({ length: limit }, (_, index) =>
      source[Math.round(index * (source.length - 1) / (limit - 1))]
    );
  }

  function financeTrend(summary) {
    const color = cleanText(summary && summary.price_change_color).toLowerCase();
    const change = cleanText(summary && summary.price_change_text);
    if (/^(?:success|positive|green)$/.test(color) || /^\+/.test(change)) return 'positive';
    if (/^(?:danger|negative|error|red)$/.test(color) || /^(?:-|−)/.test(change)) return 'negative';
    return 'neutral';
  }

  function financePartFromWidget(value) {
    const widget = value && typeof value === 'object' && !Array.isArray(value) ? value : {};
    const order = Array.isArray(widget.timeframe_order) ? widget.timeframe_order.slice(0, 12) : [];
    const configs = widget.timeframe_configs && typeof widget.timeframe_configs === 'object'
      ? widget.timeframe_configs
      : {};
    const selectedLabel = cleanText(widget.default_range).slice(0, 16);
    const selected = configs[selectedLabel] && typeof configs[selectedLabel] === 'object'
      ? configs[selectedLabel]
      : configs[order.find((label) => configs[label])] || {};
    const summary = selected.summary && typeof selected.summary === 'object'
      ? selected.summary
      : widget.fallback_summary && typeof widget.fallback_summary === 'object'
        ? widget.fallback_summary
        : {};
    const chart = selected.chart && typeof selected.chart === 'object' ? selected.chart : {};
    const points = boundedSamples(chart.data, MAX_FINANCE_POINTS).map((point, index) => ({
      x: cleanText(point && (point.formatted || point.timestamp || index)).slice(0, 64),
      y: Number(point && point.close)
    })).filter((point) => point.x && Number.isFinite(point.y));
    const metrics = (Array.isArray(widget.metrics_display) ? widget.metrics_display : [])
      .slice(0, 8)
      .flatMap((row) => Array.isArray(row && row.cols) ? row.cols.slice(0, 8) : [])
      .map((metric) => ({
        label: cleanText(metric && metric.label).slice(0, 64),
        value: cleanText(metric && metric.value).slice(0, 96)
      }))
      .filter((metric) => metric.label && metric.value)
      .slice(0, 16);
    const title = cleanText(widget.asset_display_name).slice(0, 120);
    const primaryValue = cleanText(
      widget.current_price_text || summary.price_text ||
      widget.fallback_summary && widget.fallback_summary.price_text
    ).slice(0, 64);
    if (!title || !primaryValue || points.length < 2) return null;
    const symbolMatch = title.match(/\(([A-Za-z0-9._-]{1,24})\)\s*$/);
    const payload = {
      title,
      symbol: symbolMatch ? safeToken(symbolMatch[1], 24) : '',
      primaryValue,
      secondaryValue: cleanText(summary.price_change_text).slice(0, 96),
      trend: financeTrend(summary),
      periods: order.map((label) => ({
        id: safeToken(label, 16).toLowerCase(),
        label: cleanText(label).slice(0, 16),
        selected: label === selectedLabel
      })).filter((period) => period.id && period.label && configs[period.label]),
      metrics,
      chart: { kind: 'line', points }
    };
    return {
      type: 'rich_card',
      text: title,
      kind: 'finance',
      richContent: {
        schema: 'yilong.rich-content.v1',
        kind: 'finance',
        source: 'private_response',
        payload
      }
    };
  }

  function clientChartPartFromMetadata(metadata) {
    const references = metadata && Array.isArray(metadata.content_references)
      ? metadata.content_references
      : [];
    const reference = references.find((item) => item &&
      item.type === 'client_defined_widget' &&
      item.category === 'visualization' &&
      item.data && item.data.widget_type === 'charts_widget_v2' &&
      item.data.language === 'recharts-json' &&
      item.data.content && typeof item.data.content === 'object');
    if (!reference) return null;
    const content = reference.data.content;
    if (cleanText(content.chartType).toLowerCase() !== 'line') return null;
    const title = cleanText(content.meta && content.meta.title).slice(0, 120);
    const xKey = safeToken(content.xKey, 48);
    const series = (Array.isArray(content.series) ? content.series : [])
      .slice(0, MAX_CHART_SERIES)
      .map((item) => ({
        key: safeToken(item && item.dataKey, 48),
        label: cleanText(item && item.label).slice(0, 64),
        valuePrefix: cleanText(item && item.valuePrefix).slice(0, 16),
        valueSuffix: cleanText(item && item.valueSuffix).slice(0, 16)
      }))
      .filter((item) => item.key && item.label);
    if (!title || !xKey || !series.length) return null;
    const points = boundedSamples(content.data, MAX_CHART_POINTS).map((item) => ({
      x: cleanText(item && item[xKey]).slice(0, 64),
      values: series.map((entry) => typeof (item && item[entry.key]) === 'number'
        ? item[entry.key]
        : Number.NaN)
    })).filter((point) => point.x && point.values.every(Number.isFinite));
    if (points.length < 2) return null;
    const payload = {
      title,
      description: cleanText(content.meta && content.meta.description).slice(0, 240),
      chartType: 'line',
      series,
      points
    };
    return {
      type: 'rich_card',
      text: title,
      kind: 'chart',
      richContent: {
        schema: 'yilong.rich-content.v1',
        kind: 'chart',
        source: 'private_response',
        payload
      }
    };
  }

  function visibleContentText(value) {
    const text = String(value || '');
    const marker = '\ue200genui\ue202';
    let result = '';
    let offset = 0;
    while (offset < text.length) {
      const start = text.indexOf(marker, offset);
      if (start < 0) return result + text.slice(offset);
      result += text.slice(offset, start);
      const end = text.indexOf('\ue201', start + marker.length);
      if (end < 0) return result;
      offset = end + 1;
    }
    return result;
  }

  function packedFinanceWidgets(payload) {
    const visible = assistantEnvelope(payload);
    const metadata = visible && visible.message && visible.message.metadata;
    const widgets = metadata && metadata.view_state && metadata.view_state.widgets;
    if (!widgets || typeof widgets !== 'object' || Array.isArray(widgets)) return [];
    return Object.entries(widgets).slice(0, MAX_FINANCE_WIDGETS).map(([widgetId, packed]) => {
      const compressed = packed && typeof packed.__compressed === 'string'
        ? packed.__compressed
        : '';
      const encoding = cleanText(packed && packed.__encoding);
      if (encoding !== 'gzip-json-base64url-v1' || !compressed ||
          compressed.length > MAX_PACKED_WIDGET_LENGTH || !/^[A-Za-z0-9_-]+$/.test(compressed)) {
        return null;
      }
      return {
        widgetId: cleanText(widgetId).slice(0, 180),
        messageId: String(visible.message.id || '').slice(0, 180),
        turnId: String(
          metadata.turn_exchange_id || metadata.working_turn_id || ''
        ).slice(0, 180),
        conversationId: String(
          visible.envelope.conversation_id || visible.envelope.conversationId || ''
        ).slice(0, 180),
        encoding,
        compressed
      };
    }).filter(Boolean);
  }

  function citationRecords(metadata) {
    const references = metadata && Array.isArray(metadata.content_references)
      ? metadata.content_references
      : [];
    return references.slice(0, 32).flatMap((reference, referenceIndex) => {
      if (!reference || reference.type !== 'grouped_webpages' || !Array.isArray(reference.items)) return [];
      const primary = reference.items[0];
      const url = publicSourceUrl(primary && primary.url);
      const label = sourceLabel(primary);
      if (!url || !label) return [];
      const refs = primary && Array.isArray(primary.refs) ? primary.refs.length : 0;
      const supporting = primary && Array.isArray(primary.supporting_websites)
        ? primary.supporting_websites.length
        : 0;
      const groupSize = Math.max(1, refs, supporting + 1);
      return [{
        start: Number(reference.start_idx),
        end: Number(reference.end_idx),
        matchedText: String(reference.matched_text || ''),
        part: {
          type: 'citation',
          text: label,
          url,
          markerText: label + (groupSize > 1 ? ' +' + (groupSize - 1) : ''),
          citationId: 'private-ref-' + referenceIndex,
          groupSize,
          targetHost: sourceHost(url)
        }
      }];
    });
  }

  function linkedText(value, citations) {
    let text = String(value || '');
    const replacements = citations.map((citation) => {
      let start = citation.start;
      let end = citation.end;
      const marker = citation.matchedText;
      const exact = Number.isInteger(start) && Number.isInteger(end) && start >= 0 && end >= start &&
        end <= text.length && text.slice(start, end) === marker;
      if (!exact) {
        start = marker && text.indexOf(marker) === text.lastIndexOf(marker) ? text.indexOf(marker) : -1;
        end = start >= 0 ? start + marker.length : -1;
      }
      return { citation, start, end };
    }).filter((item) => item.start >= 0 && item.end > item.start)
      .sort((left, right) => right.start - left.start);
    replacements.forEach(({ citation, start, end }) => {
      const label = citation.part.markerText.replace(/[\[\]()]/g, '');
      text = text.slice(0, start) + '[' + label + '](' + citation.part.url + ')' + text.slice(end);
    });
    return cleanText(text);
  }

  function contentText(content) {
    if (!content || typeof content !== 'object') return '';
    if (Array.isArray(content.parts)) {
      return cleanText(content.parts.map((part) => {
        if (typeof part === 'string') return part;
        if (!part || typeof part !== 'object') return '';
        return typeof part.text === 'string' ? part.text :
          (typeof part.content === 'string' ? part.content : '');
      }).filter(Boolean).map(visibleContentText).join('\n'));
    }
    if (typeof content.text === 'string') return cleanText(visibleContentText(content.text));
    if (typeof content.content === 'string') return cleanText(visibleContentText(content.content));
    return '';
  }

  function assistantFrame(payload) {
    const visible = assistantEnvelope(payload);
    if (!visible) return null;
    const { envelope, message } = visible;
    const contentType = String(message.content && message.content.content_type || '').toLowerCase();
    if (contentType && contentType !== 'text') return null;
    const citations = citationRecords(message.metadata);
    const text = linkedText(contentText(message.content), citations).slice(0, MAX_TEXT_LENGTH);
    if (!text) return null;
    const rawStatus = String(message.status || envelope.status || '').toLowerCase();
    const completed = /^(completed|finished_successfully|finished)$/.test(rawStatus);
    return {
      id: String(message.id || '').slice(0, 180),
      conversationId: String(envelope.conversation_id || envelope.conversationId || '').slice(0, 180),
      text,
      citations: citations.map((citation) => citation.part),
      state: completed ? 'completed' : 'streaming'
    };
  }

  function createSseDecoder(onPayload, onDone) {
    let buffer = '';
    let closed = false;

    function processEvent(rawEvent) {
      const data = String(rawEvent || '').split(/\r?\n/)
        .filter((line) => line.startsWith('data:'))
        .map((line) => line.slice(5).trimStart())
        .join('\n')
        .trim();
      if (!data) return;
      if (data === '[DONE]') {
        closed = true;
        if (typeof onDone === 'function') onDone();
        return;
      }
      try {
        const payload = JSON.parse(data);
        if (typeof onPayload === 'function') onPayload(payload);
      } catch (_) {
        // Unknown frames remain owned by the official page and are ignored.
      }
    }

    function push(chunk) {
      if (closed || !chunk) return;
      buffer += String(chunk);
      if (buffer.length > MAX_BUFFER_LENGTH) {
        buffer = buffer.slice(-MAX_BUFFER_LENGTH);
      }
      let boundary = buffer.search(/\r?\n\r?\n/);
      while (boundary >= 0) {
        const separator = buffer.slice(boundary).match(/^\r?\n\r?\n/)[0];
        const event = buffer.slice(0, boundary);
        buffer = buffer.slice(boundary + separator.length);
        processEvent(event);
        if (closed) return;
        boundary = buffer.search(/\r?\n\r?\n/);
      }
    }

    function finish() {
      if (closed) return;
      if (buffer.trim()) processEvent(buffer);
      buffer = '';
      if (!closed && typeof onDone === 'function') onDone();
      closed = true;
    }

    return Object.freeze({ push, finish });
  }

  function conversationMatches(pathname, conversationId) {
    const path = String(pathname || '');
    if (!conversationId || path === '/' || path === '') return true;
    const match = path.match(/^\/c\/([^/?#]+)/);
    return !match || match[1] === conversationId;
  }

  function primaryText(message) {
    const content = message && Array.isArray(message.content) ? message.content : [];
    const part = content.find((item) => item && (item.type === 'markdown' || item.type === 'text'));
    return cleanText(part && part.text);
  }

  function mergedMessage(message, stream) {
    const replaceCitations = Array.isArray(stream.citations) && stream.citations.length > 0;
    const replaceFinance = Array.isArray(stream.richParts) && stream.richParts.length > 0;
    const content = Array.isArray(message.content)
      ? message.content.filter((part) => part &&
        (!replaceCitations || part.type !== 'citation') &&
        (!replaceFinance || !(
          (part.type === 'rich_card' && (part.kind === 'finance' ||
            part.richContent && part.richContent.kind === 'finance')) ||
          ['artifact', 'chart', 'interactive'].includes(part.type)
        ))).slice()
      : [];
    const index = content.findIndex((item) => item && (item.type === 'markdown' || item.type === 'text'));
    const part = { type: 'markdown', text: stream.text || '' };
    if (index >= 0) content[index] = Object.assign({}, content[index], part);
    else content.unshift(part);
    (stream.citations || []).forEach((citation) => content.push(Object.assign({}, citation)));
    (stream.richParts || []).forEach((richPart) => content.push(Object.assign({}, richPart)));
    return Object.assign({}, message, { state: stream.state, content });
  }

  function mergeMessages(messages, stream) {
    const values = Array.isArray(messages) ? messages : [];
    if (!stream || (!stream.text && stream.state !== 'streaming')) return values;
    let assistantIndex = -1;
    for (let index = values.length - 1; index >= 0; index -= 1) {
      if (values[index] && values[index].role === 'assistant') {
        assistantIndex = index;
        break;
      }
    }
    const assistant = assistantIndex >= 0 ? values[assistantIndex] : null;
    const text = primaryText(assistant);
    let latestUserIndex = -1;
    for (let index = values.length - 1; index >= 0; index -= 1) {
      if (values[index] && values[index].role === 'user') {
        latestUserIndex = index;
        break;
      }
    }
    const sameTurn = !!assistant && latestUserIndex >= 0 && assistantIndex > latestUserIndex;
    const sameMessage = !!assistant && (
      (stream.id && (assistant.id === stream.id || assistant.id === 'private-stream:' + stream.id)) ||
      (text && stream.text && (stream.text.startsWith(text) || text.startsWith(stream.text))) ||
      sameTurn
    );
    if (sameMessage) {
      if (text.length > stream.text.length) {
        stream = Object.assign({}, stream, { text });
      }
      const result = values.slice();
      result[assistantIndex] = mergedMessage(assistant, stream);
      return result;
    }
    return values.concat([{
      id: stream.id ? 'private-stream:' + stream.id : 'private-stream:assistant',
      role: 'assistant',
      state: stream.state,
      content: [{ type: 'markdown', text: stream.text }].concat(
        (stream.citations || []).map((citation) => Object.assign({}, citation)),
        (stream.richParts || []).map((richPart) => Object.assign({}, richPart))
      )
    }]);
  }

  function createSession(options) {
    const now = typeof options.now === 'function' ? options.now : Date.now;
    let stream = null;
    let compactDocument = null;
    let compactContinuation = null;

    function clonePatchValue(value) {
      if (value === undefined) return undefined;
      try { return JSON.parse(JSON.stringify(value)); }
      catch (_) { return null; }
    }

    function pointerSegments(path) {
      const value = String(path || '');
      if (!value || value.length > 320 || value[0] !== '/') return null;
      const segments = value.slice(1).split('/').map((segment) =>
        segment.replace(/~1/g, '/').replace(/~0/g, '~')
      );
      if (!segments.length || segments.length > 16 || segments[0] !== 'message' ||
          segments.some((segment) => !segment ||
            segment === '__proto__' || segment === 'prototype' || segment === 'constructor')) return null;
      return segments;
    }

    function patchContainer(root, segments) {
      let owner = root;
      for (let index = 0; index < segments.length - 1; index += 1) {
        const segment = segments[index];
        if (!owner || typeof owner !== 'object' || !Object.prototype.hasOwnProperty.call(owner, segment)) {
          return null;
        }
        owner = owner[segment];
      }
      return owner && typeof owner === 'object'
        ? { owner, key: segments[segments.length - 1] }
        : null;
    }

    function applyPatchOperation(root, operation) {
      if (!root || typeof root !== 'object' || !operation || typeof operation !== 'object') return false;
      const kind = String(operation.o || '').toLowerCase();
      if (!/^(?:add|append|replace|remove)$/.test(kind)) return false;
      const segments = pointerSegments(operation.p);
      if (!segments) return false;
      const target = patchContainer(root, segments);
      if (!target) return false;
      const { owner, key } = target;
      if (kind === 'remove') {
        if (Array.isArray(owner) && /^\d+$/.test(key)) owner.splice(Number(key), 1);
        else delete owner[key];
        return true;
      }
      const value = clonePatchValue(operation.v);
      if (kind === 'append') {
        const existing = owner[key];
        if (typeof existing === 'string' && typeof value === 'string') {
          owner[key] = (existing + value).slice(0, MAX_PATCH_TEXT_LENGTH);
          return true;
        }
        if (Array.isArray(existing)) {
          const additions = Array.isArray(value) ? value : [value];
          owner[key] = existing.concat(additions).slice(0, MAX_PATCH_ARRAY_LENGTH);
          return true;
        }
        if (existing && typeof existing === 'object' && value && typeof value === 'object' &&
            !Array.isArray(value)) {
          Object.assign(existing, value);
          return true;
        }
      }
      if (Array.isArray(owner) && key === '-') owner.push(value);
      else owner[key] = value;
      return true;
    }

    function applyCompactPayload(payload) {
      if (!payload || typeof payload !== 'object' || Array.isArray(payload)) return false;
      if (Number.isFinite(payload.c) && payload.v && typeof payload.v === 'object') {
        compactDocument = clonePatchValue(payload.v);
        compactContinuation = null;
        return compactDocument ? acceptVisiblePayload(compactDocument) : false;
      }
      if (!compactDocument) return false;
      let changed = false;
      const patchBatch = Array.isArray(payload.v) && payload.v.length > 0 &&
        payload.v.every((operation) => operation && typeof operation === 'object' &&
          operation.o && operation.p);
      if ((payload.o === 'patch' || patchBatch) && Array.isArray(payload.v)) {
        payload.v.slice(0, MAX_PATCH_ARRAY_LENGTH).forEach((operation) => {
          if (applyPatchOperation(compactDocument, operation)) changed = true;
        });
        const last = payload.v[payload.v.length - 1];
        compactContinuation = last && last.o === 'append'
          ? { o: last.o, p: last.p }
          : null;
      } else if (payload.o && payload.p) {
        changed = applyPatchOperation(compactDocument, payload);
        compactContinuation = payload.o === 'append'
          ? { o: payload.o, p: payload.p }
          : null;
      } else if (compactContinuation && Object.keys(payload).length === 1 &&
          Object.prototype.hasOwnProperty.call(payload, 'v')) {
        changed = applyPatchOperation(compactDocument, Object.assign({}, compactContinuation, {
          v: payload.v
        }));
      }
      return changed && acceptVisiblePayload(compactDocument);
    }

    function acceptVisiblePayload(payload) {
      let accepted = false;
      const progress = progressFrame(payload);
      if (progress) {
        if (!stream) begin();
        stream = Object.assign({}, stream, progress, { updatedAt: now() });
        accepted = true;
      }
      const visible = assistantEnvelope(payload);
      if (!visible) return accepted;
      const contentType = String(visible.message.content && visible.message.content.content_type || '').toLowerCase();
      if (contentType && contentType !== 'text') return accepted;
      if (!stream) begin();
      const frame = assistantFrame(payload);
      const rawStatus = String(visible.message.status || visible.envelope.status || '').toLowerCase();
      stream = Object.assign({}, stream, {
        id: String(visible.message.id || stream.id || '').slice(0, 180),
        turnId: String(
          visible.message.metadata && (
            visible.message.metadata.turn_exchange_id ||
            visible.message.metadata.working_turn_id
          ) || stream.turnId || ''
        ).slice(0, 180),
        conversationId: String(
          visible.envelope.conversation_id || visible.envelope.conversationId || stream.conversationId || ''
        ).slice(0, 180),
        state: /^(completed|finished_successfully|finished)$/.test(rawStatus)
          ? 'completed'
          : 'streaming',
        updatedAt: now()
      }, frame || {});
      const chartPart = clientChartPartFromMetadata(visible.message.metadata);
      if (chartPart) {
        const richParts = Array.isArray(stream.richParts) ? stream.richParts.slice() : [];
        const existing = richParts.findIndex((part) => part && part.kind === 'chart');
        if (existing >= 0) richParts[existing] = chartPart;
        else if (richParts.length < MAX_FINANCE_WIDGETS) richParts.push(chartPart);
        stream.richParts = richParts;
      }
      return true;
    }

    function begin() {
      stream = {
        id: '', turnId: '', conversationId: '', text: '', progressLabel: '',
        state: 'streaming', richParts: [], updatedAt: now()
      };
      compactDocument = null;
      compactContinuation = null;
    }

    function accept(payload) {
      if (applyCompactPayload(payload)) return true;
      return acceptVisiblePayload(payload);
    }

    function finish() {
      if (!stream || !stream.text) return false;
      stream = Object.assign({}, stream, { state: 'completed', updatedAt: now() });
      return true;
    }

    function acceptRichParts(parts, identity) {
      const values = Array.isArray(parts) ? parts.filter(Boolean).slice(0, MAX_FINANCE_WIDGETS) : [];
      if (!values.length) return false;
      const context = identity && typeof identity === 'object' ? identity : {};
      if (stream && stream.conversationId && context.conversationId &&
          stream.conversationId !== context.conversationId) return false;
      if (stream && stream.turnId && context.turnId && stream.turnId !== context.turnId) return false;
      const messageMismatch = stream && stream.id && context.messageId && stream.id !== context.messageId;
      const sameTurn = stream && stream.turnId && context.turnId && stream.turnId === context.turnId;
      const sameConversationWithoutTurn = stream && !stream.turnId && !context.turnId &&
        stream.conversationId && context.conversationId &&
        stream.conversationId === context.conversationId;
      if (messageMismatch && !sameTurn && !sameConversationWithoutTurn) return false;
      if (!stream) begin();
      const existing = Array.isArray(stream.richParts) ? stream.richParts.slice() : [];
      values.forEach((part) => {
        const kind = cleanText(part && part.kind);
        const title = cleanText(part && part.text);
        const index = existing.findIndex((candidate) =>
          cleanText(candidate && candidate.kind) === kind && cleanText(candidate && candidate.text) === title
        );
        if (index >= 0) existing[index] = part;
        else if (existing.length < MAX_FINANCE_WIDGETS) existing.push(part);
      });
      stream = Object.assign({}, stream, {
        id: String(stream.id || context.messageId || '').slice(0, 180),
        turnId: String(stream.turnId || context.turnId || '').slice(0, 180),
        conversationId: String(stream.conversationId || context.conversationId || '').slice(0, 180),
        richParts: existing.map((part) => JSON.parse(JSON.stringify(part))),
        updatedAt: now()
      });
      return true;
    }

    function reset() {
      stream = null;
      compactDocument = null;
      compactContinuation = null;
    }

    function current(pathname) {
      if (!stream || (!stream.text && !stream.progressLabel && !stream.id && !stream.richParts.length) ||
          now() - stream.updatedAt > MAX_AGE_MS ||
          !conversationMatches(pathname, stream.conversationId)) return null;
      return Object.assign({}, stream);
    }

    function merge(values, pathname) {
      const active = current(pathname);
      if (!active) return values;
      return mergeMessages(values, active);
    }

    function packedWidgets() {
      return packedFinanceWidgets(compactDocument);
    }

    return Object.freeze({
      begin,
      accept,
      acceptRichParts,
      finish,
      reset,
      current,
      merge,
      packedWidgets
    });
  }

  return Object.freeze({
    assistantFrame,
    clientChartPartFromMetadata,
    createSession,
    createSseDecoder,
    financePartFromWidget,
    mergeMessages,
    packedFinanceWidgets,
    progressFrame
  });
});
