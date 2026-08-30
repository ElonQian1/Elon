'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.join(__dirname, '..');
const asset = (name) => fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'assets', name
), 'utf8');
const policySource = asset('chatgpt_web_private_text_transaction_policy.js');
const relaySource = asset('chatgpt_web_private_text_transaction_relay.js');
const adapterSource = asset('chatgpt_web_adapter.js');
const orchestratorSource = asset('chatgpt_web_text_transaction_orchestrator.js');
const streamSource = asset('chatgpt_web_private_stream_transport.js');
const pageAdapterSource = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'kotlin', 'com', 'elon', 'app',
  'chatgptweb', 'ChatGptWebPageAdapter.kt'
), 'utf8');
const buildGradle = fs.readFileSync(path.join(root, 'android', 'app', 'build.gradle'), 'utf8');

class FakeHeaders {
  constructor(values = {}) {
    this.values = {};
    if (values instanceof FakeHeaders) {
      Object.assign(this.values, values.values);
    } else {
      Object.entries(values).forEach(([name, value]) => {
        this.values[String(name).toLowerCase()] = String(value);
      });
    }
  }
  get(name) { return this.values[String(name).toLowerCase()] || null; }
  delete(name) { delete this.values[String(name).toLowerCase()]; }
  forEach(callback) {
    Object.entries(this.values).forEach(([name, value]) => callback(value, name, this));
  }
}

class FakeRequest {
  constructor(input, init = {}) {
    const source = input instanceof FakeRequest ? input : null;
    this.url = source ? source.url : String(input);
    this.method = String(init.method || source && source.method || 'GET').toUpperCase();
    this.headers = init.headers instanceof FakeHeaders
      ? init.headers
      : source ? source.headers : new FakeHeaders(init.headers || {});
    this._body = Object.prototype.hasOwnProperty.call(init, 'body')
      ? String(init.body)
      : source ? source._body : '';
    this.body = this._body ? {} : null;
    this.signal = init.signal || source && source.signal || null;
  }
  clone() { return new FakeRequest(this); }
  text() { return Promise.resolve(this._body); }
}

function response(status = 200, contentType = 'text/event-stream; charset=utf-8') {
  return {
    ok: status >= 200 && status < 300,
    status,
    headers: new FakeHeaders({ 'content-type': contentType })
  };
}

function createContext(enabled = true, options = {}) {
  const calls = [];
  let uuidSequence = 0;
  const location = {
    origin: 'https://chatgpt.com',
    href: 'https://chatgpt.com/c/conversation-one',
    pathname: '/c/conversation-one'
  };
  const window = {
    __elonChatGptPrivateTextTransactionsEnabled: enabled,
    setTimeout: options.setTimeout || setTimeout,
    clearTimeout: options.clearTimeout || clearTimeout,
    crypto: {
      randomUUID: () => {
        uuidSequence += 1;
        return `00000000-0000-4000-8000-${String(uuidSequence).padStart(12, '0')}`;
      }
    },
    fetch(input, init = {}) {
      calls.push({ input, init });
      return options.fetch
        ? options.fetch(input, init)
        : Promise.resolve(response());
    }
  };
  window.window = window;
  const sandbox = {
    window,
    location,
    URL,
    Request: FakeRequest,
    AbortController,
    Uint8Array,
    Promise,
    Date,
    JSON,
    Object,
    String,
    Number,
    Array,
    RegExp
  };
  vm.runInNewContext(policySource, sandbox, {
    filename: 'chatgpt_web_private_text_transaction_policy.js'
  });
  vm.runInNewContext(relaySource, sandbox, {
    filename: 'chatgpt_web_private_text_transaction_relay.js'
  });
  return { window, location, calls };
}

const tick = () => new Promise((resolve) => setImmediate(resolve));

function pendingTransportContext() {
  let timeoutCallback = null;
  const fixture = createContext(true, {
    setTimeout(callback) {
      timeoutCallback = callback;
      return 1;
    },
    clearTimeout() {},
    fetch(input, init) {
      if (init.__elonPrivateTransport !== 'text_transaction_v1') {
        return Promise.resolve(response());
      }
      return new Promise((resolve, reject) => {
        const rejectAborted = () => {
          const error = new Error('aborted');
          error.name = 'AbortError';
          reject(error);
        };
        if (init.signal && init.signal.aborted) {
          rejectAborted();
        } else if (init.signal) {
          init.signal.addEventListener('abort', rejectAborted, { once: true });
        }
      });
    }
  });
  return Object.assign(fixture, {
    triggerTimeout() {
      assert.equal(typeof timeoutCallback, 'function');
      timeoutCallback();
    }
  });
}

assert.match(buildGradle, /ELON_CHATGPT_PRIVATE_TEXT_TRANSACTIONS/);
assert.match(buildGradle, /CHATGPT_PRIVATE_TEXT_TRANSACTIONS_ENABLED/);
assert.ok(
  pageAdapterSource.indexOf('PRIVATE_FETCH_TAP_ASSET') <
  pageAdapterSource.indexOf('PRIVATE_TEXT_TRANSACTION_RELAY_ASSET')
);
assert.ok(
  pageAdapterSource.indexOf('chatgpt_web_private_text_transaction_relay.js') <
  pageAdapterSource.indexOf('chatgpt_web_private_stream_transport.js')
);
assert.ok(
  pageAdapterSource.indexOf('chatgpt_web_text_transaction_orchestrator.js') <
  pageAdapterSource.indexOf('chatgpt_web_adapter.js')
);
assert.match(orchestratorSource, /tryPrivateSend/);
assert.match(orchestratorSource, /private_text_v1:accepted/);
assert.match(orchestratorSource, /private_text_v1:unknown:/);
assert.match(orchestratorSource, /tryPrivateRegeneration/);
assert.match(orchestratorSource, /private_text_v1:regenerate_accepted/);
assert.match(adapterSource, /command\.allowPrivateTextTransaction === true/);
assert.match(adapterSource, /function invalidatePrivateTextContext\(\)/);
assert.match(adapterSource, /relay\.invalidateContext\(\)/);
[
  'select_model_option',
  'select_composer_tool',
  'request_attachment_upload',
  'set_ui_control_selected',
  'select_ui_control_choice',
  'set_ui_control_slider',
  'open_conversation',
  'open_project',
  'new_conversation'
].forEach((action) => {
  const start = adapterSource.indexOf(`if (action === '${action}'`);
  assert.ok(start >= 0, `missing adapter action: ${action}`);
  const next = adapterSource.indexOf("if (action === '", start + 1);
  const actionBlock = adapterSource.slice(start, next >= 0 ? next : undefined);
  assert.match(
    actionBlock,
    /invalidatePrivateTextContext\(\)/,
    `${action} must invalidate the previous authoritative request template`
  );
});
assert.match(streamSource, /preparePrivateSend/);
assert.match(streamSource, /mergePrivateUser/);
assert.match(streamSource, /finishPrivateSend/);

(async () => {
  const disabled = createContext(false);
  assert.equal(disabled.window.__elonChatGptPrivateTextTransactionRelay, undefined);

  const fixture = createContext(true);
  const relay = fixture.window.__elonChatGptPrivateTextTransactionRelay;
  assert.equal(relay.version, 15);
  assert.equal(relay.state().state, 'template_unavailable');

  const officialBody = {
    action: 'next',
    parent_message_id: 'parent-message-one',
    conversation_id: 'conversation-one',
    websocket_request_id: 'request-before',
    messages: [{
      id: 'user-message-one',
      author: { role: 'user' },
      create_time: 1,
      content: { content_type: 'text', parts: ['first official prompt'] },
      metadata: { request_id: 'request-before', turn_exchange_id: 'turn-before' }
    }]
  };
  const streamFirstFixture = createContext(true);
  const streamFirstRelay = streamFirstFixture.window.__elonChatGptPrivateTextTransactionRelay;
  const streamFirstValue = {
    id: 'assistant-race-one',
    conversationId: 'conversation-one',
    state: 'completed',
    text: 'must-not-enter-the-receipt'
  };
  const streamReceipt = streamFirstFixture.window
    .__elonChatGptPrivateTextTransactionPolicy
    .createStreamReceipt(streamFirstValue, '/c/conversation-one', Date.now());
  assert.notEqual(streamReceipt, null);
  assert.equal(JSON.stringify(streamReceipt).includes(streamFirstValue.text), false);
  assert.equal(streamFirstRelay.observeStream(streamFirstValue), false);
  await streamFirstFixture.window.fetch('/backend-api/f/conversation', {
    method: 'POST',
    body: JSON.stringify(officialBody)
  });
  await tick();
  assert.equal(
    streamFirstRelay.state().state,
    'ready',
    'a completed structural receipt is reconciled when request cloning finishes later'
  );
  const attachmentBody = JSON.parse(JSON.stringify(officialBody));
  attachmentBody.messages[0].metadata.attachments = [{ id: 'uploaded-file-one' }];
  await fixture.window.fetch('/backend-api/f/conversation/prepare', {
    method: 'POST',
    body: JSON.stringify({ action: 'next' })
  });
  await tick();
  assert.equal(
    relay.state().state,
    'template_unavailable',
    'prepare and init endpoints never participate in the message transaction template'
  );
  assert.equal(
    fixture.window.__elonChatGptPrivateTextTransactionPolicy.templateRejectionCode(
      attachmentBody,
      '/c/conversation-one',
      Date.now()
    ),
    'non_text_payload'
  );
  const currentOfficialBody = Object.assign({}, officialBody, { action: 'continue' });
  assert.notEqual(
    fixture.window.__elonChatGptPrivateTextTransactionPolicy.createTemplate(
      currentOfficialBody,
      '/c/conversation-one',
      Date.now()
    ),
    null,
    'the current official pure-text continue action remains replayable'
  );
  assert.equal(
    fixture.window.__elonChatGptPrivateTextTransactionPolicy.templateRejectionCode(
      Object.assign({}, officialBody, { action: 'variant' }),
      '/c/conversation-one',
      Date.now()
    ),
    'action_variant'
  );
  const projectTemplate =
    fixture.window.__elonChatGptPrivateTextTransactionPolicy.createTemplate(
      officialBody,
      '/g/g-p-project-one',
      Date.now()
    );
  assert.notEqual(projectTemplate, null, 'a project new-chat route can seed a text template');
  assert.notEqual(
    fixture.window.__elonChatGptPrivateTextTransactionPolicy.acceptStream(
      projectTemplate,
      {
        id: 'assistant-project-one',
        conversationId: 'conversation-one',
        state: 'completed'
      },
      '/g/g-p-project-one/c/conversation-one',
      Date.now()
    ),
    null,
    'the seeded project template follows the official route into its created conversation'
  );
  assert.notEqual(
    fixture.window.__elonChatGptPrivateTextTransactionPolicy.createTemplate(
      officialBody,
      '/zh-Hans/chat/c/conversation-one',
      Date.now()
    ),
    null,
    'safe same-origin chat route changes do not disable the transaction template'
  );
  assert.notEqual(
    fixture.window.__elonChatGptPrivateTextTransactionPolicy.createTemplate(
      officialBody,
      '/chat/@project/model:gpt/c/conversation-one',
      Date.now()
    ),
    null,
    'URL pchar route markers remain bound without exposing or interpreting route IDs'
  );
  assert.equal(
    fixture.window.__elonChatGptPrivateTextTransactionPolicy.createTemplate(
      officialBody,
      '/share/conversation-one',
      Date.now()
    ),
    null,
    'public share routes can never seed an authenticated write template'
  );
  assert.equal(
    fixture.window.__elonChatGptPrivateTextTransactionPolicy.templateRejectionCode(
      officialBody,
      '/share/conversation-one',
      Date.now()
    ),
    'invalid_page_path_blocked'
  );
  assert.equal(
    fixture.window.__elonChatGptPrivateTextTransactionPolicy.createTemplate(
      attachmentBody,
      '/c/conversation-one',
      Date.now()
    ),
    null,
    'the pure-text transport never captures a request containing attachment metadata'
  );
  const emptyAttachmentMetadata = JSON.parse(JSON.stringify(officialBody));
  emptyAttachmentMetadata.messages[0].metadata.attachments = [];
  assert.notEqual(
    fixture.window.__elonChatGptPrivateTextTransactionPolicy.createTemplate(
      emptyAttachmentMetadata,
      '/c/conversation-one',
      Date.now()
    ),
    null,
    'empty capability metadata does not disable otherwise pure-text requests'
  );
  await fixture.window.fetch('/backend-api/f/conversation', {
    method: 'POST',
    body: JSON.stringify(attachmentBody)
  });
  await tick();
  assert.equal(
    relay.state().state,
    'capture_non_text_payload',
    'rejected templates expose only a bounded structural reason code'
  );
  const secret = 'page-memory-only';
  const officialRequest = fixture.window.fetch('/backend-api/f/conversation', {
    method: 'POST',
    headers: new FakeHeaders({ authorization: secret }),
    body: JSON.stringify(officialBody)
  });
  fixture.location.pathname = '/transient-route-after-fetch';
  await officialRequest;
  await tick();
  fixture.location.pathname = '/c/conversation-one';
  assert.equal(relay.state().state, 'stream_not_confirmed');
  assert.equal(
    relay.dispatch({ prompt: 'must not send yet', requestId: 'mcp_before' }).dispatched,
    false
  );

  assert.equal(relay.observeStream({
    id: 'assistant-message-one',
    conversationId: 'conversation-one',
    state: 'completed'
  }), true);
  assert.equal(relay.state().state, 'ready');

  const transaction = relay.dispatch({
    prompt: 'second native prompt',
    requestId: 'mcp_private1'
  });
  assert.equal(transaction.dispatched, true);
  assert.equal(
    relay.dispatch({ prompt: 'duplicate', requestId: 'mcp_private2' }).dispatched,
    false,
    'a private write remains single-flight until response headers settle'
  );
  const completion = await transaction.completion;
  assert.equal(completion.status, 'accepted');
  assert.equal(completion.code, 'accepted');
  assert.equal(relay.state().state, 'busy', 'response headers do not end an active stream');
  assert.equal(relay.state().activeKind, 'send');
  assert.equal(fixture.calls.length, 4, 'prepare, diagnostic, official, and private requests were sent');
  const privateCall = fixture.calls[3];
  assert.equal(privateCall.init.__elonPrivateTransport, 'text_transaction_v1');
  assert.equal(privateCall.input.headers.get('authorization'), secret);
  const sentBody = JSON.parse(privateCall.input._body);
  assert.equal(sentBody.parent_message_id, 'assistant-message-one');
  assert.equal(sentBody.conversation_id, 'conversation-one');
  assert.equal(sentBody.messages[0].content.parts[0], 'second native prompt');
  assert.notEqual(sentBody.messages[0].id, officialBody.messages[0].id);
  assert.notEqual(sentBody.websocket_request_id, officialBody.websocket_request_id);
  assert.equal(JSON.stringify(relay.state()).includes(secret), false);
  assert.equal(JSON.stringify(relay.state()).includes('second native prompt'), false);

  const dynamicProofFixture = createContext();
  const dynamicProofRelay = dynamicProofFixture.window.__elonChatGptPrivateTextTransactionRelay;
  await dynamicProofFixture.window.fetch('/backend-api/f/conversation', {
    method: 'POST',
    headers: new FakeHeaders({
      authorization: secret,
      'openai-sentinel-proof-token': 'one-request-only'
    }),
    body: JSON.stringify(officialBody)
  });
  await tick();
  assert.equal(dynamicProofRelay.state().state, 'capture_dynamic_proof');
  const dynamicProofDispatch = dynamicProofRelay.dispatch({
    prompt: 'must use official flow',
    requestId: 'mcp_proof1'
  });
  assert.equal(dynamicProofDispatch.dispatched, false);
  assert.equal(dynamicProofDispatch.code, 'capture_dynamic_proof');

  const timeoutFixture = pendingTransportContext();
  const timeoutRelay = timeoutFixture.window.__elonChatGptPrivateTextTransactionRelay;
  await timeoutFixture.window.fetch('/backend-api/f/conversation', {
    method: 'POST',
    headers: new FakeHeaders({ authorization: secret }),
    body: JSON.stringify(officialBody)
  });
  await tick();
  assert.equal(timeoutRelay.observeStream({
    id: 'assistant-timeout-seed',
    conversationId: 'conversation-one',
    state: 'completed'
  }), true);
  const timedOutTransaction = timeoutRelay.dispatch({
    prompt: 'timeout without replay',
    requestId: 'mcp_timeout1'
  });
  assert.equal(timedOutTransaction.dispatched, true);
  timeoutFixture.triggerTimeout();
  const timedOutCompletion = await timedOutTransaction.completion;
  assert.equal(timedOutCompletion.status, 'unknown');
  assert.equal(timedOutCompletion.code, 'timeout');
  assert.equal(timeoutRelay.state().active, false);
  assert.equal(timeoutRelay.state().failures, 1);

  const stoppedFixture = pendingTransportContext();
  const stoppedRelay = stoppedFixture.window.__elonChatGptPrivateTextTransactionRelay;
  await stoppedFixture.window.fetch('/backend-api/f/conversation', {
    method: 'POST',
    headers: new FakeHeaders({ authorization: secret }),
    body: JSON.stringify(officialBody)
  });
  await tick();
  assert.equal(stoppedRelay.observeStream({
    id: 'assistant-stop-seed',
    conversationId: 'conversation-one',
    state: 'completed'
  }), true);
  const stoppedTransaction = stoppedRelay.dispatch({
    prompt: 'explicit user stop',
    requestId: 'mcp_stoppending'
  });
  assert.equal(stoppedTransaction.dispatched, true, stoppedTransaction.code);
  assert.equal(stoppedRelay.stop('mcp_stoppending'), true);
  const stoppedCompletion = await stoppedTransaction.completion;
  assert.equal(stoppedCompletion.status, 'accepted');
  assert.equal(stoppedCompletion.code, 'stopped');
  assert.equal(stoppedRelay.state().active, false);
  assert.equal(stoppedRelay.state().failures, 0);

  assert.equal(relay.observeStream({
    id: 'assistant-message-two',
    conversationId: 'conversation-one',
    state: 'completed'
  }), true);
  assert.equal(relay.state().state, 'ready');
  assert.equal(relay.state().active, false);

  const regenerateBody = {
    action: 'variant',
    parent_message_id: sentBody.messages[0].id,
    conversation_id: 'conversation-one',
    websocket_request_id: 'regenerate-before',
    turn_exchange_id: 'regenerate-turn-before',
    messages: []
  };
  await fixture.window.fetch('/backend-api/f/conversation', {
    method: 'POST',
    headers: new FakeHeaders({ authorization: secret }),
    body: JSON.stringify(regenerateBody)
  });
  await tick();
  assert.equal(relay.state().regenerateReady, true);

  const regeneration = relay.dispatchRegenerate({ requestId: 'mcp_regen1' });
  assert.equal(regeneration.dispatched, true);
  assert.equal(regeneration.kind, 'regenerate');
  assert.equal((await regeneration.completion).status, 'accepted');
  assert.equal(relay.state().activeKind, 'regenerate');
  assert.equal(fixture.calls.length, 6, 'regeneration adds exactly one private request');
  const regeneratedBody = JSON.parse(fixture.calls[5].input._body);
  assert.equal(regeneratedBody.action, 'variant');
  assert.equal(regeneratedBody.parent_message_id, sentBody.messages[0].id);
  assert.notEqual(regeneratedBody.websocket_request_id, regenerateBody.websocket_request_id);
  assert.notEqual(regeneratedBody.turn_exchange_id, regenerateBody.turn_exchange_id);
  assert.equal(relay.observeStream({
    id: 'assistant-message-regenerated',
    conversationId: 'conversation-one',
    state: 'completed'
  }), true);
  assert.equal(relay.state().state, 'ready');

  fixture.location.pathname = '/c/conversation-two';
  assert.equal(
    relay.dispatch({ prompt: 'wrong conversation', requestId: 'mcp_wrong' }).dispatched,
    false,
    'a template can never cross conversation paths'
  );
  fixture.location.pathname = '/c/conversation-one';

  const stoppable = relay.dispatch({
    prompt: 'stop this native prompt',
    requestId: 'mcp_stop1'
  });
  assert.equal(stoppable.dispatched, true);
  assert.equal((await stoppable.completion).status, 'accepted');
  assert.equal(relay.stop('mcp_other'), false);
  assert.equal(relay.stop('mcp_stop1'), true);
  assert.equal(relay.state().active, false);
  assert.equal(relay.state().state, 'stream_not_confirmed');
  assert.equal(
    relay.dispatch({ prompt: 'must reconcile first', requestId: 'mcp_afterstop' }).dispatched,
    false,
    'a stopped or uncertain turn can never reuse the old parent message'
  );
  assert.equal(relay.observeStream({
    id: 'assistant-message-stopped',
    conversationId: 'conversation-one',
    state: 'completed'
  }), true);
  const regenerateStoppedTurn = relay.dispatchRegenerate({ requestId: 'mcp_regenstop' });
  assert.equal(regenerateStoppedTurn.dispatched, true);
  assert.equal((await regenerateStoppedTurn.completion).status, 'accepted');
  const regenerateStoppedBody = JSON.parse(fixture.calls.at(-1).input._body);
  assert.equal(
    regenerateStoppedBody.parent_message_id,
    stoppable.userMessageId,
    'a stopped turn reconciles against the user message that actually created it'
  );
  assert.equal(relay.observeStream({
    id: 'assistant-message-after-stop-regenerate',
    conversationId: 'conversation-one',
    state: 'completed'
  }), true);

  assert.equal(relay.invalidateContext(), true);
  assert.equal(relay.state().state, 'template_unavailable');
  assert.equal(relay.state().regenerateReady, false);
  assert.equal(
    relay.dispatch({ prompt: 'stale state must not send', requestId: 'mcp_stale' }).dispatched,
    false,
    'model, tool, temporary-chat, and navigation changes invalidate the old request template'
  );

  await fixture.window.fetch('/backend-api/f/conversation', {
    method: 'POST',
    headers: new FakeHeaders({ authorization: secret }),
    body: JSON.stringify(officialBody)
  });
  await tick();
  assert.equal(relay.observeStream({
    id: 'assistant-message-reseeded',
    conversationId: 'conversation-one',
    state: 'completed'
  }), true);
  const pendingInvalidation = relay.dispatch({
    prompt: 'invalidate after active',
    requestId: 'mcp_invalidate'
  });
  assert.equal(pendingInvalidation.dispatched, true);
  assert.equal((await pendingInvalidation.completion).status, 'accepted');
  assert.equal(relay.invalidateContext(), false, 'an active request remains reconcilable');
  assert.equal(relay.state().state, 'busy');
  assert.equal(relay.observeStream({
    id: 'assistant-message-invalidated',
    conversationId: 'conversation-one',
    state: 'completed'
  }), true);
  assert.equal(relay.state().state, 'template_unavailable');

  relay.dispose();
  assert.equal(relay.state().state, 'disposed');
  console.log('CHATGPT_WEB_PRIVATE_TEXT_TRANSACTION_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
