(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptModelLabelPolicy = api;
})(typeof window !== 'undefined' ? window : globalThis, function () {
  'use strict';

  function normalize(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim().toLowerCase();
  }

  function isModelLabel(value) {
    const signal = normalize(value);
    if (!signal) return false;
    if (/^(pro|自动|快速|思考|低|中|高|极高)(?:\s+\1)*$/.test(signal)) return true;
    return /\b(?:gpt|o\d|auto|thinking|instant|sol)\b/i.test(signal) ||
      /\b\d+(?:\.\d+)+\b/.test(signal) ||
      /model|模型|能力|推理|思考强度|极速/.test(signal) ||
      /(^|\s)(?:轻度|标准|中度|重度|极高)($|\s)/.test(signal);
  }

  function isModelControl(candidate) {
    if (typeof candidate === 'string') return isModelLabel(candidate);
    return isModelLabel(candidate && candidate.label) ||
      isModelLabel(candidate && candidate.signal);
  }

  return Object.freeze({ isModelControl, isModelLabel });
});
