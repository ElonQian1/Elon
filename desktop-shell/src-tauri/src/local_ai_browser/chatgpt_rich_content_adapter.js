(function () {
  'use strict';

  const API_VERSION = 2;
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
    const points = source.chart && Array.isArray(source.chart.points)
      ? source.chart.points.slice(0, 512).map((point) => ({
          x: cleanText(point && point.x, 64),
          y: Number(point && point.y)
        })).filter((point) => point.x && Number.isFinite(point.y))
      : [];
    if (points.length > 1) payload.chart = { kind: 'line', points };
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

  function visibleChart(root) {
    if (!(root instanceof Element)) return undefined;
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
    const common = commonRichContent && typeof commonRichContent.parts === 'function'
      ? commonRichContent.parts(content)
      : [];
    return finance.concat(common).slice(0, 16);
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
    sampleChartGeometry,
    visibleChart,
    normalizeFinancePayload,
    fromAuthorizedEnvelope
  });
})();
