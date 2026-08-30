(function () {
  'use strict';

  const root = window;
  if (root.__elonWinChatGptRealtimeVoiceJsonDelta || location.origin !== 'https://chatgpt.com') return;

  const MAX_CHANNELS = 16;
  const MAX_DELTA_KEYS = 8;
  const MAX_PATH_CHARS = 512;
  const MAX_PATH_DEPTH = 32;
  const MAX_PATCH_OPERATIONS = 64;
  const MAX_COLLECTION_ITEMS = 4096;
  const MAX_OBJECT_KEYS = 256;
  const MAX_RESULT_CHARS = 128 * 1024;
  const OPERATIONS = new Set(['patch', 'add', 'remove', 'replace', 'append', 'truncate']);
  const BLOCKED_OBJECT_KEYS = new Set(['__proto__', 'constructor', 'prototype']);

  function safeObjectKey(value) {
    return typeof value === 'string' && !BLOCKED_OBJECT_KEYS.has(value);
  }

  function clone(value) {
    if (value === undefined) return undefined;
    try { return JSON.parse(JSON.stringify(value)); } catch (_) { return null; }
  }

  function resultWithinLimit(value) {
    try { return typeof value === 'string' ? value.length <= MAX_RESULT_CHARS
      : JSON.stringify(value).length <= MAX_RESULT_CHARS; }
    catch (_) { return false; }
  }

  function parsePath(path) {
    if (path === '') return [];
    const segments = String(path).replace(/^\//, '').split('/');
    if (segments.length > MAX_PATH_DEPTH) return null;
    return segments.map((segment) => {
      const decoded = segment.replace(/~1/g, '/').replace(/~0/g, '~');
      return /^(?:0|[1-9][0-9]*)$/.test(decoded) ? Number(decoded) : decoded;
    });
  }

  function getValue(parent, key) {
    if (Array.isArray(parent) && Number.isInteger(key) && key >= 0 && key < parent.length) {
      return parent[key];
    }
    if (parent && typeof parent === 'object' && !Array.isArray(parent) && safeObjectKey(key)) {
      return parent[key];
    }
    return undefined;
  }

  function setValue(parent, key, value, insertArray) {
    if (Array.isArray(parent) && Number.isInteger(key) && key >= 0 && key <= MAX_COLLECTION_ITEMS) {
      while (parent.length < key) parent.push(null);
      if (insertArray && key < parent.length) parent.splice(key, 0, clone(value));
      else parent[key] = clone(value);
      return true;
    }
    if (parent && typeof parent === 'object' && !Array.isArray(parent) && safeObjectKey(key)) {
      if (!Object.prototype.hasOwnProperty.call(parent, key) &&
        Object.keys(parent).length >= MAX_OBJECT_KEYS) return false;
      parent[key] = clone(value);
      return true;
    }
    return false;
  }

  function removeValue(parent, key) {
    if (Array.isArray(parent) && Number.isInteger(key) && key >= 0 && key < parent.length) {
      parent.splice(key, 1);
      return true;
    }
    if (parent && typeof parent === 'object' && !Array.isArray(parent) && safeObjectKey(key)) {
      delete parent[key];
      return true;
    }
    return false;
  }

  function childOrCreate(parent, key, nextIsIndex) {
    const current = getValue(parent, key);
    if (current && typeof current === 'object') return current;
    const created = nextIsIndex ? [] : {};
    return setValue(parent, key, created, false) ? getValue(parent, key) : null;
  }

  function appendValue(parent, key, value) {
    const current = getValue(parent, key);
    let next = value;
    if (typeof current === 'string' && typeof value === 'string') next = current + value;
    else if (Array.isArray(current)) next = current.concat(Array.isArray(value) ? value : [value]);
    else if (current && typeof current === 'object' && value && typeof value === 'object' &&
      !Array.isArray(current) && !Array.isArray(value)) {
      next = { ...current };
      Object.keys(value).slice(0, MAX_OBJECT_KEYS).forEach((field) => { next[field] = clone(value[field]); });
    }
    return setValue(parent, key, next, false);
  }

  function truncateValue(parent, key, value) {
    const size = Number(value);
    if (!Number.isInteger(size) || size < 0 || size > MAX_COLLECTION_ITEMS) return false;
    const current = getValue(parent, key);
    if (typeof current === 'string') return setValue(parent, key, current.slice(0, size), false);
    if (Array.isArray(current)) return setValue(parent, key, current.slice(0, size), false);
    return true;
  }

  function decodeDelta(value, previous, inherit) {
    if (!value || typeof value !== 'object' || Array.isArray(value) ||
      Object.keys(value).length > MAX_DELTA_KEYS) return null;
    const channelValue = value.channel !== undefined ? value.channel : value.c;
    const pathValue = value.path !== undefined ? value.path : value.p;
    const operationValue = value.op !== undefined ? value.op : value.o;
    const hasValue = Object.prototype.hasOwnProperty.call(value, 'value') ||
      Object.prototype.hasOwnProperty.call(value, 'v');
    const decoded = {
      channel: Number.isInteger(channelValue) ? channelValue : (inherit ? previous.channel : 0),
      path: typeof pathValue === 'string' ? pathValue : (inherit ? previous.path : ''),
      operation: typeof operationValue === 'string' ? operationValue : (inherit ? previous.operation : ''),
      value: hasValue ? (value.value !== undefined ? value.value : value.v) :
        (inherit ? previous.value : undefined),
      hasValue: hasValue || (inherit && previous.hasValue),
    };
    if (decoded.path.length > MAX_PATH_CHARS || !OPERATIONS.has(decoded.operation)) return null;
    return decoded;
  }

  function applyOperation(parent, key, delta, decodeStandalone) {
    if (delta.operation === 'add') return setValue(parent, key, delta.value, true);
    if (delta.operation === 'replace') return setValue(parent, key, delta.value, false);
    if (delta.operation === 'remove') return removeValue(parent, key);
    if (delta.operation === 'append') return appendValue(parent, key, delta.value);
    if (delta.operation === 'truncate') return truncateValue(parent, key, delta.value);
    if (delta.operation !== 'patch' || !Array.isArray(delta.value) ||
      delta.value.length > MAX_PATCH_OPERATIONS) return false;
    const holder = { root: clone(getValue(parent, key)) };
    for (const patch of delta.value) {
      const nested = decodeStandalone(patch);
      if (!nested || !applyAtPath(holder, 'root', nested, decodeStandalone)) return false;
    }
    return setValue(parent, key, holder.root, false);
  }

  function applyAtPath(holder, rootKey, delta, decodeStandalone) {
    const segments = parsePath(delta.path);
    if (!segments) return false;
    if (!segments.length) return applyOperation(holder, rootKey, delta, decodeStandalone);
    let parent = holder[rootKey];
    if (!parent || typeof parent !== 'object') {
      parent = Number.isInteger(segments[0]) ? [] : {};
      holder[rootKey] = parent;
    }
    for (let index = 0; index < segments.length - 1; index += 1) {
      parent = childOrCreate(parent, segments[index], Number.isInteger(segments[index + 1]));
      if (!parent) return false;
    }
    return applyOperation(parent, segments[segments.length - 1], delta, decodeStandalone);
  }

  function create() {
    const valuesByChannel = new Map();
    let previous = { channel: 0, path: '', operation: 'add', value: undefined, hasValue: false };
    function standalone(value) { return decodeDelta(value, previous, false); }
    return Object.freeze({
      apply(encoded) {
        const decoded = decodeDelta(encoded, previous, true);
        if (!decoded || decoded.channel < 0 || decoded.channel >= MAX_CHANNELS) return null;
        previous = decoded;
        const holder = { root: clone(valuesByChannel.get(decoded.channel)) };
        if (!applyAtPath(holder, 'root', decoded, standalone) || !resultWithinLimit(holder.root)) return null;
        valuesByChannel.set(decoded.channel, clone(holder.root));
        return clone(holder.root);
      },
      reset() {
        valuesByChannel.clear();
        previous = { channel: 0, path: '', operation: 'add', value: undefined, hasValue: false };
      },
    });
  }

  root.__elonWinChatGptRealtimeVoiceJsonDelta = Object.freeze({ version: 1, create });
})();
