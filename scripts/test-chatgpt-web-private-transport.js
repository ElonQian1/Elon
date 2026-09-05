'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_private_transport.js'
), 'utf8');
const policySource = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_private_transport_policy.js'
), 'utf8');

function jsonResponse(value) {
  return { ok: true, status: 200, json: async () => value };
}

class MemoryStorage {
  constructor() { this.values = new Map(); }
  getItem(key) { return this.values.has(key) ? this.values.get(key) : null; }
  setItem(key, value) { this.values.set(key, String(value)); }
}

function createContext(
  fetchImpl,
  researchEnabled = true,
  prefetchEnabled = true,
  storage = new MemoryStorage(),
  directoryRows = [],
  authContext = null
) {
  const timers = new Set();
  const outcomes = [];
  const shapes = [];
  const membershipAccepts = [];
  const location = {
    origin: 'https://chatgpt.com',
    pathname: '/',
    href: 'https://chatgpt.com/'
  };
  const window = {
    __elonChatGptPrivateResearchEnabled: researchEnabled,
    __elonChatGptPrivateConversationPrefetchEnabled: prefetchEnabled,
    fetch: fetchImpl,
    __elonChatGptPrivateConversationDirectory: {
      acceptConversationMembership: (id, title, projectId) => {
        membershipAccepts.push({ id, title, projectId });
        return true;
      },
      refreshProject: async () => true,
      snapshot: () => ({ conversations: directoryRows })
    },
    sessionStorage: storage,
    setTimeout: (callback) => {
      const id = setTimeout(callback, 10000);
      timers.add(id);
      return id;
    },
    clearTimeout: (id) => { clearTimeout(id); timers.delete(id); }
  };
  if (authContext) window.__elonChatGptPrivateAuthContext = authContext;
  if (researchEnabled) {
    window.__elonChatGptPrivateResearchProbe = {
      recordPrivateOutcome: (outcome, messageCount, elapsedMs) => {
        outcomes.push({ outcome, messageCount, elapsedMs });
      },
      recordPrivatePayloadShape: (payload) => { shapes.push(payload); }
    };
  }
  window.window = window;
  window.location = location;
  const context = {
    window,
    location,
    URL,
    AbortController,
    Date,
    Number,
    String,
    Array,
    Object,
    Map,
    Set,
    Promise,
    Math,
    JSON,
    encodeURIComponent
  };
  vm.runInNewContext(policySource, context, {
    filename: 'chatgpt_web_private_transport_policy.js'
  });
  for (const asset of ['chatgpt_web_private_history_projection.js', 'chatgpt_web_private_stream_policy.js']) {
    vm.runInNewContext(fs.readFileSync(path.join(
      __dirname, '..', 'android', 'app', 'src', 'main', 'assets', asset
    ), 'utf8'), context, { filename: asset });
  }
  vm.runInNewContext(source, context, { filename: 'chatgpt_web_private_transport.js' });
  return { window, timers, storage, outcomes, shapes, membershipAccepts };
}

async function flush() {
  await new Promise((resolve) => setTimeout(resolve, 20));
}

const detailPayload = {
  title: 'Visible title',
  current_node: 'assistant-node',
  mapping: {
    'user-node': {
      parent: '',
      message: {
        id: 'user-message',
        author: { role: 'user' },
        content: { parts: ['hello'] },
        status: 'finished_successfully'
      }
    },
    'assistant-node': {
      parent: 'user-node',
      message: {
        id: 'assistant-message',
        author: { role: 'assistant' },
        content: { parts: ['hi'] },
        status: 'finished_successfully'
      }
    }
  }
};

(async () => {
  const disabled = createContext(async () => jsonResponse(detailPayload), false, false);
  assert.equal(disabled.window.__elonChatGptPrivateTransport, undefined);

  const gated = createContext(async () => jsonResponse(detailPayload), true, false);
  assert.equal(gated.window.__elonChatGptPrivateTransport.version, 18);
  assert.equal(gated.window.__elonChatGptPrivateTransport.conversationPrefetchEnabled, false);
  assert.equal(gated.window.__elonChatGptPrivateTransport.conversationPrefetchReady(), false);

  const requests = [];
  const snapshots = [];
  let navigated = 0;
  const detail = createContext(async (url, options) => {
    requests.push({ url, options });
    return jsonResponse(detailPayload);
  }, false, true);
  const transport = detail.window.__elonChatGptPrivateTransport;
  assert.equal(transport.version, 18);
  assert.equal(transport.conversationPrefetchEnabled, true);
  assert.equal(transport.conversationPrefetchAvailable, true);
  assert.equal(transport.experimentalConversationPrefetchAvailable, true);
  assert.equal(transport.conversationPrefetchReady(), false);
  assert.equal(transport.prefetchConversation(
    '/c/cold-chat',
    () => assert.fail('cold prefetch must not emit'),
    () => assert.fail('cold prefetch leaves navigation to the adapter')
  ), false);

  await detail.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  assert.equal(transport.conversationPrefetchReady(), true);
  assert.equal(transport.prefetchConversation(
    '/c/plain-chat',
    (event) => snapshots.push(event),
    () => { navigated += 1; }
  ), true);
  await flush();
  assert.equal(navigated, 1);
  assert.equal(snapshots.length, 1);
  assert.equal(requests.length, 2);
  assert.equal(requests[1].url, '/backend-api/conversations/plain-chat');
  assert.equal(requests[1].options.headers.Authorization, 'page-scoped-value');
  assert.equal(requests[1].options.__elonPrivateTransport, 'conversation_prefetch');
  assert.equal(snapshots[0].composerReady, false);
  assert.equal(snapshots[0].snapshotScope, 'content');
  assert.equal(transport.health().successes, 1);
  assert.equal(transport.health().lastOutcome, 'success');
  assert.equal(detail.outcomes.length, 0);
  assert.equal(detail.shapes.length, 0);
  assert.deepEqual(
    Array.from(snapshots[0].messages, (value) => [value.role, value.content[0].text]),
    [['user', 'hello'], ['assistant', 'hi']]
  );
  assert.ok(snapshots[0].messages.every((value) => Array.isArray(value.content)),
    'private snapshots must use the same content-array contract as the native parser');

  let warmedAcquisitions = 0;
  const warmedRequests = [];
  const warmedSnapshots = [];
  const warmed = createContext(
    async (url, options) => {
      warmedRequests.push({ url, options });
      return jsonResponse(detailPayload);
    },
    false,
    true,
    new MemoryStorage(),
    [],
    {
      canAcquire: () => true,
      state: () => ({
        ready: true,
        lastOutcome: 'session_ready',
        lastSuccessAt: Date.now(),
        lastLatencyMs: 120
      }),
      subscribe: () => () => {},
      copyRequestHeaders: () => ({ Authorization: 'Bearer warmed-page-context' }),
      acquireRequestHeaders: async () => {
        warmedAcquisitions += 1;
        return { Authorization: 'Bearer warmed-page-context' };
      },
      acceptObservedHeaders: () => false,
      invalidate: () => {}
    }
  );
  const warmedTransport = warmed.window.__elonChatGptPrivateTransport;
  assert.equal(warmedTransport.conversationPrefetchReady(), true);
  assert.equal(warmedTransport.prefetchConversation(
    '/c/warmed-chat',
    (event) => warmedSnapshots.push(event),
    () => {}
  ), true);
  await flush();
  assert.equal(warmedAcquisitions, 0);
  assert.equal(warmedRequests.length, 1);
  assert.equal(warmedRequests[0].options.headers.Authorization, 'Bearer warmed-page-context');
  assert.equal(warmedSnapshots.length, 1);

  detail.window.location.pathname = '/c/voice-chat';
  assert.equal(transport.refreshCurrentConversation(
    '/c/other-chat',
    () => assert.fail('refresh cannot read a different conversation')
  ), false);
  assert.equal(transport.refreshCurrentConversation(
    '/c/voice-chat',
    (event) => snapshots.push(event)
  ), true);
  await flush();
  assert.equal(requests.length, 3);
  assert.equal(requests[2].url, '/backend-api/conversations/voice-chat');
  assert.equal(snapshots.length, 2);

  detail.window.location.pathname = '/g/g-p-family/c/project-voice-chat';
  assert.equal(transport.refreshCurrentConversation(
    '/g/g-p-family/c/project-voice-chat',
    (event) => snapshots.push(event)
  ), true);
  await flush();
  assert.equal(requests.length, 4);
  assert.equal(requests[3].url, '/backend-api/conversations/project-voice-chat');
  assert.equal(snapshots.length, 3);
  assert.equal(
    snapshots[2].url,
    'https://chatgpt.com/g/g-p-family/c/project-voice-chat'
  );

  let resolveSingleFlight;
  let singleFlightCalls = 0;
  const singleFlightSnapshots = [];
  const singleFlight = createContext(async () => {
    singleFlightCalls += 1;
    if (singleFlightCalls === 1) return jsonResponse(detailPayload);
    return new Promise((resolve) => { resolveSingleFlight = resolve; });
  });
  await singleFlight.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  singleFlight.window.location.pathname = '/c/voice-chat';
  const singleFlightTransport = singleFlight.window.__elonChatGptPrivateTransport;
  assert.equal(singleFlightTransport.refreshCurrentConversation(
    '/c/voice-chat',
    (event) => singleFlightSnapshots.push(event)
  ), true);
  assert.equal(singleFlightTransport.refreshCurrentConversation(
    '/c/voice-chat',
    () => assert.fail('a duplicate refresh must reuse the active request')
  ), true);
  await Promise.resolve();
  await Promise.resolve();
  assert.equal(singleFlightCalls, 2);
  resolveSingleFlight(jsonResponse(detailPayload));
  await flush();
  assert.equal(singleFlightCalls, 2);
  assert.equal(singleFlightSnapshots.length, 1);

  let failedNavigation = 0;
  let failedCalls = 0;
  const failed = createContext(async () => {
    failedCalls += 1;
    if (failedCalls === 1) return jsonResponse(detailPayload);
    throw new Error('offline');
  });
  await failed.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  assert.equal(failed.window.__elonChatGptPrivateTransport.prefetchConversation(
    '/c/plain-chat',
    () => assert.fail('failed prefetch must not emit a snapshot'),
    () => { failedNavigation += 1; }
  ), true);
  await flush();
  assert.equal(failedNavigation, 1);
  assert.equal(failed.window.__elonChatGptPrivateTransport.health().failures, 1);
  assert.equal(failed.outcomes[0].outcome, 'network');

  const wrappedSnapshots = [];
  const wrapped = createContext(async () => jsonResponse({ data: { conversation: detailPayload } }));
  await wrapped.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  assert.equal(wrapped.window.__elonChatGptPrivateTransport.prefetchConversation(
    '/c/wrapped-chat',
    (event) => wrappedSnapshots.push(event),
    () => {}
  ), true);
  await flush();
  assert.equal(wrappedSnapshots.length, 1);
  assert.equal(wrappedSnapshots[0].messages.length, 2);

  const linearPayload = {
    title: 'Linear title',
    linear_conversation: [
      detailPayload.mapping['user-node'].message,
      detailPayload.mapping['assistant-node'].message
    ]
  };
  const linearSnapshots = [];
  const linear = createContext(async () => jsonResponse(linearPayload));
  await linear.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  assert.equal(linear.window.__elonChatGptPrivateTransport.prefetchConversation(
    '/c/linear-chat',
    (event) => linearSnapshots.push(event),
    () => {}
  ), true);
  await flush();
  assert.equal(linearSnapshots.length, 1);
  assert.equal(linearSnapshots[0].messages.length, 2);
  assert.equal(failed.window.__elonChatGptPrivateTransport.conversationPrefetchReady(), false);

  const membershipPayload = Object.assign({}, detailPayload, { gizmo_id: 'g-p-destination' });
  const membershipResults = [];
  const membershipRequests = [];
  const membership = createContext(async (url, options) => {
    membershipRequests.push({ url, options });
    return jsonResponse(membershipPayload);
  });
  await membership.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  assert.equal(membership.window.__elonChatGptPrivateTransport.probeConversationProject(
    '/g/g-p-origin/c/moved-chat',
    'g-p-destination',
    (matched) => membershipResults.push(matched)
  ), true);
  await flush();
  assert.deepEqual(membershipResults, [true]);
  assert.deepEqual(membership.membershipAccepts, [{
    id: 'moved-chat',
    title: 'Visible title',
    projectId: 'g-p-destination'
  }]);
  assert.equal(membershipRequests.length, 2);
  assert.equal(membershipRequests[1].options.cache, 'no-store');
  assert.equal(
    membershipRequests[1].options.__elonPrivateTransport,
    'conversation_membership'
  );

  const scopedMembershipResults = [];
  const scopedMembership = createContext(async () => jsonResponse(Object.assign({}, detailPayload, {
    context_scopes: [{ scope_type: 'project', scope_id: 'g-p-destination' }]
  })));
  await scopedMembership.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  assert.equal(scopedMembership.window.__elonChatGptPrivateTransport.probeConversationProject(
    '/g/g-p-origin/c/moved-chat',
    'g-p-destination',
    (matched) => scopedMembershipResults.push(matched)
  ), true);
  await flush();
  assert.deepEqual(scopedMembershipResults, [true]);

  const templateMembershipResults = [];
  const templateMembership = createContext(async () => jsonResponse(Object.assign({}, detailPayload, {
    conversation_template_id: 'g-p-destination'
  })));
  await templateMembership.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  assert.equal(templateMembership.window.__elonChatGptPrivateTransport.probeConversationProject(
    '/g/g-p-origin/c/moved-chat',
    'g-p-destination',
    (matched) => templateMembershipResults.push(matched)
  ), true);
  await flush();
  assert.deepEqual(templateMembershipResults, [true]);

  const directoryMembershipResults = [];
  const directoryMembership = createContext(
    async () => jsonResponse(detailPayload),
    true,
    true,
    new MemoryStorage(),
    [{ id: 'moved-chat', projectId: 'g-p-destination' }]
  );
  await directoryMembership.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  assert.equal(directoryMembership.window.__elonChatGptPrivateTransport.probeConversationProject(
    '/g/g-p-origin/c/moved-chat',
    'g-p-destination',
    (matched) => directoryMembershipResults.push(matched)
  ), true);
  await flush();
  assert.deepEqual(directoryMembershipResults, [true]);

  assert.equal(membership.window.__elonChatGptPrivateTransport.probeConversationProject(
    '/g/g-p-origin/c/moved-chat',
    'not-a-project',
    () => assert.fail('invalid membership probes must not dispatch')
  ), false);

  console.log('CHATGPT_WEB_PRIVATE_TRANSPORT_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
