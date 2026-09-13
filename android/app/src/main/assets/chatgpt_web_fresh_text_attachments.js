(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, capture: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextAttachments = api;
})(typeof window === 'object' ? window : null, function (page, binding, runtime) {
  'use strict';
  const fail = () => { throw Error('attachments_active'); };
  const lease = page.__elonChatGptPrivateAttachmentSend?.prepareSubmit?.(binding.files);
  const files = lease?.readyFiles;
  if (!lease || typeof lease.current !== 'function' || typeof lease.consumeAccepted !== 'function' ||
      !lease.current() || !Array.isArray(files) || !files.length || files.length > 9 ||
      typeof runtime.textSerializeAttachments !== 'function') return fail();
  const idPattern = /^[A-Za-z0-9_-]{1,160}$/;
  const pointer = id => (id.startsWith('file_') ? 'sediment://' : 'file-service://') + id;
  if (new Set(files.map(file => file.fileId)).size !== files.length || files.some(file => {
    const spec = file.fileSpec;
    return file.status !== 'ready' || !['local', 'library'].includes(file.source) ||
      !idPattern.test(file.fileId || '') || spec?.id !== file.fileId ||
      typeof spec.name !== 'string' || !spec.name.trim() || spec.name.length > 512 ||
      !Number.isSafeInteger(spec.size) || spec.size < 1 ||
      typeof spec.mimeType !== 'string' || !/^[a-z0-9.+-]+\/[a-z0-9.+-]+$/i.test(spec.mimeType) ||
      file.sharedLibraryFileReference != null || file.mountedLibraryFileId != null ||
      spec.contextConnectorInfo != null;
  })) return fail();
  const mimeTypes = Object.freeze([...new Set(files.map(file => file.fileSpec.mimeType.toLowerCase()))]);
  const ids = new Set(files.map(file => file.fileId));
  const images = new Map(files.filter(file => 'width' in file.fileSpec && 'height' in file.fileSpec)
    .map(file => [pointer(file.fileId), file.fileSpec]));
  let prepared = null;
  function freeze(value) {
    if (value && typeof value === 'object') {
      Object.values(value).forEach(freeze);
      Object.freeze(value);
    }
    return value;
  }
  function copy(value) {
    const json = JSON.stringify(value);
    if (!json || json.length > 128 * 1024) return fail();
    return JSON.parse(json);
  }
  function current() { try { return lease.current() === true; } catch (_) { return false; } }
  function mode(context) {
    return context.projectId == null ? { kind: 'primary_assistant' }
      : { kind: 'gizmo_interaction', gizmo_id: context.projectId };
  }
  function serialize(prompt, context) {
    if (!current()) throw Error('context_changed');
    // Reviewed Ypt/nhr is the ready-file serializer, not submitComposer. It
    // reads model capabilities and returns content/metadata without dispatch.
    const raw = runtime.textSerializeAttachments(files, prompt, context.model, mode(context), context.tool ?? null);
    if (!raw || !Array.isArray(raw.attachments) || raw.attachments.length > files.length) return fail();
    const result = copy({ content: raw.content, attachments: raw.attachments });
    const represented = new Set();
    for (const item of result.attachments) {
      const file = files.find(file => file.fileId === item?.id), spec = file?.fileSpec;
      if (!spec || represented.has(item.id) || item.name !== spec.name || item.size !== spec.size ||
          item.mime_type !== spec.mimeType || item.library_file_id !== file.libraryFileId ||
          item.source !== file.source) return fail();
      represented.add(item.id);
    }
    if (typeof result.content === 'string') {
      if (result.content !== prompt) return fail();
    } else {
      const content = result.content, parts = content?.parts, seen = new Set();
      if (content?.content_type !== 'multimodal_text' || !Array.isArray(parts) || parts.length < 2 ||
          parts.length > images.size + 1 || parts.at(-1) !== prompt) return fail();
      for (const part of parts.slice(0, -1)) {
        const spec = images.get(part?.asset_pointer);
        if (!spec || seen.has(spec.id) || part.content_type !== 'image_asset_pointer' ||
            part.width !== spec.width || part.height !== spec.height || part.size_bytes !== spec.size) return fail();
        seen.add(spec.id); represented.add(spec.id);
      }
    }
    if (represented.size !== ids.size || [...ids].some(id => !represented.has(id)) || !current()) return fail();
    return freeze(result);
  }
  function prepare(context) {
    const value = serialize('', context);
    prepared = { model: context.model, projectId: context.projectId ?? null, tool: context.tool ?? null, value };
  }
  function message(prompt, context) {
    if (!prepared || prepared.model !== context.model || prepared.projectId !== (context.projectId ?? null) ||
        prepared.tool !== (context.tool ?? null)) throw Error('context_changed');
    const value = serialize(prompt, context), check = copy(value);
    if (typeof check.content === 'string') check.content = '';
    else check.content.parts[check.content.parts.length - 1] = '';
    if (JSON.stringify(check) !== JSON.stringify(prepared.value)) throw Error('context_changed');
    return { content: typeof value.content === 'string' ? { content_type: 'text', parts: [value.content] } : copy(value.content),
      metadata: value.attachments.length ? { attachments: copy(value.attachments) } : {} };
  }
  function matchesHistory(message) {
    if (!prepared) return false;
    const metadata = message?.metadata?.attachments ?? [], parts = message?.content?.parts ?? [];
    if (!Array.isArray(metadata) || !Array.isArray(parts)) return false;
    return files.every(file => metadata.some(item => item?.id === file.fileId) ||
      images.has(pointer(file.fileId)) && parts.some(part => part?.content_type === 'image_asset_pointer' &&
        part.asset_pointer === pointer(file.fileId)));
  }
  return Object.freeze({ mimeTypes, current, prepare, message, matchesHistory,
    consumeAccepted: () => lease.consumeAccepted() === true });
});
