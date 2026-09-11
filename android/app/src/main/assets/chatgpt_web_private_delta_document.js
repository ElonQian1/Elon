(function (root, factory) {
  'use strict';
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateDeltaDocument = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const own = (value, key) => Object.prototype.hasOwnProperty.call(value, key);
  const object = (value) => value !== null && typeof value === 'object' && !Array.isArray(value);
  const unsafe = (key) => key === '__proto__' || key === 'constructor' || key === 'prototype';
  const MAX_TEXT = 2 * 1024 * 1024;
  const MAX_ARRAY = 4096;
  const MAX_UNITS = 8 * 1024 * 1024;
  const REMOVE = Symbol('remove');

  function check(condition) {
    if (!condition) throw new Error('invalid_delta');
  }

  function charge(budget, units) {
    budget.units += units;
    check(budget.units <= budget.remaining);
  }

  function clone(value, budget, depth = 0) {
    check(depth <= 32 && ++budget.nodes <= 32768);
    if (value === null || typeof value === 'boolean') { charge(budget, 4); return value; }
    if (typeof value === 'number') {
      check(Number.isFinite(value));
      charge(budget, 16);
      return value;
    }
    if (typeof value === 'string') {
      check(value.length <= MAX_TEXT);
      charge(budget, value.length);
      return value;
    }
    check(value && typeof value === 'object');
    if (Array.isArray(value)) {
      check(value.length <= MAX_ARRAY);
      return value.map((item) => clone(item, budget, depth + 1));
    }
    const keys = Object.keys(value);
    check(keys.length <= MAX_ARRAY);
    const result = {};
    for (const key of keys) {
      check(!unsafe(key) && key.length <= 1024);
      charge(budget, key.length + 8);
      result[key] = clone(value[key], budget, depth + 1);
    }
    return result;
  }

  function pointer(path) {
    check(typeof path === 'string' && path.length <= 1024);
    if (!path) return [];
    const parts = (path[0] === '/' ? path.slice(1) : path).split('/');
    check(parts.length <= 32);
    return parts.map((part) => {
      const key = part.replace(/~1/g, '/').replace(/~0/g, '~');
      check(!unsafe(key));
      if (/^(?:0|[1-9]\d*)$/.test(part)) {
        const index = Number(part);
        check(Number.isSafeInteger(index) && index < MAX_ARRAY);
        return index;
      }
      return key;
    });
  }

  function operation(input, previous, budget) {
    check(object(input) && ++budget.operations <= 128);
    const op = own(input, 'o') ? input.o : previous && previous.op;
    const path = own(input, 'p') ? input.p : previous ? previous.path : '';
    check(typeof op === 'string' && /^(add|replace|remove|append|truncate|patch)$/.test(op));
    const segments = pointer(path == null ? '' : path);
    charge(budget, String(path || '').length + 16);
    if (op === 'remove') return { op, path, segments };
    check(own(input, 'v'));
    if (op === 'patch') {
      check(Array.isArray(input.v) && input.v.length <= 128);
      return { op, path, segments, value: input.v.map((child) => operation(child, null, budget)) };
    }
    const value = clone(input.v, budget);
    if (op === 'truncate') check(Number.isSafeInteger(value) && value >= 0 && value <= MAX_TEXT);
    return { op, path, segments, value };
  }

  function updateValue(current, delta) {
    const value = delta.value;
    switch (delta.op) {
      case 'remove': return REMOVE;
      case 'add':
      case 'replace': return value;
      case 'patch': {
        let result = current;
        for (const child of value) {
          result = update(result, child.segments, child);
          if (result === REMOVE) result = undefined;
        }
        return result;
      }
      case 'append':
        if (typeof current === 'string') {
          check(typeof value === 'string' && current.length + value.length <= MAX_TEXT);
          return current + value;
        }
        if (Array.isArray(current)) {
          const additions = Array.isArray(value) ? value : [value];
          check(current.length + additions.length <= MAX_ARRAY);
          return current.concat(additions);
        }
        if (object(current) && object(value)) {
          const result = Object.assign({}, current, value);
          check(Object.keys(result).length <= MAX_ARRAY);
          return result;
        }
        return value;
      case 'truncate':
        if (typeof current === 'string') return current.substring(0, value);
        if (Array.isArray(current)) {
          check(value <= MAX_ARRAY);
          const result = current.slice();
          result.length = value;
          return result;
        }
        return current;
      default: throw new Error('invalid_delta');
    }
  }

  // Only ancestors of the changed value are copied. Prior emitted frames stay
  // immutable without cloning the whole conversation for each incoming token.
  function update(current, segments, delta) {
    if (!segments.length) return updateValue(current, delta);
    const [key, ...tail] = segments;
    if (current === undefined) current = typeof key === 'number' ? [] : {};
    check(current && typeof current === 'object');
    const array = Array.isArray(current);
    check(!array || (typeof key === 'number' && key <= current.length));
    const result = array ? current.slice() : Object.assign({}, current);
    const next = update(current[key], tail, delta);
    if (next === REMOVE) {
      if (array) result.splice(key, 1);
      else delete result[key];
    } else if (array && !tail.length && delta.op === 'add') {
      check(result.length < MAX_ARRAY);
      result.splice(key, 0, next);
    } else result[key] = next;
    check(array ? result.length <= MAX_ARRAY : Object.keys(result).length <= MAX_ARRAY);
    return result;
  }

  function create() {
    let channels = new Map();
    let previous = { channel: 0, path: '', op: 'add' };
    let units = 0;
    let failed = false;
    function reset() {
      channels = new Map();
      previous = { channel: 0, path: '', op: 'add' };
      units = 0;
      failed = false;
    }
    function apply(input) {
      if (failed) return { ok: false, error: 'delta_decode' };
      try {
        check(object(input));
        const channel = own(input, 'c') ? input.c : previous.channel;
        check(Number.isSafeInteger(channel) && channel >= 0 && channel < 4096);
        check(channels.has(channel) || channels.size < 32);
        const budget = { units: 0, nodes: 0, operations: 0, remaining: MAX_UNITS - units };
        const delta = operation(input, previous, budget);
        check(units + budget.units <= MAX_UNITS);
        const value = update(channels.get(channel), delta.segments, delta);
        channels.set(channel, value === REMOVE ? undefined : value);
        previous = { channel, path: delta.path, op: delta.op };
        units += budget.units;
        return { ok: true, channel, value: value === REMOVE ? undefined : value };
      } catch (_) {
        failed = true;
        return { ok: false, error: 'delta_decode' };
      }
    }
    return Object.freeze({ apply, reset });
  }

  // Older unlabelled socket captures omit the outer patch marker. Normalize
  // that envelope only; all document mutation is shared with strict SSE v1.
  function isCompactPayload(payload) {
    if (!object(payload)) return false;
    // Symbolic socket envelopes (c: "patch") wrap full messages, not v1 channels.
    if (typeof payload.c === 'string') return false;
    const keys = Object.keys(payload);
    return keys.every((key) => /^(c|p|o|v)$/.test(key)) && (
      (own(payload, 'c') && payload.v && typeof payload.v === 'object') ||
      typeof payload.o === 'string' || typeof payload.p === 'string' ||
      (keys.length === 1 && own(payload, 'v'))
    );
  }

  function createLegacy(onDocument) {
    const decoder = create();
    let started = false;
    let continuation = null;
    function reset() {
      decoder.reset();
      started = false;
      continuation = null;
    }
    function accept(payload) {
      if (!object(payload)) return false;
      const seed = Number.isSafeInteger(payload.c) && object(payload.v) &&
        !own(payload, 'o') && !own(payload, 'p');
      const explicitSeed = Number.isSafeInteger(payload.c) && object(payload.v) &&
        payload.p === '' && (payload.o === 'add' || payload.o === 'replace');
      if (!started && !seed && !explicitSeed) return false;
      const batch = Array.isArray(payload.v) && payload.v.length > 0 &&
        payload.v.every((item) => object(item) && item.o && typeof item.p === 'string');
      let normalized = payload;
      if (seed) normalized = Object.assign({}, payload, { o: 'add', p: '' });
      else if (!own(payload, 'o') && batch) normalized = Object.assign({}, payload, { o: 'patch', p: '' });
      else if (continuation && Object.keys(payload).length === 1 && own(payload, 'v')) {
        normalized = Object.assign({}, continuation, payload);
      } else if (!isCompactPayload(payload)) return false;
      const result = decoder.apply(normalized);
      if (!result.ok) return false;
      started = true;
      const last = normalized.o === 'patch' ? normalized.v[normalized.v.length - 1] : normalized;
      continuation = last && last.o === 'append' ? { o: 'append',
        p: normalized.o === 'patch' ? (normalized.p || '') + last.p : last.p
      } : null;
      return onDocument(result.value);
    }
    return Object.freeze({ accept, reset, handles: isCompactPayload });
  }

  return Object.freeze({ version: 2, create, createLegacy, isCompactPayload });
});
