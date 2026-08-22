(function () {
  'use strict';

  const base = window.__elonGoogleWebRichContent;
  const common = window.__elonRichContentDomAdapter;
  if (!base || !common || window.__elonWinGoogleRichContentInstalled) return;
  window.__elonWinGoogleRichContentInstalled = true;

  function parts(container, fallbackText, query) {
    const prose = typeof base.parts === 'function' ? base.parts(container, fallbackText, query) : [];
    const rich = common.parts(container);
    return prose.concat(rich).slice(0, 16);
  }

  function owns(node) {
    return Boolean(
      (typeof base.owns === 'function' && base.owns(node)) ||
      common.owns(node)
    );
  }

  function cleanText(value, max) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/[\u200b-\u200f\u2060\ufeff]/g, '')
      .replace(/\s+/g, ' ').trim().slice(0, max || 160);
  }

  function normalizeWeatherPayload(value) {
    const source = value && typeof value === 'object' ? value : {};
    const rows = (Array.isArray(source.rows) ? source.rows : []).slice(0, 24).map((row) => ({
      period: cleanText(row && row.period, 48),
      condition: cleanText(row && row.condition, 64),
      temperature: cleanText(row && row.temperature, 32),
      precipitation: cleanText(row && row.precipitation, 32),
      wind: cleanText(row && row.wind, 48)
    })).filter((row) => row.period && row.condition && row.temperature);
    return {
      title: cleanText(source.title, 120) || '天气预报',
      summary: cleanText(source.summary, 240),
      rows
    };
  }

  function fromAuthorizedEnvelope(envelope, authorize) {
    if (!envelope || envelope.schema !== 'yilong.authorized-provider-response.v1' ||
        envelope.providerId !== 'google-ai-mode' || typeof authorize !== 'function') return [];
    const commonParts = common.fromAuthorizedEnvelope(envelope, authorize);
    const weather = (Array.isArray(envelope.parts) ? envelope.parts : []).slice(0, 16)
      .filter((part) => part && cleanText(part.kind, 48).toLowerCase() === 'weather')
      .filter(() => authorize(envelope.providerId, envelope.authorizationId, 'weather'))
      .map((part) => {
        const payload = normalizeWeatherPayload(part.payload);
        return payload.rows.length ? {
          type: 'rich_card',
          text: payload.title,
          kind: 'weather',
          richContent: {
            schema: 'yilong.rich-content.v1',
            kind: 'weather',
            source: 'private_response',
            payload
          }
        } : null;
      }).filter(Boolean);
    return weather.concat(commonParts).slice(0, 16);
  }

  window.__elonGoogleWebRichContent = Object.freeze(Object.assign({}, base, {
    version: Number(base.version || 0) + 100,
    parts,
    owns,
    normalizeWeatherPayload,
    fromAuthorizedEnvelope
  }));
})();
