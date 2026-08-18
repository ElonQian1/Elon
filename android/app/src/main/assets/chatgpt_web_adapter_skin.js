(function (root) {
  'use strict';

  if (root.__elonChatGptSkin) return;

  const STYLE_ID = 'elon-chatgpt-web-skin-v1';
  const ROOT_ATTRIBUTE = 'data-elon-chatgpt-skin';
  const STYLE_TEXT = [
    'html[' + ROOT_ATTRIBUTE + '="true"] body { overflow-x: hidden !important; }',
    'html[' + ROOT_ATTRIBUTE + '="true"] [data-testid="sidebar"],',
    'html[' + ROOT_ATTRIBUTE + '="true"] [data-testid="mobile-sidebar"],',
    'html[' + ROOT_ATTRIBUTE + '="true"] nav[aria-label="Chat history"],',
    'html[' + ROOT_ATTRIBUTE + '="true"] nav[aria-label="聊天记录"] { display: none !important; }',
    'html[' + ROOT_ATTRIBUTE + '="true"] [data-testid="mobile-header"],',
    'html[' + ROOT_ATTRIBUTE + '="true"] [data-testid="conversation-header"] { display: none !important; }',
    'html[' + ROOT_ATTRIBUTE + '="true"] main { width: 100% !important; max-width: none !important; }',
    'html[' + ROOT_ATTRIBUTE + '="true"] main > div { max-width: none; }'
  ].join('\n');

  function styleNode(documentRef) {
    let node = documentRef.getElementById(STYLE_ID);
    if (node) return node;
    node = documentRef.createElement('style');
    node.id = STYLE_ID;
    node.type = 'text/css';
    node.textContent = STYLE_TEXT;
    (documentRef.head || documentRef.documentElement).appendChild(node);
    return node;
  }

  function setEnabled(enabled, environment) {
    const env = environment || {};
    const documentRef = env.document || root.document;
    const origin = String(env.origin || root.location.origin || '');
    if (origin !== 'https://chatgpt.com') return { ok: false, reason: 'unsupported_origin' };
    if (!documentRef || !documentRef.documentElement) {
      return { ok: false, reason: 'document_not_ready' };
    }
    if (enabled) {
      styleNode(documentRef);
      documentRef.documentElement.setAttribute(ROOT_ATTRIBUTE, 'true');
    } else {
      documentRef.documentElement.removeAttribute(ROOT_ATTRIBUTE);
      const node = documentRef.getElementById(STYLE_ID);
      if (node && node.parentNode) node.parentNode.removeChild(node);
    }
    return { ok: true, enabled: enabled === true };
  }

  root.__elonChatGptSkin = Object.freeze({
    setEnabled,
    styleId: STYLE_ID,
    rootAttribute: ROOT_ATTRIBUTE
  });
})(window);
