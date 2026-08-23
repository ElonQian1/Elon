(function () {
  'use strict';

  const API_VERSION = 4;
  if (location.origin !== 'https://chatgpt.com' ||
      Number(window.__elonChatGptRichContent && window.__elonChatGptRichContent.version || 0) >= API_VERSION) return;

  const SCHEMA = 'yilong.rich-content.v1';
  const ROOT_ATTRIBUTE = 'data-elon-rich-content-root';
  const MAX_VISIBLE_CHART_POINTS = 96;
  const commonRichContent = window.__elonRichContentDomAdapter;
  const PERIOD = /^(?:1D|5D|1M|3M|6M|YTD|1Y|5Y|MAX)$/i;
  const MONEY = /^(?:(?:US|CA|AU|HK|NT)\s*)?[$€£¥]\s*[0-9][0-9,.]*(?:\.[0-9]+)?$/i;
  const METRIC_LABELS = [
    '打开', '交易量', '市值', '当日最低价', '当日最高价', '年度最低价', '年度最高价',
    '每股收益', '市盈率', 'Open', 'Volume', 'Market cap', 'Day low', 'Day high',
    '52-week low', '52-week high', 'EPS', 'P/E ratio'
  ];

  function cleanText(value, max) {
    return String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/[\u200b-\u200f\u2060\ufeff]/g, '')
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, max || 160);
  }

  function cleanLines(value) {
    return String(value || '').split(/\r?\n/).map((line) => cleanText(line, 160)).filter(Boolean);
  }

  function safeToken(value, max) {
    return cleanText(value, max || 24).replace(/[^A-Za-z0-9_-]/g, '').toLowerCase();
  }

  function trendFor(primary, secondary) {
    const value = cleanText(secondary || primary, 96);
    if (/^(?:\+|↑)/.test(value)) return 'positive';
    if (/^(?:-|−|↓)/.test(value)) return 'negative';
    return 'neutral';
  }

  function normalizeFinancePayload(value) {
    const source = value && typeof value === 'object' ? value : {};
    const periods = Array.isArray(source.periods) ? source.periods.slice(0, 12).map((period) => ({
      id: safeToken(period && (period.id || period.label), 16),
      label: cleanText(period && period.label, 16),
      selected: Boolean(period && period.selected)
    })).filter((period) => period.id && period.label) : [];
    const metrics = Array.isArray(source.metrics) ? source.metrics.slice(0, 16).map((metric) => ({
      label: cleanText(metric && metric.label, 64),
      value: cleanText(metric && metric.value, 96)
    })).filter((metric) => metric.label && metric.value) : [];
    const payload = {
      title: cleanText(source.title, 120),
      symbol: cleanText(source.symbol, 24).replace(/[^A-Za-z0-9._-]/g, ''),
      primaryValue: cleanText(source.primaryValue, 64),
      secondaryValue: cleanText(source.secondaryValue, 96),
      trend: ['positive', 'negative', 'neutral'].includes(source.trend)
        ? source.trend
        : trendFor(source.primaryValue, source.secondaryValue),
      periods,
      metrics
    };
    const chart = source.chart && typeof source.chart === 'object' ? source.chart : {};
    const candles = chart.kind === 'candlestick' && Array.isArray(chart.candles)
      ? chart.candles.slice(0, 512).map((candle) => ({
          x: cleanText(candle && candle.x, 64),
          open: Number(candle && candle.open),
          high: Number(candle && candle.high),
          low: Number(candle && candle.low),
          close: Number(candle && candle.close)
        })).filter((candle) => candle.x &&
          [candle.open, candle.high, candle.low, candle.close].every(Number.isFinite) &&
          candle.high >= Math.max(candle.open, candle.close) &&
          candle.low <= Math.min(candle.open, candle.close))
      : [];
    const points = chart.kind !== 'candlestick' && Array.isArray(chart.points)
      ? chart.points.slice(0, 512).map((point) => ({
          x: cleanText(point && point.x, 64),
          y: Number(point && point.y)
        })).filter((point) => point.x && Number.isFinite(point.y))
      : [];
    if (candles.length > 0) payload.chart = { kind: 'candlestick', candles };
    else if (points.length > 1) payload.chart = { kind: 'line', points };
    return payload;
  }

  function visible(node) {
    if (!(node instanceof HTMLElement) || !node.isConnected) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width > 120 && rect.height > 24 &&
      style.display !== 'none' && style.visibility !== 'hidden';
  }

  function periodControls(root) {
    return Array.from(root.querySelectorAll(
      '[role="radiogroup"] [role="radio"], [role="radiogroup"] button, '
      + '[role="tablist"] [role="tab"], [role="tablist"] button'
    )).map((node) => ({
      node,
      label: cleanText(node.getAttribute('aria-label') || node.textContent, 16)
    })).filter((entry) => PERIOD.test(entry.label));
  }

  function metricPairs(lines) {
    const result = [];
    lines.forEach((line, index) => {
      const label = METRIC_LABELS.find((candidate) =>
        line.toLocaleLowerCase() === candidate.toLocaleLowerCase()
        || line.toLocaleLowerCase().startsWith(candidate.toLocaleLowerCase() + ' ')
      );
      if (!label) return;
      const inline = cleanText(line.slice(label.length), 96);
      const value = inline || cleanText(lines[index + 1], 96);
      if (!value || METRIC_LABELS.some((candidate) => candidate.toLocaleLowerCase() === value.toLocaleLowerCase())) return;
      if (!result.some((metric) => metric.label === label)) result.push({ label, value });
    });
    return result.slice(0, 16);
  }

  function firstPrice(lines) {
    return lines.find((line) => MONEY.test(line.replace(/\s+/g, ''))) || '';
  }

  function pairFor(pairs, pattern) {
    return pairs.find((pair) => pattern.test(cleanText(pair && pair.label, 64)));
  }

  function numericValue(value) {
    const match = cleanText(value, 96).match(/[-+]?\d[\d,]*(?:\.\d+)?/);
    if (!match) return undefined;
    const parsed = Number(match[0].replace(/,/g, ''));
    return Number.isFinite(parsed) ? parsed : undefined;
  }

  function financePayloadFromPairs(pairs, title) {
    const values = Array.isArray(pairs) ? pairs.slice(0, 32).map((pair) => ({
      label: cleanText(pair && pair.label, 64),
      value: cleanText(pair && pair.value, 96)
    })).filter((pair) => pair.label && pair.value) : [];
    const symbolPair = pairFor(values, /^(?:股票代码|证券代码|代码|symbol|ticker)(?:\s|$|[（(])/i);
    const primaryPair = pairFor(values, /^(?:最新价(?:格)?|现价|last(?: price)?|latest price|price)(?:\s|$|[（(])/i)
      || pairFor(values, /^(?:收盘(?:价)?|close)(?:\s|$|[（(])/i);
    const changePair = pairFor(values, /^(?:涨跌(?:额|幅)?|change)(?:\s|$|[（(])/i);
    const openPair = pairFor(values, /^(?:开盘(?:价)?|open)(?:\s|$|[（(])/i);
    const highPair = pairFor(values, /^(?:最高(?:价)?|high)(?:\s|$|[（(])/i);
    const lowPair = pairFor(values, /^(?:最低(?:价)?|low)(?:\s|$|[（(])/i);
    const closePair = pairFor(values, /^(?:收盘(?:价)?|close)(?:\s|$|[（(])/i) || primaryPair;
    const datePair = pairFor(values, /^(?:日期|交易日|date|trading date)(?:\s|$|[（(])/i);
    const open = numericValue(openPair && openPair.value);
    const high = numericValue(highPair && highPair.value);
    const low = numericValue(lowPair && lowPair.value);
    const close = numericValue(closePair && closePair.value);
    const validCandle = [open, high, low, close].every(Number.isFinite)
      && high >= Math.max(open, close) && low <= Math.min(open, close);
    const symbolMatch = cleanText(symbolPair && symbolPair.value, 32).match(/[A-Z][A-Z0-9._-]{0,11}/i);
    const normalizedTitle = cleanText(title, 120)
      || (symbolMatch ? symbolMatch[0].toUpperCase() + ' 最新行情' : '最新行情');
    return normalizeFinancePayload({
      title: normalizedTitle,
      symbol: symbolMatch ? symbolMatch[0].toUpperCase() : '',
      primaryValue: cleanText(primaryPair && primaryPair.value, 64),
      secondaryValue: cleanText(changePair && changePair.value, 96),
      trend: trendFor(primaryPair && primaryPair.value, changePair && changePair.value),
      periods: [],
      metrics: values.filter((pair) => !/^(?:项目|数值|item|value)$/i.test(pair.label)),
      chart: validCandle ? {
        kind: 'candlestick',
        candles: [{
          x: cleanText(datePair && datePair.value, 64) || '最新交易日',
          open,
          high,
          low,
          close
        }]
      } : undefined
    });
  }

  function ohlcTableParts(content) {
    if (!(content instanceof Element) || typeof content.querySelectorAll !== 'function') return [];
    const title = Array.from(content.querySelectorAll('h1, h2, h3, h4, h5, h6'))
      .find((heading) => visible(heading));
    return Array.from(content.querySelectorAll('table')).slice(0, 8).map((table) => {
      if (!visible(table) || table.closest('[' + ROOT_ATTRIBUTE + ']')) return null;
      const pairs = Array.from(table.querySelectorAll('tr')).map((row) => {
        const cells = Array.from(row.querySelectorAll(':scope > th, :scope > td'));
        return cells.length === 2 ? {
          label: cleanText(cells[0].innerText || cells[0].textContent, 64),
          value: cleanText(cells[1].innerText || cells[1].textContent, 96)
        } : null;
      }).filter(Boolean);
      const payload = financePayloadFromPairs(
        pairs,
        cleanText(title && (title.innerText || title.textContent), 120)
      );
      if (!payload.primaryValue || !payload.chart || payload.chart.kind !== 'candlestick') return null;
      table.setAttribute(ROOT_ATTRIBUTE, 'finance');
      return {
        type: 'rich_card',
        text: payload.title,
        kind: 'finance',
        richContent: {
          schema: SCHEMA,
          kind: 'finance',
          source: 'official_dom',
          payload
        }
      };
    }).filter(Boolean).slice(0, 4);
  }

  function sampleChartGeometry(geometry) {
    try {
      if (!geometry || typeof geometry.getBBox !== 'function' ||
          typeof geometry.getTotalLength !== 'function' ||
          typeof geometry.getPointAtLength !== 'function') return [];
      const box = geometry.getBBox();
      const width = Number(box && box.width);
      const height = Number(box && box.height);
      const length = Number(geometry.getTotalLength());
      const rect = typeof geometry.getBoundingClientRect === 'function'
        ? geometry.getBoundingClientRect()
        : null;
      const displayWidth = rect ? Number(rect.width) : width;
      const displayHeight = rect ? Number(rect.height) : height;
      const style = typeof window.getComputedStyle === 'function'
        ? window.getComputedStyle(geometry)
        : null;
      if (!Number.isFinite(width) || !Number.isFinite(height) || !Number.isFinite(length) ||
          !Number.isFinite(displayWidth) || !Number.isFinite(displayHeight) ||
          width < 20 || height < 2 || displayWidth < 120 || displayHeight < 12 ||
          length < width * 0.8 || length > 1000000 ||
          (style && (style.display === 'none' || style.visibility === 'hidden' || Number(style.opacity) === 0))) return [];
      const count = Math.max(24, Math.min(MAX_VISIBLE_CHART_POINTS, Math.ceil(displayWidth / 6)));
      const sampled = [];
      for (let index = 0; index < count; index += 1) {
        const point = geometry.getPointAtLength(length * index / (count - 1));
        const x = Number(point && point.x);
        const y = Number(point && point.y);
        if (Number.isFinite(x) && Number.isFinite(y) && Math.abs(x) <= 1000000 && Math.abs(y) <= 1000000) {
          sampled.push({ x, y });
        }
      }
      if (sampled.length < 20) return [];
      let forward = 0;
      let backward = 0;
      for (let index = 1; index < sampled.length; index += 1) {
        const delta = sampled[index].x - sampled[index - 1].x;
        if (delta > width * 0.001) forward += 1;
        if (delta < width * -0.001) backward += 1;
      }
      const xValues = sampled.map((point) => point.x);
      const yValues = sampled.map((point) => point.y);
      const xSpan = Math.max(...xValues) - Math.min(...xValues);
      const minimumY = Math.min(...yValues);
      const maximumY = Math.max(...yValues);
      if (forward < sampled.length * 0.55 || backward > sampled.length * 0.2 ||
          xSpan < width * 0.75 || maximumY - minimumY < 10) return [];
      return sampled.map((point, index) => ({
        x: String(index),
        y: Number((maximumY - point.y).toFixed(4))
      }));
    } catch (_) {
      return [];
    }
  }

  function candleMetric(label, aliases) {
    const pattern = new RegExp(
      '(?:^|[\\s,;，；])(?:' + aliases + ')\\s*[:：]?\\s*([-+]?\\d[\\d,]*(?:\\.\\d+)?)',
      'i'
    );
    const match = cleanText(label, 320).match(pattern);
    if (!match) return undefined;
    const parsed = Number(String(match[1]).replace(/,/g, ''));
    return Number.isFinite(parsed) ? parsed : undefined;
  }

  function candleLabel(label, index) {
    const text = cleanText(label, 320);
    const isoDate = text.match(/\b20\d{2}[-/.]\d{1,2}[-/.]\d{1,2}(?:[ T]\d{1,2}:\d{2})?\b/);
    if (isoDate) return cleanText(isoDate[0], 64);
    const chineseDate = text.match(/20\d{2}年\d{1,2}月\d{1,2}日(?:\s*\d{1,2}:\d{2})?/);
    if (chineseDate) return cleanText(chineseDate[0], 64);
    const prefix = cleanText(text.split(/\bOpen\b|开盘价?|開盤價?/i)[0], 64)
      .replace(/[,:，：;；-]+$/g, '').trim();
    return prefix || String(index + 1);
  }

  function visibleCandlestickChart(root) {
    if (!(root instanceof Element) || typeof root.querySelectorAll !== 'function') return undefined;
    const candles = [];
    const seen = new Set();
    Array.from(root.querySelectorAll('[aria-label], svg title')).slice(0, 1500).forEach((node, index) => {
      const label = cleanText(
        typeof node.getAttribute === 'function' && node.getAttribute('aria-label') || node.textContent,
        320
      );
      const open = candleMetric(label, 'Open|开盘价?|開盤價?');
      const high = candleMetric(label, 'High|最高价?|最高價?');
      const low = candleMetric(label, 'Low|最低价?|最低價?');
      const close = candleMetric(label, 'Close|收盘价?|收盤價?');
      if (![open, high, low, close].every(Number.isFinite) ||
          high < Math.max(open, close) || low > Math.min(open, close)) return;
      const x = candleLabel(label, index);
      const identity = [x, open, high, low, close].join(':');
      if (seen.has(identity)) return;
      seen.add(identity);
      candles.push({ x, open, high, low, close });
    });
    return candles.length > 1 ? { kind: 'candlestick', candles: candles.slice(0, 512) } : undefined;
  }

  function visibleChart(root) {
    if (!(root instanceof Element)) return undefined;
    const candlesticks = visibleCandlestickChart(root);
    if (candlesticks) return candlesticks;
    let selected = [];
    let selectedRange = 0;
    Array.from(root.querySelectorAll('svg path[d], svg polyline[points]')).forEach((geometry) => {
      const points = sampleChartGeometry(geometry);
      if (points.length < 2) return;
      const values = points.map((point) => point.y);
      const range = Math.max(...values) - Math.min(...values);
      if (range > selectedRange) {
        selected = points;
        selectedRange = range;
      }
    });
    return selected.length > 1 ? { kind: 'line', points: selected } : undefined;
  }

  function qualifiesAsFinance(root) {
    const lines = cleanLines(root.innerText || root.textContent);
    const controls = periodControls(root);
    const chart = root.querySelector('[role="application"], canvas, [aria-label*="chart" i]');
    return controls.length >= 4 && Boolean(firstPrice(lines)) && Boolean(chart);
  }

  function rootFor(seed, content) {
    let current = seed instanceof HTMLElement ? seed : null;
    while (current && current !== content && content.contains(current)) {
      if (visible(current) && qualifiesAsFinance(current)) return current;
      current = current.parentElement;
    }
    return null;
  }

  function financeRoots(content) {
    if (!(content instanceof HTMLElement)) return [];
    const seeds = Array.from(content.querySelectorAll(
      '[role="radiogroup"], [role="application"], canvas, [aria-label*="chart" i]'
    ));
    const roots = [];
    seeds.forEach((seed) => {
      const root = rootFor(seed, content);
      if (!root || roots.some((known) => known === root || known.contains(root))) return;
      roots.push(root);
    });
    return roots.slice(0, 4);
  }

  function payloadFor(root) {
    const lines = cleanLines(root.innerText || root.textContent);
    const primaryValue = firstPrice(lines);
    const priceIndex = lines.indexOf(primaryValue);
    const title = lines.slice(0, Math.max(0, priceIndex)).reverse().find((line) =>
      !PERIOD.test(line) && !METRIC_LABELS.includes(line) && line.length <= 120
    ) || '';
    const secondaryValue = lines.slice(priceIndex + 1).find((line) =>
      /(?:^|\s)[+\-−]?(?:(?:US|CA|AU|HK|NT)\s*)?[$€£¥]?\s*[0-9][0-9,.]*(?:\.[0-9]+)?\s*\([+\-−]?[0-9.]+%\)/i.test(line)
    ) || '';
    const symbolMatch = title.match(/\(([A-Za-z0-9._-]{1,24})\)\s*$/);
    const periods = periodControls(root).map(({ node, label }) => ({
      id: safeToken(label, 16),
      label,
      selected: node.getAttribute('aria-checked') === 'true'
        || node.getAttribute('aria-selected') === 'true'
        || node.getAttribute('aria-pressed') === 'true'
        || node.getAttribute('data-state') === 'active'
    }));
    return normalizeFinancePayload({
      title,
      symbol: symbolMatch ? symbolMatch[1] : '',
      primaryValue,
      secondaryValue,
      trend: trendFor(primaryValue, secondaryValue),
      periods,
      metrics: metricPairs(lines),
      chart: visibleChart(root)
    });
  }

  function parts(content) {
    const finance = financeRoots(content).map((root) => {
      const payload = payloadFor(root);
      root.setAttribute(ROOT_ATTRIBUTE, 'finance');
      return {
        type: 'rich_card',
        text: payload.title || '市场行情',
        kind: 'finance',
        richContent: {
          schema: SCHEMA,
          kind: 'finance',
          source: 'official_dom',
          payload
        }
      };
    }).filter((part) => part.richContent.payload.primaryValue);
    const tableFinance = ohlcTableParts(content);
    const common = commonRichContent && typeof commonRichContent.parts === 'function'
      ? commonRichContent.parts(content)
      : [];
    return finance.concat(tableFinance, common).slice(0, 16);
  }

  function fromAuthorizedEnvelope(envelope, authorize) {
    if (!envelope || envelope.schema !== 'yilong.authorized-provider-response.v1' ||
        envelope.providerId !== 'chatgpt' || typeof authorize !== 'function') return [];
    const common = commonRichContent && typeof commonRichContent.fromAuthorizedEnvelope === 'function'
      ? commonRichContent.fromAuthorizedEnvelope(envelope, authorize)
      : [];
    const finance = (Array.isArray(envelope.parts) ? envelope.parts : []).slice(0, 16)
      .filter((part) => part && cleanText(part.kind, 48).toLowerCase() === 'finance')
      .filter(() => authorize(envelope.providerId, envelope.authorizationId, 'finance'))
      .map((part) => {
        const payload = normalizeFinancePayload(part.payload);
        return payload.title && payload.primaryValue
          ? {
              type: 'rich_card',
              text: payload.title,
              kind: 'finance',
              richContent: {
                schema: SCHEMA,
                kind: 'finance',
                source: 'private_response',
                payload
              }
            }
          : null;
      }).filter(Boolean);
    return finance.concat(common).slice(0, 16);
  }

  function owns(node) {
    return node instanceof Element && (Boolean(node.closest('[' + ROOT_ATTRIBUTE + ']')) ||
      Boolean(commonRichContent && commonRichContent.owns(node)));
  }

  window.__elonChatGptRichContent = Object.freeze({
    version: API_VERSION,
    schema: SCHEMA,
    parts,
    owns,
    financeRoots,
    financePayloadFromPairs,
    ohlcTableParts,
    sampleChartGeometry,
    visibleCandlestickChart,
    visibleChart,
    normalizeFinancePayload,
    fromAuthorizedEnvelope
  });
})();
