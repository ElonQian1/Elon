(function (root, factory) {
  'use strict';
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateDeltaDocument = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const MAX_PATCH_TEXT_LENGTH = 524288;
  const MAX_PATCH_ARRAY_LENGTH = 128;

  function createLegacy(onDocument) {
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
        return compactDocument ? onDocument(compactDocument) : false;
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
      return changed && onDocument(compactDocument);
    }

    function reset() {
      compactDocument = null;
      compactContinuation = null;
    }

    return Object.freeze({ accept: applyCompactPayload, reset });
  }

  return Object.freeze({ version: 1, createLegacy });
});
