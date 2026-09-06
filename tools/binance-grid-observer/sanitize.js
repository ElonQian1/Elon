(function (root) {
  'use strict';
  const ORIGIN = 'https://www.binance.com';
  const CHANNEL = 'binance-grid-observer-v1';
  const SCHEMA_VERSION = 'binance-grid-observation.v1';
  const limits = Object.freeze({ bodyBytes: 131072, observationBytes: 16384, depth: 6,
    nodes: 128, fields: 32, arraySamples: 3, concurrency: 4, readMs: 5000, lifetimeMs: 900000 });
  const KEYS = new Set(('code msg message success data total count page pageSize limit rows list records items ' +
    'symbol symbols side direction strategyId strategyType strategyStatus strategyName gridId gridType gridNum ' +
    'gridCount lowerPrice upperPrice minPrice maxPrice investment initialValue leverage marginType marginMode ' +
    'stopType status orderStatus orders positions quantity price amount profit pnl realizedPnl unrealizedPnl ' +
    'gridProfit totalProfit fundingFee fee commission createTime updateTime time startTime endTime ' +
    'openPositionOnCreation autoAddMargin stopClosePositions tpslClosePositions triggerPrice stopPrice ' +
    'stopLoss takeProfit stopLossPrice takeProfitPrice entryPrice markPrice liquidationPrice ' +
    'working pending finished available balance asset assets currency intervals type result hasMore ' +
    'nextCursor cursor enabled minQty maxQty stepSize tickSize minNotional maxNotional filters ' +
    'algoId algoType algoOrderType investmentAmount lowerLimit upperLimit trailingUp trailingDown').split(' '));
  const SEGMENTS = new Set(('bapi fapi api futures future um v1 v2 v3 v4 private public friendly ' +
    'grid grids strategy strategies robot robots bot bots future-grid futures-grid grid-trading trading-bot ' +
    'trading-bots futuresGrid gridStrategy strategy-grid algo order orders list query detail details info ' +
    'get create update modify stop cancel close history running pending finished profit pnl position positions ' +
    'leverage parameters parameter config symbol symbols all current user get-strategy query-strategy ' +
    'get-grid query-grid').split(' '));
  const CANDIDATES = new Set(('grid grids strategy strategies robot robots bot bots future-grid futures-grid ' +
    'grid-trading trading-bot trading-bots futuresgrid gridstrategy strategy-grid get-strategy query-strategy ' +
    'get-grid query-grid').split(' '));
  const METHODS = new Set(['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS']);
  const OUTCOMES = new Set(['json', 'non_json', 'unreadable', 'too_large', 'timeout', 'network_error']);
  const LEAVES = new Set(['string', 'number', 'boolean', 'null', 'truncated']);
  const encoder = new TextEncoder();

  function normalizePath(input) {
    try {
      if (typeof input !== 'string' || input.length > 4096) return null;
      const url = new URL(input, ORIGIN);
      if (url.origin !== ORIGIN || url.username || url.password || url.pathname.length > 1024) return null;
      const segments = url.pathname.split('/').filter(Boolean);
      if (segments.length === 0 || segments.length > 20) return null;
      const decoded = segments.map(value => decodeURIComponent(value));
      if (decoded.some(value => /auth|login|logout|token|credential|password|register|deposit|withdraw|transfer|payment|wallet|capital|fiat|otp|captcha|account/i.test(value))) return null;
      if (decoded.some(value => /recommend|ranking|leaderboard|marketplace|copy|follow|discover|popular/i.test(value))) return null;
      if (!decoded.some(value => CANDIDATES.has(value.toLowerCase()))) return null;
      return '/' + decoded.map(value => SEGMENTS.has(value) ? value : '{segment}').join('/');
    } catch (_) { return null; }
  }

  function record(value) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
    const descriptors = Object.getOwnPropertyDescriptors(value);
    if (Object.values(descriptors).some(item => !Object.hasOwn(item, 'value'))) return null;
    return descriptors;
  }

  function shapeOf(value) {
    const budget = { nodes: 0 };
    function visit(item, depth) {
      if (++budget.nodes > limits.nodes || depth >= limits.depth) return { type: 'truncated' };
      if (item === null) return { type: 'null' };
      if (['string', 'number', 'boolean'].includes(typeof item)) return { type: typeof item };
      if (Array.isArray(item)) {
        const items = [];
        for (let index = 0; index < Math.min(item.length, limits.arraySamples) && budget.nodes < limits.nodes; index++) {
          items.push(visit(item[index], depth + 1));
        }
        return { type: 'array', items };
      }
      const descriptors = record(item);
      if (!descriptors) return { type: 'truncated' };
      const fields = Object.create(null);
      let unknownFields = false;
      let selected = 0;
      for (const key of Object.keys(descriptors).sort()) {
        if (!KEYS.has(key) || selected >= limits.fields || budget.nodes >= limits.nodes) {
          unknownFields = true;
          continue;
        }
        fields[key] = visit(descriptors[key].value, depth + 1);
        selected++;
      }
      return { type: 'object', fields, unknownFields };
    }
    try { return visit(value, 0); } catch (_) { return { type: 'truncated' }; }
  }

  function shapeFromJson(text) {
    try {
      if (typeof text !== 'string' || text.length > limits.bodyBytes || encoder.encode(text).length > limits.bodyBytes) return null;
      return shapeOf(JSON.parse(text));
    } catch (_) { return null; }
  }

  function exact(descriptors, keys) {
    return descriptors && Object.keys(descriptors).length === keys.length && keys.every(key => Object.hasOwn(descriptors, key));
  }

  function sanitizeShape(value, budget, depth) {
    if (++budget.nodes > limits.nodes || depth > limits.depth) throw new Error('shape_limit');
    const descriptors = record(value);
    if (!descriptors || !descriptors.type) throw new Error('shape_type');
    const type = descriptors.type.value;
    if (LEAVES.has(type) && exact(descriptors, ['type'])) return { type };
    if (type === 'array' && exact(descriptors, ['type', 'items'])) {
      const items = descriptors.items.value;
      if (!Array.isArray(items) || items.length > limits.arraySamples) throw new Error('shape_array');
      return { type, items: items.map(item => sanitizeShape(item, budget, depth + 1)) };
    }
    if (type === 'object' && exact(descriptors, ['type', 'fields', 'unknownFields'])) {
      const raw = record(descriptors.fields.value);
      if (!raw || Object.keys(raw).length > limits.fields || typeof descriptors.unknownFields.value !== 'boolean') throw new Error('shape_fields');
      const fields = Object.create(null);
      for (const key of Object.keys(raw).sort()) {
        if (!KEYS.has(key)) throw new Error('unknown_field');
        fields[key] = sanitizeShape(raw[key].value, budget, depth + 1);
      }
      return { type, fields, unknownFields: descriptors.unknownFields.value };
    }
    throw new Error('shape_type');
  }

  function sanitizeObservation(value) {
    try {
      const raw = record(value);
      if (!exact(raw, ['schema_version', 'method', 'path', 'status', 'requestShape', 'responseShape', 'outcome'])) return null;
      const path = raw.path.value;
      if (raw.schema_version.value !== SCHEMA_VERSION || !METHODS.has(raw.method.value)
        || typeof path !== 'string' || normalizePath(path) !== path
        || !Number.isInteger(raw.status.value) || raw.status.value < 0 || raw.status.value > 599
        || !OUTCOMES.has(raw.outcome.value)) return null;
      const requestShape = raw.requestShape.value === null ? null : sanitizeShape(raw.requestShape.value, { nodes: 0 }, 0);
      const responseShape = raw.responseShape.value === null ? null : sanitizeShape(raw.responseShape.value, { nodes: 0 }, 0);
      if ((raw.outcome.value === 'json') !== (responseShape !== null)) return null;
      const clean = { schema_version: SCHEMA_VERSION, method: raw.method.value, path, status: raw.status.value,
        requestShape, responseShape, outcome: raw.outcome.value };
      return encoder.encode(JSON.stringify(clean)).length <= limits.observationBytes ? clean : null;
    } catch (_) { return null; }
  }

  const api = Object.freeze({ ORIGIN, CHANNEL, SCHEMA_VERSION, limits, normalizePath, shapeOf, shapeFromJson, sanitizeObservation });
  root.BinanceGridSanitizer = api;
  if (typeof window === 'undefined' && typeof module !== 'undefined' && module.exports) module.exports = api;
})(globalThis);
