(function (page) {
  'use strict';
  if (page?.location?.origin !== 'https://chatgpt.com') return;
  const CHUNK = 24576, MAX_CACHE = 32 * 1024 * 1024;
  async function read(saved, revision, index, offset, identity) {
    const asset = saved.assets?.get(index);
    if (!asset || !Number.isSafeInteger(offset) || offset < 0) throw new Error('invalid_asset_cursor');
    const check = async () => {
      if ((await identity()).owner !== saved.owner) throw new Error('identity_changed');
      if (saved.expires < Date.now()) throw new Error('cursor_expired');
    };
    await check();
    if (!saved.assetBytes) { saved.assetBytes = new Map(); saved.assetByteCount = 0; }
    let value = saved.assetBytes.get(index);
    if (!value) {
      const download = page.__elonChatGptPrivateFileDownload;
      if (!download?.readSource) throw new Error('reader_update_required');
      value = await download.readSource('/c/' + saved.snapshot.conversation_id,
        { ...asset.source, name: asset.name }, asset.scope);
      await check();
      if (!(value.bytes instanceof Uint8Array) || value.bytes.length > 8 * 1024 * 1024) {
        throw new Error('download_file_too_large');
      }
      if (saved.assetByteCount + value.bytes.length > MAX_CACHE) throw new Error('asset_cache_limit');
      saved.assetBytes.set(index, value);
      saved.assetByteCount += value.bytes.length;
    }
    if (offset > value.bytes.length || offset % CHUNK !== 0) throw new Error('invalid_asset_cursor');
    const bytes = value.bytes.subarray(offset, offset + CHUNK), next = offset + bytes.length;
    saved.expires = Date.now() + 180000;
    return { schema: 'yilong.web-conversation.asset.v1', conversation_id: saved.snapshot.conversation_id,
      revision, asset_index: index, byte_offset: offset, total_bytes: value.bytes.length,
      media_type: value.mediaType, name: value.name, data: page.btoa(String.fromCharCode(...bytes)),
      has_more: next < value.bytes.length,
      next_cursor: next < value.bytes.length ? `asset.${revision}.${index}.${next}` : null };
  }
  page.__elonConversationAssets = Object.freeze({ version: 1, read });
})(typeof window === 'object' ? window : null);
