(function () {
  'use strict';

  if (window.__elonChatGptImageGallerySync || location.origin !== 'https://chatgpt.com') return;
  const nativeBridge = window.elonChatGptImageGalleryNative;
  const imageAssets = window.__elonChatGptImageAssets;
  const cachedHandles = new Set(
    Array.isArray(window.__elonChatGptCachedImageHandles)
      ? window.__elonChatGptCachedImageHandles.filter((value) => /^image_[a-f0-9]{16}$/.test(value))
      : []
  );
  const adapterVersion = Number(window.__elonChatGptAdapterTargetVersion || 0);
  if (!nativeBridge || typeof nativeBridge.postMessage !== 'function' ||
      !imageAssets || typeof imageAssets.scan !== 'function' ||
      typeof imageAssets.request !== 'function' || adapterVersion <= 0) return;

  let sequence = 0;
  function emitEvent(event) {
    nativeBridge.postMessage(JSON.stringify({
      schema: 'yilong.ai.ui.v1',
      adapterVersion,
      providerId: 'chatgpt',
      source: 'official_web_image_gallery',
      sequence: ++sequence,
      event
    }));
  }

  function delay(milliseconds) {
    return new Promise((resolve) => window.setTimeout(resolve, milliseconds));
  }

  function scroller() {
    const candidates = Array.from(document.querySelectorAll(
      'main, [data-radix-scroll-area-viewport], [class*="overflow-y-auto"]'
    ));
    return candidates.find((node) => node.scrollHeight > node.clientHeight + 80) || document.scrollingElement;
  }

  async function collect() {
    const handles = new Set();
    let unchanged = 0;
    for (let step = 0; step < 18; step += 1) {
      imageAssets.scan(document).forEach((item) => handles.add(item.assetHandle));
      const before = handles.size;
      const target = scroller();
      if (target) target.scrollTo({ top: target.scrollHeight, behavior: 'auto' });
      await delay(handles.size === 0 ? 650 : (step === 0 ? 900 : 360));
      imageAssets.scan(document).forEach((item) => handles.add(item.assetHandle));
      unchanged = handles.size === before ? unchanged + 1 : 0;
      if ((handles.size > 0 && unchanged >= 3) || handles.size >= 80) break;
    }
    return Array.from(handles).slice(0, 80);
  }

  async function run() {
    emitEvent({ type: 'image_gallery_snapshot', state: 'loading', observedCount: 0 });
    try {
      await delay(500);
      const handles = await collect();
      const pending = handles.filter((handle) => !cachedHandles.has(handle)).slice(0, 24);
      let exportedCount = 0;
      for (const handle of pending) {
        const result = await imageAssets.request(handle, emitEvent);
        if (result && result.ok) exportedCount += 1;
      }
      if (pending.length > 0 && exportedCount === 0) throw new Error('asset_export_failed');
      emitEvent({
        type: 'image_gallery_snapshot',
        state: 'ready',
        observedCount: handles.length
      });
    } catch (_) {
      emitEvent({ type: 'image_gallery_snapshot', state: 'failed', observedCount: 0 });
    }
  }

  window.__elonChatGptImageGallerySync = Object.freeze({ run });
  run();
})();
