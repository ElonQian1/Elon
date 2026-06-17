(function () {
  const TOKEN_KEYS = ['lodex_token', 'elon_token'];

  function readToken() {
    for (const key of TOKEN_KEYS) {
      const value = localStorage.getItem(key);
      if (value) return value;
    }
    return '';
  }

  function safeNodeAdminUrl() {
    const params = new URLSearchParams(location.search);
    const raw = params.get('node_admin') || 'http://127.0.0.1:7799/';
    try {
      const url = new URL(raw);
      const host = url.hostname.toLowerCase();
      if ((host === '127.0.0.1' || host === 'localhost') && /^https?:$/.test(url.protocol)) {
        return url.toString();
      }
    } catch (_) {}
    return 'http://127.0.0.1:7799/';
  }

  function escapeHtml(value) {
    return String(value == null ? '' : value)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  function clean(value) {
    return String(value == null ? '' : value).trim();
  }

  function firstChar(value, fallback) {
    return Array.from(clean(value) || fallback || '龙')[0] || '龙';
  }

  function formatTime(value) {
    if (!value) return '';
    const date = new Date(Number(value) || value);
    if (Number.isNaN(date.getTime())) return String(value).slice(0, 16);
    return date.toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    });
  }

  window.ElonPcKit = {
    TOKEN_KEYS,
    readToken,
    safeNodeAdminUrl,
    escapeHtml,
    clean,
    firstChar,
    formatTime
  };
})();
