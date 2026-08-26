(function () {
  'use strict';

  if (window.__elonRichContentDomAdapter) return;
  if (!['https://chatgpt.com', 'https://google.com', 'https://www.google.com'].includes(location.origin)) return;

  const SCHEMA = 'yilong.rich-content.v1';
  const ROOT_ATTRIBUTE = 'data-elon-common-rich-content-root';
  const MAX_MEDIA_ITEMS = 8;

  function cleanText(value, max) {
    return String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/[\u200b-\u200f\u2060\ufeff]/g, '')
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, max || 240);
  }

  function safePublicUrl(value) {
    try {
      const raw = String(value || '').trim();
      if (!raw) return '';
      const url = new URL(raw, location.href);
      if (url.protocol !== 'https:' || url.username || url.password ||
          (url.port && url.port !== '443')) return '';
      // Official media commonly carries resize, cache-busting, or short-lived
      // signature parameters. Rejecting the whole URL made visible answer media
      // disappear from the native snapshot. Keep only the public origin/path;
      // Rust repeats the same sanitation before anything reaches React/cache.
      return (url.origin + url.pathname).slice(0, 1200);
    } catch (_) {
      return '';
    }
  }

  function safePublicMediaUrl(value) {
    try {
      const raw = String(value || '').trim();
      if (!raw) return '';
      const url = new URL(raw, location.href);
      // Provider image proxies often put the actual public asset in one of these
      // well-known parameters. Prefer that stable nested URL before dropping the
      // proxy query string; this keeps the picture without persisting its token.
      for (const key of ['url', 'image_url', 'imgurl', 'src']) {
        const nested = safePublicUrl(url.searchParams.get(key));
        if (nested) return nested;
      }
      return safePublicUrl(url.href);
    } catch (_) {
      return '';
    }
  }

  function visible(node, minimumWidth, minimumHeight) {
    if (!(node instanceof Element) || !node.isConnected) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width >= minimumWidth && rect.height >= minimumHeight &&
      style.display !== 'none' && style.visibility !== 'hidden' && Number(style.opacity) !== 0;
  }

  function imageDimensions(node) {
    const rect = node.getBoundingClientRect();
    return {
      width: Math.round(Number(node.naturalWidth || node.width || rect.width || 0)),
      height: Math.round(Number(node.naturalHeight || node.height || rect.height || 0))
    };
  }

  function mediaType(url) {
    const extension = String(url || '').toLowerCase().match(/\.([a-z0-9]{2,5})$/);
    if (!extension) return 'image/*';
    if (extension[1] === 'jpg' || extension[1] === 'jpeg') return 'image/jpeg';
    if (['png', 'gif', 'webp', 'avif', 'svg'].includes(extension[1])) return 'image/' + extension[1];
    return 'image/*';
  }

  function normalizeMediaGalleryPayload(value) {
    const source = value && typeof value === 'object' ? value : {};
    const seen = new Set();
    const items = (Array.isArray(source.items) ? source.items : []).slice(0, MAX_MEDIA_ITEMS)
      .map((item) => {
        const url = safePublicMediaUrl(item && item.url);
        const alt = cleanText(item && (item.alt || item.title), 180);
        const width = Math.max(0, Math.min(8192, Number(item && item.width) || 0));
        const height = Math.max(0, Math.min(8192, Number(item && item.height) || 0));
        const sourceUrl = safePublicUrl(item && item.sourceUrl);
        return {
          url,
          alt,
          mediaType: mediaType(url),
          ...(width > 0 ? { width } : {}),
          ...(height > 0 ? { height } : {}),
          ...(sourceUrl ? { sourceUrl } : {})
        };
      })
      .filter((item) => item.url && item.alt && !seen.has(item.url) && seen.add(item.url));
    return { title: cleanText(source.title, 120) || '回答图片', items };
  }

  function normalizeMapPayload(value) {
    const source = value && typeof value === 'object' ? value : {};
    const places = (Array.isArray(source.places) ? source.places : []).slice(0, 12)
      .map((place) => cleanText(place, 120)).filter(Boolean);
    return {
      title: cleanText(source.title, 120) || '地图结果',
      summary: cleanText(source.summary, 500),
      places
    };
  }

  function richPart(kind, payload, source) {
    return {
      type: 'rich_card',
      text: payload.title,
      kind,
      richContent: { schema: SCHEMA, kind, source: source || 'official_dom', payload }
    };
  }

  function mapPart(container) {
    const selector = [
      '[data-testid*="map" i]', '[aria-label*="map" i]', '[aria-label*="地图"]',
      '[aria-label*="地圖"]', 'iframe[src*="maps" i]'
    ].join(',');
    const root = Array.from(container.querySelectorAll(selector)).find((node) =>
      !node.closest('[' + ROOT_ATTRIBUTE + ']') && visible(node, 180, 100)
    );
    if (!root) return null;
    const textRoot = root.closest('figure, article, section') || root;
    const lines = String(textRoot.innerText || textRoot.getAttribute('aria-label') || '')
      .split(/\r?\n/).map((line) => cleanText(line, 160)).filter(Boolean).slice(0, 12);
    const payload = normalizeMapPayload({
      title: lines[0] || root.getAttribute('aria-label') || '地图结果',
      summary: lines.slice(1, 4).join(' · '),
      places: lines.slice(1)
    });
    root.setAttribute(ROOT_ATTRIBUTE, 'map');
    return richPart('map', payload, 'official_dom');
  }

  function mediaGalleryPart(container) {
    const items = [];
    const nodes = [];
    Array.from(container.querySelectorAll('img')).forEach((node) => {
      if (node.closest('[' + ROOT_ATTRIBUTE + '], [data-elon-rich-content-root]')) return;
      const dimensions = imageDimensions(node);
      if (!visible(node, 96, 72) || dimensions.width < 128 || dimensions.height < 96) return;
      const url = safePublicMediaUrl(node.currentSrc || node.src || node.getAttribute('src'));
      const caption = node.closest('figure')?.querySelector('figcaption');
      const labelledBy = cleanText(node.getAttribute('aria-labelledby'), 120);
      const labelledText = labelledBy
        ? cleanText(document.getElementById(labelledBy)?.textContent, 180)
        : '';
      const nearbyTitle = node.closest('figure, article, section')?.querySelector('h1, h2, h3, h4');
      const alt = cleanText(
        node.alt
          || node.getAttribute('aria-label')
          || labelledText
          || caption?.textContent
          || node.getAttribute('title')
          || nearbyTitle?.textContent
          || `回答图片 ${items.length + 1}`,
        180
      );
      if (!url || !alt) return;
      const anchor = node.closest('a[href]');
      items.push({
        url,
        alt,
        width: dimensions.width,
        height: dimensions.height,
        sourceUrl: safePublicUrl(anchor && anchor.href)
      });
      nodes.push(node);
    });
    const payload = normalizeMediaGalleryPayload({ title: '回答图片', items });
    if (!payload.items.length) return null;
    nodes.forEach((node) => node.setAttribute(ROOT_ATTRIBUTE, 'media_gallery'));
    return richPart('media_gallery', payload, 'official_dom');
  }

  function parts(container) {
    if (!(container instanceof Element)) return [];
    const result = [];
    const map = mapPart(container);
    if (map) result.push(map);
    const gallery = mediaGalleryPart(container);
    if (gallery) result.push(gallery);
    return result;
  }

  function owns(node) {
    return node instanceof Element && Boolean(node.closest('[' + ROOT_ATTRIBUTE + ']'));
  }

  function fromAuthorizedEnvelope(envelope, authorize) {
    if (!envelope || envelope.schema !== 'yilong.authorized-provider-response.v1' ||
        typeof authorize !== 'function') return [];
    return (Array.isArray(envelope.parts) ? envelope.parts : []).slice(0, 16).map((part) => {
      const kind = cleanText(part && part.kind, 48).toLowerCase();
      if (!authorize(envelope.providerId, envelope.authorizationId, kind)) return null;
      if (kind === 'media_gallery') {
        const payload = normalizeMediaGalleryPayload(part.payload);
        return payload.items.length ? richPart(kind, payload, 'private_response') : null;
      }
      if (kind === 'map') {
        const payload = normalizeMapPayload(part.payload);
        return payload.summary || payload.places.length ? richPart(kind, payload, 'private_response') : null;
      }
      return null;
    }).filter(Boolean);
  }

  window.__elonRichContentDomAdapter = Object.freeze({
    version: 1,
    schema: SCHEMA,
    parts,
    owns,
    normalizeMediaGalleryPayload,
    normalizeMapPayload,
    fromAuthorizedEnvelope
  });
})();
