(function (root, factory) {
  'use strict';
  const exported = factory();
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateLibraryRasterPolicy = exported;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const special = new Set(['saved_entity', 'flashcards', 'deep_research_report', 'learning_quiz',
    'app_block', 'site_preview', 'writing_block', 'chat_message_snapshot']);
  const scoped = ['gizmo_id', 'project_id', 'context_scopes', 'preview_file', 'mounted_library_file_id',
    'library_file_id', 'shared_library_file_id', 'library_download_id', 'context_connector_info', 'library_provider'];

  // A type marker alone does not change a raster file into an unsupported document.
  // Each operation still validates its catalog identity, access and size constraints.
  function matches(file) {
    return typeof file?.library_artifact_type === 'string' &&
      /^(?:[a-z][a-z0-9_]{0,63})?$/.test(file.library_artifact_type) && !special.has(file.library_artifact_type) &&
      typeof file.name === 'string' && !/\.flashcards$/i.test(file.name.trimEnd()) &&
      /^file[_-][A-Za-z0-9_-]{1,152}$/.test(file.file_id || '') &&
      ['image/png', 'image/jpeg', 'image/webp'].includes(file.mime_type) &&
      (file.is_project == null || file.is_project === false) && scoped.every(key => file[key] == null);
  }
  return Object.freeze({ version: 1, matches });
});
