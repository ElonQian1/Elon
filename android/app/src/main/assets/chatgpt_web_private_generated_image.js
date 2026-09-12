(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateGeneratedImage = factory(root);
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const RUNTIMES = ['c1811d6e-jnjixufzpxcwv4zx.js', 'ecab41d6-l7yjx7xz4auxotb7.js'];
  const PATH = /^(?:\/g\/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?)?\/c\/([A-Za-z0-9_-]{1,160})$/i;
  const RESTRICTED = ['gizmo_id', 'project_id', 'library_file_id', 'shared_library_file_id',
    'library_download_id', 'context_scopes', 'source_url', 'context_connector', 'connector_id',
    'context_connector_info'];

  function named(fiber, name) {
    return typeof fiber?.type === 'function' && fiber.type.name === name;
  }

  function children(owner) {
    const queue = owner.child ? [owner.child] : [], seen = new Set(), found = [];
    while (queue.length) {
      const fiber = queue.pop();
      if (!fiber || seen.has(fiber) || seen.size >= 384) return null;
      seen.add(fiber);
      // Stay inside this image, never borrow actions from a nested image/portal.
      if (named(fiber, 'lo') || fiber.tag === 4) return null;
      if (named(fiber, 'Xt') || named(fiber, 'qa')) found.push(fiber);
      if (fiber.sibling) queue.push(fiber.sibling);
      if (fiber.child) queue.push(fiber.child);
    }
    return found;
  }

  function capture(node, path, ancestors) {
    const match = PATH.exec(path || '');
    if (!match || !RUNTIMES.every(file => {
      const url = 'https://chatgpt.com/cdn/assets/' + file;
      return page.__elonChatGptPrivateRuntimeBindings?.observed(url) ||
        page.performance?.getEntriesByName?.(url, 'resource')?.length > 0;
    })) return null;
    const owners = ancestors.filter(fiber => named(fiber, 'lo'));
    if (owners.length !== 1) return null;
    const owner = owners[0], props = owner.memoizedProps, asset = props?.pointer;
    if (!props || props.isPreview === true || props.compact === true ||
        typeof props.messageId !== 'string' || !/^[A-Za-z0-9_-]{1,160}$/.test(props.messageId) ||
        typeof props.conversation?.serverId$ !== 'function' || props.conversation.serverId$() !== match[1] ||
        asset?.content_type !== 'image_asset_pointer' || asset.metadata?.is_no_auth_placeholder === true ||
        !page.__elonChatGptPrivateImagePointer?.parse(asset.asset_pointer) ||
        RESTRICTED.some(key => asset[key] != null)) return null;
    const candidates = children(owner);
    if (!candidates) return null;
    const actions = candidates.filter(fiber => named(fiber, 'Xt'));
    const renderers = candidates.filter(fiber => named(fiber, 'qa'));
    if (actions.length !== 1 || renderers.length !== 1) return null;
    const action = actions[0].memoizedProps, renderer = renderers[0].memoizedProps;
    // The inline overlay is committed only after the final image has rendered.
    // Its explicit account policy chooses the same default watermarked download.
    if (action?.conversation !== props.conversation || action.messageId !== props.messageId ||
        action.fullSizeImageAsset !== asset || typeof action.hasWatermarkedDownload !== 'boolean' ||
        typeof action.isRenderedImageWatermarked !== 'boolean' ||
        typeof action.imageUrl !== 'string' || !action.imageUrl ||
        action.imageAssetPointer !== renderer?.src ||
        renderer.datadogImageContext?.conversation_id !== match[1] ||
        renderer.datadogImageContext?.message_id !== props.messageId) return null;
    const src = node.currentSrc || node.getAttribute('src') || '';
    const absolute = value => typeof value === 'string' && value
      ? new URL(value, page.location.origin).href : '';
    if (!src || absolute(node.getAttribute('src')) !== absolute(src) ||
        ![renderer.src, action.imageUrl].some(value => absolute(value) === absolute(src))) return null;
    const watermark = asset.metadata?.watermarked_asset_pointer;
    if (action.isRenderedImageWatermarked && !action.hasWatermarkedDownload) return null;
    const assetPointer = action.hasWatermarkedDownload ? watermark : asset.asset_pointer;
    if (!page.__elonChatGptPrivateImagePointer?.parse(assetPointer)) return null;
    const source = { image: { content_type: 'image_asset_pointer', asset_pointer: assetPointer },
      attachments: [], generatedImageConversationId: match[1] };
    return { path, source,
      signature: JSON.stringify([path, props.messageId, asset.asset_pointer,
        action.hasWatermarkedDownload, action.isRenderedImageWatermarked, source]),
      token: page.__elonChatGptDocumentToken };
  }

  return Object.freeze({ version: 1, capture });
});
