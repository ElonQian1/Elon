(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root && (!root.__elonGoogleWebAnswerCandidatePolicy ||
      Number(root.__elonGoogleWebAnswerCandidatePolicy.version || 0) < api.version)) {
    root.__elonGoogleWebAnswerCandidatePolicy = Object.freeze(api);
  }
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  const navigationLabels = new Set([
    'ai mode', 'all', 'image', 'images', 'video', 'videos', 'news', 'map', 'maps',
    'shopping', 'book', 'books', 'flight', 'flights', 'finance', 'more',
    'ai模式', '全部', '图片', '视频', '新闻', '地图', '购物', '图书', '航班', '财经', '更多'
  ]);

  function nonNegative(value) {
    const number = Number(value);
    return Number.isFinite(number) ? Math.max(0, number) : 0;
  }

  function navigationOnlyText(value) {
    const text = String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/[｜|·•/]+/g, '\n')
      .trim();
    if (!text || text.length > 240) return false;
    const lines = text.split(/\r?\n/)
      .map((line) => line.trim().toLowerCase().replace(/\s+/g, ' '))
      .filter(Boolean);
    if (lines.length >= 4) {
      const matched = lines.filter((line) => navigationLabels.has(line)).length;
      if (matched >= 4 && matched / lines.length >= 0.7) return true;
    }
    const labels = [
      /\bai\s+mode\b/gi, /\ball\b/gi, /\bimages?\b/gi, /\bvideos?\b/gi,
      /\bnews\b/gi, /\bmaps?\b/gi, /\bshopping\b/gi, /\bbooks?\b/gi,
      /\bflights?\b/gi, /\bfinance\b/gi, /\bmore\b/gi,
      /ai\s*模式/gi, /全部/g, /图片/g, /视频/g, /新闻/g, /地图/g,
      /购物/g, /图书/g, /航班/g, /财经/g, /更多/g
    ];
    let remainder = text;
    let matched = 0;
    for (const pattern of labels) {
      remainder = remainder.replace(pattern, () => {
        matched += 1;
        return ' ';
      });
    }
    return matched >= 4 && !remainder.replace(/[\s,，、;；:：-]+/g, '');
  }

  function accepts(metrics) {
    const hasQuery = metrics && metrics.hasQuery === true;
    const textLength = nonNegative(metrics && metrics.textLength);
    const citations = nonNegative(metrics && metrics.citations);
    const semanticBlocks = nonNegative(metrics && metrics.semanticBlocks);
    const links = nonNegative(metrics && metrics.links);
    const tabControls = nonNegative(metrics && metrics.tabControls);
    const explicit = metrics && metrics.explicit === true;
    if (!hasQuery || textLength < 8) return false;
    if (navigationOnlyText(metrics && metrics.text)) return false;
    if (tabControls > 0 && semanticBlocks === 0 && citations === 0) return false;
    if (links >= 3 && semanticBlocks === 0 && citations === 0) return false;
    if (!explicit && semanticBlocks === 0 && citations === 0 && textLength < 80) return false;
    return true;
  }

  function penalty(metrics) {
    const links = nonNegative(metrics && metrics.links);
    const tabControls = nonNegative(metrics && metrics.tabControls);
    return Math.min(links, 20) * 180 + Math.min(tabControls, 10) * 600;
  }

  return Object.freeze({ version: 2, accepts, penalty, navigationOnlyText });
});
