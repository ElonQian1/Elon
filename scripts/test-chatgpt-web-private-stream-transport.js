'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const zlib = require('node:zlib');

const asset = (name) => fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets', name
), 'utf8');
const policySource = asset('chatgpt_web_private_stream_policy.js');
const transportSource = asset('chatgpt_web_private_stream_transport.js');
const compactFixture = JSON.parse(fs.readFileSync(path.join(
  __dirname, 'fixtures', 'chatgpt-private-stream-compact.json'
), 'utf8'));
const buildGradle = fs.readFileSync(path.join(__dirname, '..', 'android', 'app', 'build.gradle'), 'utf8');
const pageAdapter = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'kotlin',
  'com', 'elon', 'app', 'chatgptweb', 'ChatGptWebPageAdapter.kt'
), 'utf8');

assert.match(
  buildGradle,
  /findProperty\("ELON_CHATGPT_PRIVATE_STREAM_OBSERVER"\)[\s\S]*?\?\.toBoolean\(\) \?: true/
);
assert.match(buildGradle, /buildConfigField "boolean", "CHATGPT_PRIVATE_STREAM_OBSERVER_ENABLED"/);
assert.match(pageAdapter, /BuildConfig\.CHATGPT_PRIVATE_STREAM_OBSERVER_ENABLED/);
assert.match(pageAdapter, /WebViewFeature\.DOCUMENT_START_SCRIPT/);
assert.match(pageAdapter, /WebViewCompat\.addDocumentStartJavaScript/);
assert.match(pageAdapter, /chatgpt_web_private_socket_tap\.js/);
assert.ok(
  pageAdapter.indexOf('chatgpt_web_private_stream_policy.js') <
  pageAdapter.indexOf('chatgpt_web_private_stream_transport.js')
);
assert.ok(
  pageAdapter.indexOf('chatgpt_web_private_stream_transport.js') <
  pageAdapter.indexOf('chatgpt_web_adapter.js')
);

const tick = () => new Promise((resolve) => setImmediate(resolve));

function createResponse(chunks) {
  const encoded = chunks.map((value) => new TextEncoder().encode(value));
  return {
    ok: true,
    status: 200,
    headers: { get: (name) => name === 'content-type' ? 'text/event-stream; charset=utf-8' : null },
    clone: () => ({
      body: {
        getReader: () => {
          let index = 0;
          return {
            read: async () => index < encoded.length
              ? { done: false, value: encoded[index++] }
              : { done: true },
            releaseLock: () => {}
          };
        }
      }
    })
  };
}

function createJsonResponse(payload) {
  return {
    ok: true,
    status: 200,
    headers: { get: (name) => name === 'content-type' ? 'application/json' : null },
    clone: () => ({ json: async () => payload })
  };
}

function createAccessResponse(status) {
  return {
    ok: false,
    status,
    headers: { get: () => 'application/json' },
    clone: () => ({ json: async () => ({}) })
  };
}

function context(enabled, response) {
  let calls = 0;
  const outcomes = [];
  const shapes = [];
  const socketListeners = new Set();
  const originalFetch = async () => {
    calls += 1;
    return response;
  };
  const window = {
    __elonChatGptPrivateStreamObserverEnabled: enabled,
    __elonChatGptPrivateResearchProbe: {
      recordPrivateStreamOutcome: (outcome, frames, elapsedMs) =>
        outcomes.push({ outcome, frames, elapsedMs }),
      recordPrivateStreamShape: (shape) => shapes.push(shape)
    },
    __elonChatGptPrivateSocketTap: {
      version: 1,
      subscribe: (listener) => {
        socketListeners.add(listener);
        return () => socketListeners.delete(listener);
      }
    },
    fetch: originalFetch
  };
  window.window = window;
  const sandbox = {
    window,
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/conversation-one' },
    URL,
    Promise,
    Date,
    JSON,
    TextDecoder,
    DecompressionStream,
    Blob,
    Uint8Array,
    atob,
    Set,
    Object,
    String,
    Number,
    Array,
    RegExp
  };
  vm.runInNewContext(policySource, sandbox, { filename: 'chatgpt_web_private_stream_policy.js' });
  vm.runInNewContext(transportSource, sandbox, { filename: 'chatgpt_web_private_stream_transport.js' });
  return {
    window,
    originalFetch,
    calls: () => calls,
    outcomes,
    shapes,
    emitSocket: (value) => socketListeners.forEach((listener) => listener(value)),
    socketListenerCount: () => socketListeners.size
  };
}

(async () => {
  const response = createResponse([
    'data: {"conversation_id":"conversation-one","message":{"id":"assistant-one",',
    '"author":{"role":"assistant"},"status":"in_progress","content":{"parts":["hello"]}}}\n\n',
    'data: {"conversation_id":"conversation-one","message":{"id":"assistant-one",',
    '"author":{"role":"assistant"},"status":"finished_successfully","content":{"parts":["hello world"]}}}\n\n',
    'data: [DONE]\n\n'
  ]);
  const enabled = context(true, response);
  assert.equal(enabled.window.__elonChatGptPrivateStreamTransport.version, 8);
  assert.equal(enabled.socketListenerCount(), 1);
  let notifications = 0;
  enabled.window.__elonChatGptPrivateStreamTransport.subscribe(() => { notifications += 1; });

  const request = { method: 'POST', url: 'https://chatgpt.com/backend-api/f/conversation' };
  const init = { method: 'POST' };
  Object.defineProperty(init, 'headers', { get: () => { throw new Error('headers must not be read'); } });
  Object.defineProperty(init, 'body', { get: () => { throw new Error('body must not be read'); } });
  const returned = await enabled.window.fetch(request, init);
  await tick();
  await tick();
  assert.equal(returned, response);
  assert.equal(enabled.calls(), 1);
  assert.ok(notifications >= 2);
  assert.deepEqual(enabled.outcomes.map((item) => [item.outcome, item.frames]), [
    ['first', 1],
    ['success', 2]
  ]);
  assert.deepEqual(enabled.shapes, [
    't:none/k:conversation_id.message/dt:none/dk:none/mk:author.content.id.status/ck:parts',
    't:none/k:conversation_id.message/dt:none/dk:none/mk:author.content.id.status/ck:parts'
  ]);

  const compactResponse = createResponse([
    'data: {"c":"patch","o":"replace","p":"/messages/2/content/parts/0","v":{"text":"sentinel"}}\n\n',
    'data: [DONE]\n\n'
  ]);
  const compact = context(true, compactResponse);
  await compact.window.fetch(request, init);
  await tick();
  await tick();
  assert.ok(compact.shapes.includes(
    'compact/c:patch/o:replace/p:/messages/{index}/content/parts/{index}/v:object/vk:text'
  ));
  compact.emitSocket(JSON.stringify({
    type: 'reply',
    id: 'official-request-id',
    reply: JSON.stringify({
      c: 'patch',
      o: 'replace',
      p: '/messages/2',
      v: {
        conversation_id: 'conversation-one',
        message: {
          id: 'assistant-socket',
          author: { role: 'assistant' },
          status: 'finished_successfully',
          content: { parts: ['socket answer'] }
        }
      }
    })
  }));
  assert.equal(compact.calls(), 1, 'socket observation never duplicates the official request');
  assert.equal(
    compact.window.__elonChatGptPrivateStreamTransport.current('/c/conversation-one').text,
    'socket answer'
  );
  assert.ok(compact.shapes.some((shape) => shape.startsWith('socket/compact/')));
  compact.emitSocket(JSON.stringify({
    type: 'conversation-update',
    payload: {
      conversation_id: 'conversation-one',
      update_type: 'message',
      update_content: [JSON.stringify({
        message: {
          id: 'assistant-conversation-update',
          author: { role: 'assistant' },
          status: 'finished_successfully',
          content: { parts: ['conversation update answer'] }
        }
      })]
    }
  }));
  assert.equal(
    compact.window.__elonChatGptPrivateStreamTransport.current('/c/conversation-one').text,
    'conversation update answer'
  );
  compact.emitSocket(JSON.stringify({
    type: 'conversation-update',
    payload: {
      conversation_id: 'conversation-one',
      update_type: 'message',
      update_content: {
        opaque_wrapper: {
          nested_items: [{
            message: {
              id: 'assistant-wrapped-update',
              author: { role: 'assistant' },
              status: 'finished_successfully',
              content: { parts: ['wrapped conversation update answer'] }
            }
          }]
        }
      }
    }
  }));
  assert.equal(
    compact.window.__elonChatGptPrivateStreamTransport.current('/c/conversation-one').text,
    'wrapped conversation update answer'
  );
  assert.ok(compact.shapes.includes(
    'socket/field:update_content/shape:object/k:opaque_wrapper'
  ));

  const statusResponse = createJsonResponse({
    result: {
      updates: [{
        message: {
          id: 'assistant-stream-status',
          author: { role: 'assistant' },
          status: 'finished_successfully',
          content: { parts: ['stream status answer'] }
        }
      }]
    }
  });
  const status = context(true, statusResponse);
  await status.window.fetch(
    'https://chatgpt.com/backend-api/conversation/conversation-one/stream_status',
    { method: 'GET' }
  );
  await tick();
  await tick();
  assert.equal(status.calls(), 1, 'stream status observation never duplicates the official request');
  assert.equal(
    status.window.__elonChatGptPrivateStreamTransport.current('/c/conversation-one').text,
    'stream status answer'
  );
  assert.ok(status.shapes.some((shape) => shape.startsWith('status/')));
  await status.window.fetch(
    'https://chatgpt.com/backend-api/conversation/conversation-one/stream_status',
    { method: 'GET' }
  );
  await tick();
  await tick();
  assert.deepEqual(status.outcomes.map((item) => item.outcome), ['first', 'success']);
  assert.equal(
    enabled.window.__elonChatGptPrivateStreamTransport.current('/c/conversation-one').text,
    'hello world'
  );
  const merged = enabled.window.__elonChatGptPrivateStreamTransport.mergeMessages([], '/c/conversation-one');
  assert.equal(merged.length, 1);
  assert.equal(merged[0].state, 'completed');
  assert.equal(merged[0].content[0].text, 'hello world');

  assert.equal(compactFixture.sourceShapeSha256.length, 64);
  const compactFrame = enabled.window.__elonChatGptPrivateStreamPolicy.assistantFrame(
    compactFixture.completed.payload
  );
  assert.equal(compactFrame.state, 'completed');
  assert.match(compactFrame.text, /\[Reuters \+1\]\(https:\/\/www\.reuters\.com\/markets\/example\)/);
  assert.equal(compactFrame.citations.length, 1);
  assert.equal(compactFrame.citations[0].type, 'citation');
  assert.equal(compactFrame.citations[0].text, 'Reuters');
  assert.equal(compactFrame.citations[0].url, 'https://www.reuters.com/markets/example');
  assert.equal(compactFrame.citations[0].markerText, 'Reuters +1');
  assert.equal(compactFrame.citations[0].citationId, 'private-ref-0');
  assert.equal(compactFrame.citations[0].groupSize, 2);
  assert.equal(compactFrame.citations[0].targetHost, 'reuters.com');
  assert.equal(
    enabled.window.__elonChatGptPrivateStreamPolicy.assistantFrame(
      compactFixture.nonVisibleToolFrame.payload
    ),
    null,
    'compact tool-call frames must not leak into the visible assistant reply'
  );
  const compactMerged = enabled.window.__elonChatGptPrivateStreamPolicy.mergeMessages([], compactFrame);
  assert.equal(compactMerged[0].content.map((part) => part.type).join(','), 'markdown,citation');
  assert.equal(compactMerged[0].content[1].url, 'https://www.reuters.com/markets/example');

  const widget = {
    default_range: '1D',
    timeframe_order: ['1D'],
    timeframe_configs: {
      '1D': {
        chart: { data: [
          { timestamp: 1, close: 77000, formatted: '12:00 上午' },
          { timestamp: 2, close: 77100, formatted: '12:05 上午' }
        ] },
        summary: {
          price_text: 'US$77,100.00',
          price_change_text: '+US$100.00 (0.13%)',
          price_change_color: 'success'
        }
      }
    },
    asset_display_name: 'Bitcoin (BTC)',
    current_price_text: 'US$77,100.00',
    metrics_display: [{ cols: [{ label: '当日最低价', value: '75,853' }] }]
  };
  const compressedWidget = zlib.gzipSync(Buffer.from(JSON.stringify(widget))).toString('base64url');
  const widgetPayload = {
    conversation_id: 'conversation-one',
    message: {
      id: 'assistant-widget',
      author: { role: 'assistant' },
      status: 'finished_successfully',
      content: { content_type: 'text', parts: ['finance answer'] },
      metadata: { view_state: { widgets: {
        'finance-widget:0': {
          __encoding: 'gzip-json-base64url-v1',
          __compressed: compressedWidget
        }
      } } }
    }
  };
  const finance = context(true, createResponse([
    'data: ' + JSON.stringify(widgetPayload) + '\n\n',
    'data: [DONE]\n\n'
  ]));
  await finance.window.fetch(request, init);
  for (let index = 0; index < 30; index += 1) {
    const active = finance.window.__elonChatGptPrivateStreamTransport.current('/c/conversation-one');
    if (active && active.richParts && active.richParts.length) break;
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  const financeCurrent = finance.window.__elonChatGptPrivateStreamTransport.current('/c/conversation-one');
  assert.equal(financeCurrent.richParts.length, 1);
  assert.equal(financeCurrent.richParts[0].richContent.kind, 'finance');
  assert.equal(financeCurrent.richParts[0].richContent.payload.chart.points.length, 2);
  assert.ok(finance.shapes.includes('widget/finance/decoded'));

  const largeTurnId = 'turn-finance-live';
  const largeWidgetPayload = {
    c: 10,
    v: {
      conversation_id: 'conversation-one',
      message: {
        id: 'assistant-widget-prefetch',
        author: { role: 'assistant' },
        status: 'finished_partial_completion',
        content: { content_type: 'text', parts: ['\ue200genui\ue202chart\ue201'] },
        metadata: {
          turn_exchange_id: largeTurnId,
          view_state: { widgets: {
            'assistant-widget-prefetch:0': {
              __encoding: 'gzip-json-base64url-v1',
              __compressed: compressedWidget
            }
          } }
        }
      }
    }
  };
  const largeFinalPayload = {
    c: 16,
    v: {
      conversation_id: 'conversation-one',
      message: {
        id: 'assistant-visible-final',
        author: { role: 'assistant' },
        status: 'finished_successfully',
        content: { content_type: 'text', parts: ['visible finance answer'] },
        metadata: { turn_exchange_id: largeTurnId }
      }
    }
  };
  const paddingPayload = {
    c: 15,
    v: {
      conversation_id: 'conversation-one',
      message: {
        id: 'assistant-padding',
        author: { role: 'assistant' },
        status: 'finished_partial_completion',
        content: { content_type: 'text', parts: ['x'.repeat(600000)] },
        metadata: { turn_exchange_id: largeTurnId }
      }
    }
  };
  const large = context(true, createResponse([
    'data: ' + JSON.stringify(largeWidgetPayload) + '\n\n' +
      'data: ' + JSON.stringify(paddingPayload) + '\n\n' +
      'data: ' + JSON.stringify(largeFinalPayload) + '\n\n' +
      'data: [DONE]\n\n'
  ]));
  await large.window.fetch(request, init);
  for (let index = 0; index < 50; index += 1) {
    const active = large.window.__elonChatGptPrivateStreamTransport.current('/c/conversation-one');
    if (active && active.richParts && active.richParts.length) break;
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  const largeCurrent = large.window.__elonChatGptPrivateStreamTransport.current('/c/conversation-one');
  assert.equal(largeCurrent.id, 'assistant-visible-final');
  assert.equal(largeCurrent.turnId, largeTurnId);
  assert.equal(largeCurrent.richParts.length, 1,
    'an async widget from the same turn survives later assistant message ids');
  assert.equal(largeCurrent.richParts[0].richContent.payload.chart.points.length, 2);
  assert.ok(large.shapes.includes('widget/finance/decoded'));

  await enabled.window.fetch({
    method: 'POST',
    url: 'https://chatgpt.com/backend-api/f/conversation/stream'
  }, { method: 'POST' });
  await tick();
  assert.equal(enabled.calls(), 2, 'versioned stream paths use the same single official request');

  await enabled.window.fetch({
    method: 'POST',
    url: 'https://chatgpt.com/backend-anon/conversation'
  }, { method: 'POST' });
  await tick();
  assert.equal(enabled.calls(), 3, 'guest conversation streams are observed without request replay');

  const denied = context(true, createAccessResponse(403));
  await denied.window.fetch(request, init);
  const deniedAccess = denied.window.__elonChatGptPrivateStreamTransport.access();
  assert.equal(deniedAccess.reason, 'login_required');
  assert.equal(deniedAccess.status, 403);
  assert.ok(deniedAccess.observedAt > 0, 'a passive 401/403 response adds a bounded login hint');
  assert.equal(denied.calls(), 1, 'access classification must not replay the official request');

  const limited = context(true, createAccessResponse(429));
  await limited.window.fetch(request, init);
  assert.equal(limited.window.__elonChatGptPrivateStreamTransport.access().reason, 'rate_limited');

  enabled.window.__elonChatGptPrivateStreamTransport.dispose();
  assert.equal(enabled.window.fetch, enabled.originalFetch);
  assert.equal(enabled.socketListenerCount(), 0);

  const disabled = context(false, response);
  assert.equal(disabled.window.__elonChatGptPrivateStreamTransport, undefined);
  assert.equal(disabled.window.fetch, disabled.originalFetch);

  console.log('CHATGPT_WEB_PRIVATE_STREAM_TRANSPORT_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
