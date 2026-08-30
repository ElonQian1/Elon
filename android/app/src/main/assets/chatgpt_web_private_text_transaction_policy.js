(function () {
  'use strict';

  const existing = window.__elonChatGptPrivateTextTransactionPolicy;
  if (existing && Number(existing.version) >= 11) return;

  const MAX_PROMPT_LENGTH = 20000;
  const MAX_MESSAGES = 8;
  const MAX_TEMPLATE_AGE_MS = 10 * 60 * 1000;
  const SAFE_ID = /^[A-Za-z0-9_-]{8,180}$/;
  const REQUEST_ID = /^mcp_[a-z0-9]{1,32}$/;
  const CONVERSATION_PATH = /^\/c\/([A-Za-z0-9_-]{8,180})$/;
  const GIZMO_CONVERSATION_PATH =
    /^\/g\/(g-p-[A-Za-z0-9_-]{4,180})\/c\/([A-Za-z0-9_-]{8,180})$/;
  const SAFE_PATH_SEGMENT = /^[A-Za-z0-9_:@-]{1,180}$/;
  const BLOCKED_PATH_ROOT = /^(?:auth|cdn-cgi|share)$/;
  const NON_TEXT_PAYLOAD_KEY = /(?:attachment|file|upload|asset|image|media|content_reference)/i;
  const TEXT_SEND_ACTIONS = new Set(['next', 'continue']);

  function cloneJson(value) {
    try { return JSON.parse(JSON.stringify(value)); }
    catch (_) { return null; }
  }

  function cleanPath(value) {
    const raw = String(value || '').split(/[?#]/, 1)[0];
    if (!raw.startsWith('/') || raw.length > 240) return '';
    const segments = raw.split('/').filter(Boolean);
    if (segments.length > 8 || segments.some((segment) => !SAFE_PATH_SEGMENT.test(segment)) ||
        (segments[0] && BLOCKED_PATH_ROOT.test(segments[0]))) return '';
    return segments.length ? '/' + segments.join('/') : '/';
  }

  function pathRejectionCode(value) {
    const raw = String(value || '').split(/[?#]/, 1)[0];
    if (!raw) return 'empty';
    if (!raw.startsWith('/')) return 'not_absolute';
    if (raw.length > 240) return 'too_long';
    const segments = raw.split('/').filter(Boolean);
    if (segments.length > 8) return 'too_many_segments';
    if (segments[0] && BLOCKED_PATH_ROOT.test(segments[0])) return 'blocked';
    if (segments.some((segment) => segment.length > 180)) return 'long_segment';
    const invalid = segments.find((segment) => !SAFE_PATH_SEGMENT.test(segment));
    if (!invalid) return 'unknown';
    if (invalid.includes('%')) return 'percent_encoded';
    if (invalid.includes('.')) return 'dot';
    if (/[^\x20-\x7E]/.test(invalid)) return 'unicode';
    return 'special_character';
  }

  function pathConversationId(path) {
    const direct = CONVERSATION_PATH.exec(path);
    if (direct) return direct[1];
    const gizmo = GIZMO_CONVERSATION_PATH.exec(path);
    return gizmo ? gizmo[2] : '';
  }

  function cleanId(value) {
    const text = String(value || '');
    return SAFE_ID.test(text) ? text : '';
  }

  function userMessageIndex(messages) {
    if (!Array.isArray(messages) || messages.length < 1 || messages.length > MAX_MESSAGES) return -1;
    let selected = -1;
    messages.forEach((message, index) => {
      if (!message || typeof message !== 'object') return;
      const role = String(message.author && message.author.role || '');
      if (role === 'user') selected = index;
    });
    return selected;
  }

  function hasPayload(value, depth) {
    if (depth > 8) return true;
    if (value == null) return false;
    if (typeof value === 'string') return value.trim().length > 0;
    if (typeof value === 'number' || typeof value === 'boolean') return true;
    if (Array.isArray(value)) return value.some((item) => hasPayload(item, depth + 1));
    if (typeof value !== 'object') return true;
    return Object.keys(value).some((key) => hasPayload(value[key], depth + 1));
  }

  function hasNonTextPayload(value, depth) {
    if (!value || typeof value !== 'object' || depth > 8) return depth > 8;
    return Object.keys(value).some((key) => {
      const child = value[key];
      if (NON_TEXT_PAYLOAD_KEY.test(key) && hasPayload(child, depth + 1)) return true;
      return child && typeof child === 'object' && hasNonTextPayload(child, depth + 1);
    });
  }

  function textSendActionRejectionCode(action) {
    if (TEXT_SEND_ACTIONS.has(action)) return '';
    if (typeof action !== 'string') return 'action_invalid';
    const token = action.trim().toLowerCase();
    return /^[a-z][a-z0-9_-]{0,31}$/.test(token)
      ? 'action_' + token
      : 'action_unrecognized';
  }

  function templateRejectionCode(body, pagePath, capturedAt) {
    const value = cloneJson(body);
    const path = cleanPath(pagePath);
    if (!value) return 'invalid_body';
    if (!path) return 'invalid_page_path_' + pathRejectionCode(pagePath);
    const actionRejection = textSendActionRejectionCode(value.action);
    if (actionRejection) return actionRejection;
    if (hasNonTextPayload(value, 0)) return 'non_text_payload';
    const index = userMessageIndex(value.messages);
    if (index < 0) return 'user_message_missing';
    const message = value.messages[index];
    const content = message && message.content;
    if (!cleanId(message && message.id)) return 'invalid_user_message_id';
    if (!content || content.content_type !== 'text') return 'content_not_text';
    if (!Array.isArray(content.parts) || content.parts.length !== 1) return 'invalid_text_parts';
    if (typeof content.parts[0] !== 'string') return 'text_part_not_string';
    const parentMessageId = cleanId(value.parent_message_id);
    const conversationId = value.conversation_id == null ? '' : cleanId(value.conversation_id);
    if (!parentMessageId) return 'invalid_parent_message_id';
    if (value.conversation_id != null && !conversationId) return 'invalid_conversation_id';
    const timestamp = Number(capturedAt);
    if (!Number.isFinite(timestamp) || timestamp <= 0) return 'invalid_timestamp';
    return '';
  }

  function createTemplate(body, pagePath, capturedAt) {
    if (templateRejectionCode(body, pagePath, capturedAt)) return null;
    const value = cloneJson(body);
    const path = cleanPath(pagePath);
    const index = userMessageIndex(value.messages);
    const message = value.messages[index];
    const parentMessageId = cleanId(value.parent_message_id);
    const conversationId = value.conversation_id == null ? '' : cleanId(value.conversation_id);
    const timestamp = Number(capturedAt);
    return {
      body: value,
      userIndex: index,
      pagePath: path,
      capturedAt: timestamp,
      parentMessageId,
      conversationId,
      userMessageId: cleanId(message.id),
      streamConfirmed: false
    };
  }

  function createRegenerateTemplate(body, pagePath, capturedAt) {
    const value = cloneJson(body);
    const path = cleanPath(pagePath);
    if (!value || !path || value.action !== 'variant') return null;
    if (value.messages != null && (!Array.isArray(value.messages) || value.messages.length !== 0)) {
      return null;
    }
    const parentMessageId = cleanId(value.parent_message_id);
    const conversationId = cleanId(value.conversation_id);
    const timestamp = Number(capturedAt);
    if (!parentMessageId || !conversationId || !Number.isFinite(timestamp) || timestamp <= 0) {
      return null;
    }
    return {
      body: value,
      pagePath: path,
      capturedAt: timestamp,
      parentMessageId,
      conversationId
    };
  }

  function streamRejectionCode(template, stream, pagePath, observedAt) {
    if (!stream || typeof stream !== 'object') return 'missing';
    if (String(stream.state || '') !== 'completed') return 'state';
    const assistantId = cleanId(stream.id);
    if (!assistantId) return 'assistant_id';
    const path = cleanPath(pagePath);
    if (!path) return 'page_path';
    const conversationId = cleanId(
      stream.conversationId || template && template.conversationId || pathConversationId(path)
    );
    if (!conversationId) return 'conversation_id';
    const timestamp = Number(observedAt);
    if (!Number.isFinite(timestamp) || timestamp <= 0) return 'timestamp';
    const pathConversationIdValue = pathConversationId(path);
    if (pathConversationIdValue && pathConversationIdValue !== conversationId) {
      return 'path_conversation_mismatch';
    }
    return '';
  }

  function createStreamReceipt(stream, pagePath, observedAt, fallbackConversationId) {
    const template = fallbackConversationId ? { conversationId: fallbackConversationId } : null;
    if (streamRejectionCode(template, stream, pagePath, observedAt)) return null;
    const path = cleanPath(pagePath);
    return Object.freeze({
      id: cleanId(stream.id),
      conversationId: cleanId(
        stream.conversationId || fallbackConversationId || pathConversationId(path)
      ),
      state: 'completed',
      pagePath: path,
      observedAt: Number(observedAt)
    });
  }

  function acceptStream(template, stream, pagePath, observedAt) {
    if (!template) return null;
    const receipt = createStreamReceipt(
      stream,
      pagePath,
      observedAt,
      template.conversationId
    );
    if (!receipt) return null;
    return Object.assign({}, template, {
      pagePath: receipt.pagePath,
      parentMessageId: receipt.id,
      conversationId: receipt.conversationId,
      streamConfirmed: true,
      capturedAt: receipt.observedAt
    });
  }

  function ready(template, pagePath, now) {
    if (!template || template.streamConfirmed !== true) return false;
    const path = cleanPath(pagePath);
    const timestamp = Number(now);
    if (!path || path !== template.pagePath || !Number.isFinite(timestamp)) return false;
    return timestamp - template.capturedAt >= 0 &&
      timestamp - template.capturedAt <= MAX_TEMPLATE_AGE_MS;
  }

  function buildBody(template, command, identifiers, now) {
    const prompt = String(command && command.prompt || '');
    const requestId = String(command && command.requestId || '');
    const userMessageId = cleanId(identifiers && identifiers.userMessageId);
    const requestUuid = cleanId(identifiers && identifiers.requestUuid);
    const turnId = cleanId(identifiers && identifiers.turnId);
    const timestamp = Number(now);
    if (!ready(template, command && command.pagePath, timestamp) || !REQUEST_ID.test(requestId) ||
        !prompt.trim() || prompt.length > MAX_PROMPT_LENGTH || !userMessageId ||
        !requestUuid || !turnId) return null;
    const body = cloneJson(template.body);
    const message = body && body.messages && body.messages[template.userIndex];
    if (!message || !message.content || !Array.isArray(message.content.parts)) return null;
    message.id = userMessageId;
    message.create_time = timestamp / 1000;
    message.content.parts = [prompt];
    if (message.metadata && typeof message.metadata === 'object') {
      if (Object.prototype.hasOwnProperty.call(message.metadata, 'request_id')) {
        message.metadata.request_id = requestUuid;
      }
      if (Object.prototype.hasOwnProperty.call(message.metadata, 'turn_exchange_id')) {
        message.metadata.turn_exchange_id = turnId;
      }
    }
    body.parent_message_id = template.parentMessageId;
    body.conversation_id = template.conversationId;
    if (Object.prototype.hasOwnProperty.call(body, 'websocket_request_id')) {
      body.websocket_request_id = requestUuid;
    }
    if (Object.prototype.hasOwnProperty.call(body, 'client_request_id')) {
      body.client_request_id = requestUuid;
    }
    return { body, userMessageId };
  }

  function buildRegenerateBody(template, command, identifiers, currentTurn, now) {
    const requestId = String(command && command.requestId || '');
    const requestUuid = cleanId(identifiers && identifiers.requestUuid);
    const turnId = cleanId(identifiers && identifiers.turnId);
    const pagePath = cleanPath(command && command.pagePath);
    const conversationId = cleanId(currentTurn && currentTurn.conversationId);
    const userMessageId = cleanId(currentTurn && currentTurn.userMessageId);
    const timestamp = Number(now);
    if (!template || !REQUEST_ID.test(requestId) || !requestUuid || !turnId || !pagePath ||
        pagePath !== template.pagePath || !conversationId || !userMessageId ||
        conversationId !== template.conversationId || !Number.isFinite(timestamp) ||
        timestamp - template.capturedAt < 0 || timestamp - template.capturedAt > MAX_TEMPLATE_AGE_MS) {
      return null;
    }
    const body = cloneJson(template.body);
    if (!body) return null;
    body.parent_message_id = userMessageId;
    body.conversation_id = conversationId;
    if (Object.prototype.hasOwnProperty.call(body, 'websocket_request_id')) {
      body.websocket_request_id = requestUuid;
    }
    if (Object.prototype.hasOwnProperty.call(body, 'client_request_id')) {
      body.client_request_id = requestUuid;
    }
    if (Object.prototype.hasOwnProperty.call(body, 'turn_exchange_id')) {
      body.turn_exchange_id = turnId;
    }
    return { body, userMessageId };
  }

  function invalidate(template, userMessageId) {
    if (!template) return null;
    const nextUserMessageId = cleanId(userMessageId);
    return Object.assign({}, template, {
      userMessageId: nextUserMessageId || template.userMessageId,
      streamConfirmed: false
    });
  }

  function classifyResponse(response) {
    if (!response || typeof response !== 'object') return { accepted: false, code: 'missing_response' };
    const status = Number(response.status) || 0;
    const contentType = String(response.headers && response.headers.get &&
      response.headers.get('content-type') || '').toLowerCase();
    if (response.ok === true && contentType.includes('text/event-stream')) {
      return { accepted: true, code: 'accepted', status };
    }
    if (status === 401 || status === 403) return { accepted: false, code: 'auth', status };
    if (status === 429) return { accepted: false, code: 'rate_limited', status };
    return { accepted: false, code: status ? 'http' : 'network', status };
  }

  window.__elonChatGptPrivateTextTransactionPolicy = Object.freeze({
    version: 11,
    acceptStream,
    buildBody,
    buildRegenerateBody,
    classifyResponse,
    createStreamReceipt,
    createTemplate,
    createRegenerateTemplate,
    invalidate,
    ready,
    streamRejectionCode,
    templateRejectionCode
  });
})();
