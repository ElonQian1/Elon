(function () {
  'use strict';

  var VERSION = 2;
  var MAX_PERIODS = 12;
  var MAX_POINTS_PER_PERIOD = 192;
  if (location.origin !== 'https://chatgpt.com') return;
  var base = window.__elonChatGptPrivateStreamPolicy;
  if (!base || typeof base.financePartFromWidget !== 'function') return;
  if (base.__elonWinPrivateFinancePeriodsWrapped === true) return;

  function cleanText(value, limit) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim()
      .slice(0, limit || 240);
  }

  function safeToken(value, limit) {
    return cleanText(value, limit || 24).replace(/[^A-Za-z0-9._-]/g, '');
  }

  function cloneJson(value) {
    try { return JSON.parse(JSON.stringify(value)); }
    catch (_) { return null; }
  }

  function boundedSamples(values) {
    var source = Array.isArray(values) ? values : [];
    if (source.length <= MAX_POINTS_PER_PERIOD) return source.slice();
    return Array.from({ length: MAX_POINTS_PER_PERIOD }, function (_, index) {
      return source[Math.round(index * (source.length - 1) / (MAX_POINTS_PER_PERIOD - 1))];
    });
  }

  function trend(summary) {
    var color = cleanText(summary && summary.price_change_color, 32).toLowerCase();
    var change = cleanText(summary && summary.price_change_text, 96);
    if (/^(?:success|positive|green)$/.test(color) || /^\+/.test(change)) return 'positive';
    if (/^(?:danger|negative|error|red)$/.test(color) || /^(?:-|−)/.test(change)) return 'negative';
    return 'neutral';
  }

  function periodView(widget, label, selected) {
    var configs = widget && widget.timeframe_configs;
    var config = configs && configs[label];
    if (!config || typeof config !== 'object') return null;
    var chart = config.chart && typeof config.chart === 'object' ? config.chart : {};
    var points = boundedSamples(chart.data).map(function (point, index) {
      return {
        x: cleanText(point && (point.formatted || point.timestamp || index), 64),
        y: Number(point && point.close),
      };
    }).filter(function (point) { return point.x && Number.isFinite(point.y); });
    if (points.length < 2) return null;
    var summary = config.summary && typeof config.summary === 'object' ? config.summary : {};
    return {
      id: safeToken(label, 16).toLowerCase(),
      label: cleanText(label, 16),
      selected: selected === true,
      primaryValue: cleanText(
        summary.price_text || widget.current_price_text ||
        widget.fallback_summary && widget.fallback_summary.price_text,
        64
      ),
      secondaryValue: cleanText(summary.price_change_text, 96),
      trend: trend(summary),
      chart: { kind: 'line', points: points },
    };
  }

  function periodViews(widget) {
    if (!widget || typeof widget !== 'object' || Array.isArray(widget)) return [];
    var configs = widget.timeframe_configs && typeof widget.timeframe_configs === 'object'
      ? widget.timeframe_configs
      : {};
    var order = Array.isArray(widget.timeframe_order)
      ? widget.timeframe_order.slice(0, MAX_PERIODS)
      : Object.keys(configs).slice(0, MAX_PERIODS);
    var selected = cleanText(widget.default_range, 16);
    return order.map(function (label) {
      return periodView(widget, cleanText(label, 16), cleanText(label, 16) === selected);
    }).filter(function (view) { return view && view.id && view.label; });
  }

  function financePartFromWidget(widget) {
    var part;
    try { part = base.financePartFromWidget(widget); }
    catch (_) { return null; }
    var views = periodViews(widget);
    if (!part || !views.length || !part.richContent || !part.richContent.payload) return part;
    var enriched = cloneJson(part);
    if (!enriched) return part;
    enriched.richContent.payload.periodViews = views;
    return enriched;
  }

  function financePartsFromMetadata(metadata) {
    var references = metadata && Array.isArray(metadata.content_references)
      ? metadata.content_references
      : [];
    return references.slice(0, MAX_PERIODS).map(function (reference) {
      var initialState = reference && reference.dil && reference.dil.initialState;
      return initialState && typeof initialState === 'object' && !Array.isArray(initialState)
        ? financePartFromWidget(initialState)
        : null;
    }).filter(Boolean).slice(0, 4);
  }

  var wrapped = Object.freeze(Object.assign({}, base, {
    __elonWinPrivateFinancePeriodsWrapped: true,
    financePartFromWidget: financePartFromWidget,
    financePartsFromMetadata: financePartsFromMetadata,
  }));
  window.__elonChatGptPrivateStreamPolicy = wrapped;
  window.__elonWinChatGptPrivateFinancePeriods = Object.freeze({
    version: VERSION,
    basePolicy: base,
    periodViews: periodViews,
    policy: wrapped,
  });
})();
