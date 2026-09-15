(function (root) {
  'use strict';
  const PREFIX = 'elon-social-cache-v1:';
  const AGE = 7 * 86400000;
  function create(storage, now = Date.now) {
    function keys() { return Object.keys(storage).filter(key => key.startsWith(PREFIX)); }
    function remove(key) { try { storage.removeItem(PREFIX + key); } catch {} }
    function get(key) {
      try {
        const entry = JSON.parse(storage.getItem(PREFIX + key));
        if (!entry || entry.v !== 1 || !Number.isFinite(entry.at) || now() - entry.at < 0 || now() - entry.at > AGE) return null;
        return entry.data;
      } catch { return null; }
    }
    function put(key, data) {
      try {
        const value = JSON.stringify({ v: 1, at: now(), data });
        if (value.length > 1000000) return false;
        const entries = keys().filter(k => k !== PREFIX + key).map(k => {
          const raw = storage.getItem(k) || '';
          let at = 0; try { at = JSON.parse(raw).at || 0; } catch {}
          return { key: k, size: raw.length, at };
        }).sort((a, b) => Number(b.key === PREFIX + 'profile') - Number(a.key === PREFIX + 'profile') || b.at - a.at);
        let size = value.length;
        entries.forEach((entry, i) => {
          size += entry.size;
          if (i >= 39 || size > 1800000 || now() - entry.at > AGE) storage.removeItem(entry.key);
        });
        storage.setItem(PREFIX + key, value);
        return true;
      } catch { return false; }
    }
    async function fingerprint(token) {
      if (!token || !root.crypto?.subtle) return null;
      const bytes = await root.crypto.subtle.digest('SHA-256', new TextEncoder().encode(token));
      return Array.from(new Uint8Array(bytes), b => b.toString(16).padStart(2, '0')).join('');
    }
    return {
      get, put, remove,
      clear() { try { keys().forEach(key => storage.removeItem(key)); } catch {} },
      async profile(token, user, allowed = () => true) {
        const hash = await fingerprint(token);
        if (!hash || !allowed()) return null;
        if (user) put('profile', { hash, user: { id: user.id, nickname: user.nickname, account: user.account } });
        const saved = get('profile');
        return saved?.hash === hash ? saved.user : null;
      },
    };
  }
  root.ElonSocialChatCache = { create };
})(globalThis);
