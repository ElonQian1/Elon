(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextReceipts = api;
})(typeof window === 'object' ? window : null, function (limit) {
  'use strict';
  limit ??= 32;
  if (!Number.isInteger(limit) || limit < 1 || limit > 32) throw Error('receipt_limit_invalid');
  const records = new Map();
  let retiredThrough = 0n, attempts = 0;
  function sequence(id) {
    // ChatGptWebObservedState issues positive, monotonically increasing Long
    // command IDs in base 36. Keep their retired range without retaining text.
    if (!/^mcp_[1-9a-z][a-z0-9]{0,12}$/.test(id || '')) return null;
    let value = 0n;
    for (const char of id.slice(4)) value = value * 36n + BigInt(parseInt(char, 36));
    return value <= 9223372036854775807n ? value : null;
  }
  function retired(id) {
    const value = sequence(id);
    return value !== null && value <= retiredThrough;
  }
  function admit(id) {
    const value = sequence(id);
    if (value === null) return 'invalid_command';
    if (value <= retiredThrough) return 'request_retired';
    if (records.has(id)) return 'request_id_conflict';
    if (records.size >= limit) {
      for (const [key, record] of records) {
        if (record.retirable() !== true) continue;
        const old = sequence(key);
        retiredThrough = old > retiredThrough ? old : retiredThrough;
        records.delete(key);
        break;
      }
    }
    if (value <= retiredThrough) return 'request_retired';
    return records.size < limit ? '' : 'receipt_capacity';
  }
  return Object.freeze({
    get: id => records.get(id), retired, admit,
    set(id, record) {
      if (retired(id) || records.has(id) || sequence(id) === null || records.size >= limit ||
          typeof record?.retirable !== 'function') throw Error('receipt_admission_required');
      records.set(id, record); attempts = Math.min(65535, attempts + 1);
    },
    clear() { records.clear(); retiredThrough = 0n; attempts = 0; },
    attempts: () => attempts,
    size: () => records.size
  });
});
