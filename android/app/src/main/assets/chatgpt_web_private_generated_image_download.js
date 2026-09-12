(function (root, factory) {
  'use strict';
  const pointer = typeof module === 'object' && module.exports
    ? require('./chatgpt_web_private_image_pointer.js') : root?.__elonChatGptPrivateImagePointer;
  const api = factory(pointer);
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateGeneratedImageDownload = api;
})(typeof window === 'object' ? window : null, function (pointerParser) {
  'use strict';

  function target(source) {
    if (source?.generatedImageHistory !== true || source.attachmentsUnconfirmed ||
        !Array.isArray(source.attachments) || source.attachments.length ||
        source.image?.content_type !== 'image_asset_pointer') return null;
    const metadata = source.image.metadata;
    if (metadata != null && (typeof metadata !== 'object' || Array.isArray(metadata)) ||
        metadata?.is_no_auth_placeholder === true) return null;
    const base = pointerParser?.parse(source.image.asset_pointer);
    const watermark = metadata?.watermarked_asset_pointer == null ? null
      : pointerParser?.parse(metadata.watermarked_asset_pointer);
    if (!base || metadata?.watermarked_asset_pointer != null && !watermark) return null;
    return { projectId: null, imageConversationScope: true,
      generatedImageAssets: { base, watermark } };
  }

  function accountPolicy(runtime, shared) {
    try {
      if (runtime?.state?.().profile_id !== 'web_20260912' || runtime.peek?.('shared') !== shared) return null;
      // Current lo/Xt use shared.bK (canonical mq), a pure session-store getter.
      // It is not a React hook and does not need a rendered composer or image.
      const account = shared?.mq?.();
      const paid = account?.hasPaidSubscription?.();
      return typeof account?.id === 'string' && account.id && typeof paid === 'boolean'
        ? { id: account.id, paid } : null;
    } catch (_) { return null; }
  }

  async function prepare(root, job, current) {
    if (!current(job)) throw new Error('download_cancelled');
    const assets = job.entry.generatedImageAssets;
    if (!assets?.base) throw new Error('download_scope_unconfirmed');
    let pointer = assets.base, policyCurrent;
    if (assets.watermark) {
      const runtime = root.__elonChatGptPrivateRuntimeBindings;
      if (runtime?.state?.().profile_id !== 'web_20260912') throw new Error('download_scope_unconfirmed');
      const shared = runtime.peek?.('shared') || await runtime.load?.('shared');
      if (!current(job)) throw new Error('download_cancelled');
      const policy = accountPolicy(runtime, shared);
      if (!policy) throw new Error('download_scope_unconfirmed');
      pointer = policy.paid ? assets.base : assets.watermark;
      policyCurrent = () => {
        const live = accountPolicy(runtime, shared);
        return live?.id === policy.id && live.paid === policy.paid;
      };
    }
    job.entry = Object.freeze({ ...job.entry, fileId: pointer.id,
      downloadFileId: pointer.downloadFileId, downloadQuery: pointer.downloadQuery,
      generatedPolicyCurrent: policyCurrent });
    if (!current(job)) throw new Error('download_cancelled');
  }

  return Object.freeze({ version: 1, target, prepare });
});
