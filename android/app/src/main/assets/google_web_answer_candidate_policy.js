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

  function sourceCollection(metrics) {
    const citations = nonNegative(metrics && metrics.citations);
    const links = nonNegative(metrics && metrics.links);
    const semanticBlocks = nonNegative(metrics && metrics.semanticBlocks);
    const narrativeBlocks = nonNegative(metrics && metrics.narrativeBlocks);
    const sourceResultItems = nonNegative(metrics && metrics.sourceResultItems);
    const textLength = nonNegative(metrics && metrics.textLength);
    const citationTextRatio = nonNegative(metrics && metrics.citationTextRatio);
    if (citations < 2 || links < 2) return false;
    // Google renders the right-hand source rail as several long list items. Those
    // snippets look narrative, but each item is a compact externally-linked result.
    if (sourceResultItems >= 2 && textLength <= Math.min(1200, sourceResultItems * 400)) {
      return true;
    }
    if (narrativeBlocks >= 2) return false;
    return (
      citationTextRatio >= 0.45 ||
      (semanticBlocks <= citations + 1 && textLength <= citations * 240)
    );
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

  function transientStatusText(value) {
    const text = String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/\s+/g, ' ')
      .trim()
      .toLowerCase();
    if (!text || text.length > 80) return false;
    return /^(?:ai\s*mode\s*)?(?:answer|response)\s+(?:is\s+)?ready[.!。！]?$/.test(text) ||
      /^(?:searching|loading|generating|preparing)(?:\s+(?:answer|response|results?))?[.!…]*$/.test(text) ||
      /^(?:ai\s*模式)?(?:回答|回复|响应)(?:已经|已)?准备就绪[。！!]?$/.test(text) ||
      /^(?:正在)?(?:生成|加载|准备)(?:回答|回复|响应)?(?:中)?[.。…]*$/.test(text) ||
      /^(?:正在)?(?:搜索|检索|查询)(?:中)?[.。…]*$/.test(text);
  }

  function shareSurfaceText(value) {
    const text = String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/\s+/g, ' ')
      .trim()
      .toLowerCase();
    if (!text || text.length > 800) return false;
    const shareHeading = /share (?:a )?public link|分享公开链接|公开链接用于分享/.test(text);
    const shareActions = /copy link|复制链接|facebook|reddit|whatsapp/.test(text);
    return shareHeading && shareActions;
  }

  function disclosureOnlyText(value) {
    const text = String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/\s+/g, ' ')
      .trim()
      .toLowerCase();
    if (!text || text.length > 40) return false;
    if (/^(?:show|view|expand|collapse)\s+(?:all|more|less)$/.test(text) ||
        /^(?:expand|collapse)$/.test(text)) return true;
    const compact = text.replace(/[\s,，、;；:：|｜·•/]+/g, '');
    return /^(?:(?:收起|展开)(?:全部)?|(?:全部)?显示|显示全部|隐藏全部){1,3}$/.test(compact);
  }

  function pageChromeText(value) {
    const text = String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/\s+/g, ' ')
      .trim()
      .toLowerCase();
    if (!text || text.length > 1600) return false;
    const signedOut = /您已退出(?:账号|帐号)|若要访问历史记录.*请登录|you(?:'re| are) signed out|sign in to (?:access|view).*history/.test(text);
    const chromePatterns = [
      /打开边栏|关闭边栏|open (?:the )?sidebar|close (?:the )?sidebar/,
      /新话题|新对话|new (?:chat|conversation|thread)/,
      /共享的公开链接|分享公开链接|shared public links?|public links?/,
      /查看我的.*历史记录|ai\s*模式历史记录|view my.*history|ai mode history/,
      /搜索消息串|search (?:chats?|threads?|conversations?)/,
      /管理\s*ai\s*模式|manage\s*ai\s*mode/
    ];
    const chromeSignals = chromePatterns.filter((pattern) => pattern.test(text)).length;
    return signedOut && chromeSignals >= 1 || chromeSignals >= 4;
  }

  function shortAnswerAllowed(metrics) {
    const trustedAnswerContainer = metrics && metrics.trustedAnswerContainer === true;
    if (!metrics || (metrics.afterQuery !== true && !trustedAnswerContainer) ||
        metrics.interactive === true || (metrics.liveRegion === true && !trustedAnswerContainer) ||
        nonNegative(metrics.links) !== 0 ||
        nonNegative(metrics.tabControls) !== 0) return false;
    const controls = nonNegative(metrics.controls);
    return controls === 0 || (metrics.explicit === true && controls <= 8);
  }

  function accepts(metrics) {
    const hasQuery = metrics && metrics.hasQuery === true;
    const textLength = nonNegative(metrics && metrics.textLength);
    const citations = nonNegative(metrics && metrics.citations);
    const semanticBlocks = nonNegative(metrics && metrics.semanticBlocks);
    const links = nonNegative(metrics && metrics.links);
    const tabControls = nonNegative(metrics && metrics.tabControls);
    const liveRegion = metrics && metrics.liveRegion === true;
    const explicit = metrics && metrics.explicit === true;
    if (!hasQuery || textLength < 1) return false;
    if (metrics && metrics.resultListItem === true) return false;
    if (sourceCollection(metrics)) return false;
    if (navigationOnlyText(metrics && metrics.text)) return false;
    if (transientStatusText(metrics && metrics.text)) return false;
    if (shareSurfaceText(metrics && metrics.text)) return false;
    if (disclosureOnlyText(metrics && metrics.text)) return false;
    if (pageChromeText(metrics && metrics.text)) return false;
    if (textLength < 8) return shortAnswerAllowed(metrics);
    if (liveRegion && semanticBlocks === 0 && citations === 0) return false;
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

  function select(candidates) {
    const values = Array.isArray(candidates) ? candidates.filter(Boolean) : [];
    // `accepts` normally removes source rails. Keep the final selector fail-closed
    // too, so a future scoring change cannot promote a known source collection.
    const eligible = values.filter((candidate) => candidate.sourceCollection !== true);
    const afterQuery = eligible.filter((candidate) => candidate.afterQuery === true);
    const trusted = eligible.filter((candidate) => candidate.trustedAnswerContainer === true);
    const pool = afterQuery.length ? afterQuery : (trusted.length ? trusted : eligible);
    // Google AI Mode commonly nests the complete answer above short trusted leaf nodes.
    // Prefer a multi-block narrative before using the numeric score so that link/control
    // penalties cannot reduce the full numbered response to its introduction or last line.
    const narrativePool = pool.filter((candidate) => nonNegative(candidate.narrativeBlocks) >= 2);
    const preferredPool = narrativePool.length ? narrativePool : pool;
    return preferredPool.sort((left, right) =>
      Number(right.score || 0) - Number(left.score || 0) ||
      nonNegative(right.textLength) - nonNegative(left.textLength) ||
      nonNegative(left.domOrder) - nonNegative(right.domOrder)
    )[0] || null;
  }

  return Object.freeze({
    version: 16,
    accepts,
    penalty,
    select,
    sourceCollection,
    navigationOnlyText,
    transientStatusText,
    shareSurfaceText,
    disclosureOnlyText,
    pageChromeText,
    shortAnswerAllowed
  });
});
