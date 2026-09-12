(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateMessageImage = factory(root);
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const RUNTIME = 'https://chatgpt.com/cdn/assets/8b34dbc2-fqgb3eqijpn96umi.js';
  const PATH = /^(?:\/g\/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?)?\/c\/([A-Za-z0-9_-]{1,160})$/i;
  const ownerPath = page.__elonChatGptCommittedOwnerPath ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_committed_owner_path.js') : null);

  function capture(node) {
    if (!node?.isConnected || !(node instanceof page.HTMLImageElement) ||
        !node.closest('[data-message-author-role="user"], [data-message-author-role="assistant"]')) return null;
    if (!(page.__elonChatGptPrivateRuntimeBindings?.observed(RUNTIME) ||
        page.performance?.getEntriesByName?.(RUNTIME, 'resource')?.length > 0)) return null;
    const path = page.location.pathname, match = PATH.exec(path);
    const keys = Object.keys(node).filter(key => key.startsWith('__reactFiber$'));
    if (!match || keys.length !== 1) return null;
    const chain = ownerPath?.resolve(node[keys[0]])?.ancestors || [];
    // Den's observed download callback pairs asset with conversation/library
    // scope. A nearby thumbnail URL or another image's owner is not a descriptor.
    const owners = chain.filter(fiber => typeof fiber.type === 'function' && fiber.type.name === 'Den');
    if (owners.length !== 1) return null;
    const props = owners[0].memoizedProps, asset = props?.asset;
    const pointer = page.__elonChatGptPrivateImagePointer?.parse(asset?.asset_pointer);
    if (!pointer || props.isOptimisticPlaceholder === true ||
        props.checkContextScopesForConversationId !== match[1] ||
        asset.content_type !== 'image_asset_pointer' ||
        props.libraryFileId != null && !/^libfile[_-][A-Za-z0-9_-]{1,152}$/.test(props.libraryFileId)) return null;
    if (['gizmo_id', 'project_id', 'library_file_id', 'shared_library_file_id', 'library_download_id',
      'context_scopes', 'source_url', 'context_connector', 'connector_id', 'context_connector_info']
      .some(key => asset[key] != null)) return null;
    const src = node.currentSrc || node.getAttribute('src') || '';
    if (!src || new URL(node.getAttribute('src'), page.location.origin).href !==
        new URL(src, page.location.origin).href) return null;
    const name = props.downloadFileName ?? props.imageName ?? 'image.png';
    if (typeof name !== 'string' || !name.trim() || name.length > 180 || /[\x00-\x1f\x7f]/.test(name)) return null;
    const source = {
      image: { content_type: 'image_asset_pointer', asset_pointer: asset.asset_pointer },
      attachments: [{ id: pointer.id, name: name.trim(),
        ...(props.libraryFileId ? { library_file_id: props.libraryFileId } : {}) }],
    };
    return { path, source, signature: JSON.stringify([path, source]), token: page.__elonChatGptDocumentToken };
  }

  function describe(node) {
    try {
      const binding = capture(node);
      if (!binding) return null;
      const current = () => {
        try {
          const now = capture(node);
          return !!now && now.token === binding.token && now.signature === binding.signature;
        } catch (_) { return false; }
      };
      return page.__elonChatGptPrivateFileDownload?.registerMessageImage?.(
        binding.path, binding.source, node, current) || null;
    } catch (_) { return null; }
  }

  return Object.freeze({ version: 1, describe });
});
