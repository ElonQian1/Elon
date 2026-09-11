'use strict';
const { fixture, CID, ID, PATH, LIST, SAVE, doc } = require('./chatgpt-web-private-canvas-documents.cjs');
const sharing = require('../../android/app/src/main/assets/chatgpt_web_private_canvas_document_sharing.js');
const content = require('../../android/app/src/main/assets/chatgpt_web_private_canvas_content.js');
const SHARE = SAVE + '/share', RESTORE = SAVE + '/restore', HISTORY = SAVE + '/history?before_version=';

function management() {
  const f = fixture(), versions = [doc({ version: 3, content: 'Earlier' }), doc({ version: 2, content: 'First' }), doc({ version: 1, content: '' })];
  let shared = null, pageSize = 2;
  f.page.__elonChatGptPrivateCanvasDocumentSharing = sharing;
  f.page.__elonChatGptPrivateCanvasContent = content;
  const published = () => ({ shared_textdoc_id: 'synthetic_shared_canvas', name: f.rows[0].title,
    textdoc_type: f.rows[0].textdoc_type, content: f.rows[0].content, version: f.rows[0].version,
    access: 'public', is_moderation_blocked: false, is_anonify_api_key_detected: false });
  function respond({ url, init }) {
    if (url.startsWith(HISTORY) && init.method === 'GET') return { payload: { previous_doc_states:
      structuredClone(versions.filter(value => value.version < Number(url.slice(HISTORY.length))).slice(0, pageSize)) } };
    if (url === RESTORE && init.method === 'POST') {
      const body = JSON.parse(init.body), target = versions.find(value => value.version === body.restore_from_version);
      if (body.version !== f.rows[0].version || !target) throw Error('http_409');
      f.rows[0] = { ...structuredClone(target), version: body.version + 1 };
      return { payload: { version: f.rows[0].version } };
    }
    if (url === SHARE) {
      if (init.method === 'POST') shared = published();
      return { payload: { shared_textdoc: structuredClone(shared) } };
    }
  }
  f.setHook(respond);
  const selection = (value, operation) => ({ operation, ticket: value.ticket, scope: value.scope, id: ID });
  return { ...f, versions, published, respond, selection,
    history: (value, before = 4) => f.run({ ...selection(value, 'history'), beforeVersion: before }),
    restore: (value, target = value.history.versions[0].documentVersion, confirmed = true) => f.run({
      ...selection(value, 'restore'), historyTicket: value.history.ticket, restoreVersion: target }, confirmed),
    share: (value, operation = 'share_lookup', confirmed = false) => f.run(selection(value, operation), confirmed),
    setShared: value => { shared = value; }, setPageSize: value => { pageSize = value; },
    writes: () => f.requests.filter(value => value.init.method === 'POST') };
}
module.exports = { management, CID, ID, PATH, LIST, SAVE, SHARE, RESTORE, HISTORY, doc };
