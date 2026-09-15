(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, signature: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextAttachmentIdentity = api;
})(typeof window === 'object' ? window : null, function (message) {
  'use strict';
  const fail = () => { throw Error('recovery_identity_unavailable'); };
  const id = value => typeof value === 'string' && /^[A-Za-z0-9_-]{1,160}$/.test(value);
  const attachments = message?.metadata?.attachments ?? [], content = message?.content;
  if (!Array.isArray(attachments) || attachments.length > 9 ||
      !['text', 'multimodal_text'].includes(content?.content_type) ||
      !Array.isArray(content.parts) || content.parts.length > 10) return fail();
  const references = [], files = new Set(), images = new Set();
  for (const item of attachments) {
    if (!id(item?.id) || files.has(item.id)) return fail();
    const mounted = item.mounted_library_file_id ?? null;
    const mime = item.mounted_library_mime_type ?? null;
    if (mounted !== null && (!id(mounted) || typeof mime !== 'string' || mime.length > 160 ||
        !/^[a-z0-9.+-]+\/[a-z0-9.+-]+$/i.test(mime)) || mounted === null && mime !== null) return fail();
    files.add(item.id);
    references.push(['file', item.id, mounted, mime]);
  }
  for (const part of content.parts) {
    if (typeof part === 'string') continue;
    if (content.content_type !== 'multimodal_text' || part?.content_type !== 'image_asset_pointer' ||
        typeof part.asset_pointer !== 'string' ||
        !/^(sediment|file-service):\/\/[A-Za-z0-9_-]{1,160}$/.test(part.asset_pointer) ||
        images.has(part.asset_pointer)) return fail();
    images.add(part.asset_pointer);
    references.push(['image', part.asset_pointer]);
  }
  if (images.size > 9 || references.length > 18) return fail();
  // Match the reviewed file IDs and image pointers, not server-added metadata,
  // attachment titles, prompt text or object-property/attachment ordering.
  return references.length ? JSON.stringify(references.sort((a, b) => {
    const left = JSON.stringify(a), right = JSON.stringify(b);
    return left < right ? -1 : left > right ? 1 : 0;
  })) : null;
});
