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
  return { ok: true, status: 200, text: async () => JSON.stringify(value) };
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
  authContext = null,
  clock = Date
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
    __elonChatGptPrivateJsonRequest: require('../android/app/src/main/assets/chatgpt_web_private_json_request.js'),
    AbortController,
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
    Date: clock,
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
  for (const asset of ['chatgpt_web_private_history_projection.js', 'chatgpt_web_private_delta_document.js', 'chatgpt_web_private_stream_policy.js']) {
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
  assert.equal(gated.window.__elonChatGptPrivateTransport.version, 29);
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
  assert.equal(transport.version, 29);
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

  const explicitMembershipPayload = { ...detailPayload, gizmo_id: 'g-p-destination' };
  const coldMembershipReads = [];
  let coldAcquires = 0;
  const coldMembership = createContext(async (url, options) => {
    coldMembershipReads.push({ url, options });
    return jsonResponse(explicitMembershipPayload);
  }, false, true, new MemoryStorage(), [], {
    canAcquire: () => true,
    acquireRequestHeaders: async () => { coldAcquires++; return { Authorization: 'synthetic-identity' }; }
  });
  const explicitReader = coldMembership.window.__elonChatGptPrivateTransport;
  assert.equal(explicitReader.conversationPrefetchReady(), false);
  const coldMatches = [];
  assert.equal(explicitReader.probeConversationProject('/c/new-project-chat', 'g-p-destination',
    matched => coldMatches.push(matched)), true, 'explicit membership can acquire identity without a prior official read');
  assert.equal(explicitReader.probeConversationProject('/c/new-project-chat', 'g-p-destination',
    matched => coldMatches.push(matched)), true);
  await flush();
  assert.deepEqual(coldMatches, [true, true]);
  assert.equal(coldAcquires, 1, 'duplicate membership queries retain one identity acquisition');
  assert.equal(coldMembershipReads.length, 1, 'duplicate queries retain one fresh read');
  assert.equal(coldMembershipReads[0].options.method, 'GET');
  assert.equal(coldMembershipReads[0].options.cache, 'no-store');
  assert.equal(coldMembershipReads[0].options.body, undefined);
  assert.equal(explicitReader.conversationPrefetchReady(), false, 'explicit reads do not manufacture fresh official observations');

  let membershipNow = Date.now();
  class MembershipClock extends Date { static now() { return membershipNow; } }
  const staleReads = [];
  let membershipStatus = 200;
  const staleMembership = createContext(async (url, options) => {
    staleReads.push({ url, options });
    return membershipStatus === 200 ? jsonResponse(explicitMembershipPayload) :
      { ok: false, status: membershipStatus, text: async () => '{}' };
  }, false, true, new MemoryStorage(), [], null, MembershipClock);
  await staleMembership.window.fetch('/backend-api/conversations/observed', {
    headers: { Authorization: 'synthetic-observed-identity' }
  });
  membershipNow += 120001;
  const staleReader = staleMembership.window.__elonChatGptPrivateTransport;
  assert.equal(staleReader.conversationPrefetchReady(), false);
  const staleMatches = [];
  assert.equal(staleReader.probeConversationProject('/c/new-project-chat', 'g-p-destination',
    matched => staleMatches.push(matched)), true, 'expired background freshness does not block explicit membership');
  await flush();
  assert.deepEqual(staleMatches, [true]);
  assert.equal(staleReads.length, 2);
  assert.equal(staleReader.conversationPrefetchReady(), false);
  membershipStatus = 401;
  assert.equal(staleReader.probeConversationProject('/c/new-project-chat', 'g-p-destination',
    matched => staleMatches.push(matched)), true);
  await flush();
  assert.deepEqual(staleMatches, [true, false]);
  assert.equal(staleReader.health().lastOutcome, 'auth');
  assert.equal(staleReader.probeConversationProject('/c/new-project-chat', 'g-p-destination',
    () => assert.fail('cooldown must not dispatch')), false);
  assert.equal(staleReads.length, 3, 'failed explicit read is not replayed');
  assert.equal(detail.window.__elonChatGptPrivateTransport.probeConversationProject('/c/valid', 'invalid',
    () => assert.fail('invalid project must not dispatch')), false);
  assert.equal(gated.window.__elonChatGptPrivateTransport.probeConversationProject('/c/valid', 'g-p-destination',
    () => assert.fail('disabled private reader must not dispatch')), false);
  const missingIdentity = createContext(() => assert.fail('missing identity must not fetch'), false, true);
  assert.equal(missingIdentity.window.__elonChatGptPrivateTransport.probeConversationProject('/c/valid', 'g-p-destination',
    () => assert.fail('missing identity must not dispatch')), false);

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

  const attachmentId = '00000000-0000-4000-8000-000000000001';
  const attachmentPath = '/c/' + attachmentId;
  const ordinaryPayload = { ...detailPayload, conversation_id: attachmentId,
    gizmo_id: null, is_do_not_remember: false };
  const scopeRequests = [];
  let scopePayload = ordinaryPayload;
  const scope = createContext(async (url, options) => {
    scopeRequests.push({ url, options });
    return jsonResponse(scopePayload);
  }, false, true, new MemoryStorage(), [], {
    canAcquire: () => true,
    acquireRequestHeaders: async () => ({ Authorization: 'Bearer synthetic-token' }),
  });
  scope.window.location.pathname = attachmentPath;
  scope.window.location.href = 'https://chatgpt.com' + attachmentPath;
  const reader = scope.window.__elonChatGptPrivateTransport;
  assert.equal((await reader.readAttachmentContext(attachmentPath)).ordinary, true);
  assert.equal(scopeRequests.length, 1);
  assert.equal(scopeRequests[0].url, '/backend-api/conversations/' + attachmentId);
  assert.equal(scopeRequests[0].options.method, 'GET');
  assert.equal(scopeRequests[0].options.cache, 'no-store');
  assert.equal(scopeRequests[0].options.body, undefined);
  for (const patch of [{ gizmo_id: 'g-p-project' }, { is_do_not_remember: true },
    { project_id: 'g-p-conflicting' }, { context_scopes: [{ scope_type: 'project', scope_id: 'g-p-other' }] }]) {
    scopePayload = { ...ordinaryPayload, ...patch };
    assert.equal((await reader.readAttachmentContext(attachmentPath)).ordinary, false);
  }
  for (const patch of [{}, { is_temporary_chat: true }, { is_temporary_chat: false },
    { gizmo_id: 'g-p-project' }, { project_id: 'g-p-conflicting' }, { context_scopes: ['HEALTH'] }]) {
    scopePayload = { ...ordinaryPayload, is_do_not_remember: true, ...patch };
    const receipt = await reader.readAttachmentContext(attachmentPath);
    assert.equal(receipt.ordinary, false);
    assert.equal(receipt.temporary, !patch.gizmo_id && !patch.project_id && !patch.context_scopes && patch.is_temporary_chat !== false);
    assert.equal(Object.hasOwn(receipt, 'mapping'), false);
  }
  scopePayload = { ...ordinaryPayload, is_temporary_chat: true };
  assert.equal((await reader.readAttachmentContext(attachmentPath)).ordinary, false);
  for (const patch of [{ conversation_id: 'wrong' }, { id: 'conflicting' }, { gizmo_id: undefined },
    { is_do_not_remember: undefined }, { is_do_not_remember: 'false' }, { is_temporary_chat: 'true' }]) {
    scopePayload = { ...ordinaryPayload, ...patch };
    await assert.rejects(reader.readAttachmentContext(attachmentPath), /attachment_context_unavailable/);
    assert.equal(reader.health().cooldownRemainingMs, 0, 'unknown upload scope cannot break history prefetch');
  }
  const attachmentProject = 'g-p-0123456789abcdef0123456789abcdef';
  const attachmentLeaf = '11111111-2222-4333-8444-555555555555';
  const differentLeaf = '99999999-2222-4333-8444-555555555555';
  const projectPayload = { ...ordinaryPayload, gizmo_id: attachmentProject, current_node: differentLeaf,
    mapping: { [attachmentLeaf]: { id: attachmentLeaf },
      [differentLeaf]: { id: differentLeaf }, invalid: { id: 'invalid' },
      [attachmentId]: { id: differentLeaf } } };
  for (const projectPath of [attachmentPath, '/g/' + attachmentProject + '-synthetic' + attachmentPath]) {
    scope.window.location.pathname = projectPath;
    scope.window.location.href = 'https://chatgpt.com' + projectPath;
    scopePayload = projectPayload;
    const receipt = await reader.readAttachmentContext(projectPath);
    assert.equal(receipt.projectId, attachmentProject);
    assert.deepEqual(Array.from(receipt.nodeIds), [attachmentLeaf, differentLeaf]);
    assert.equal(receipt.ordinary, false);
    assert.equal(receipt.temporary, false);
    assert.equal(Object.hasOwn(receipt, 'mapping'), false);
    assert.equal(Object.hasOwn(receipt, 'current_node'), false, 'server current_node is not the UI branch');
    assert.equal(Object.isFrozen(receipt.nodeIds), true);
    assert.equal(scopeRequests.at(-1).options.cache, 'no-store');
  }
  scope.window.location.pathname = attachmentPath;
  scope.window.location.href = 'https://chatgpt.com' + attachmentPath;
  // The current official PHt reader builds mapping from this paginated Message[].
  scopePayload = { ...projectPayload, mapping: undefined, messages: [
    { id: attachmentLeaf, author: { role: 'user' }, content: { parts: ['synthetic'] } },
    { id: differentLeaf, author: { role: 'assistant' } }, { id: 'invalid' }, null] };
  const paginatedScope = await reader.readAttachmentContext(attachmentPath);
  assert.deepEqual(Array.from(paginatedScope.nodeIds), [attachmentLeaf, differentLeaf]);
  assert.equal(paginatedScope.projectId, attachmentProject);
  assert.equal(JSON.stringify(paginatedScope).includes('synthetic'), false);
  scopePayload = { ...projectPayload, mapping: undefined, messages: Array(20001).fill({ id: attachmentLeaf }) };
  assert.deepEqual(Array.from((await reader.readAttachmentContext(attachmentPath)).nodeIds), []);
  for (const patch of [{ project_id: 'g-p-fedcba9876543210fedcba9876543210' },
    { is_do_not_remember: true }, { is_temporary_chat: true }, { context_scopes: ['HEALTH'] }]) {
    scopePayload = { ...projectPayload, ...patch };
    assert.equal((await reader.readAttachmentContext(attachmentPath)).projectId, undefined);
  }
  for (const mapping of [null, [], {}, { invalid: { id: 'invalid' } }]) {
    scopePayload = { ...projectPayload, mapping };
    assert.deepEqual(Array.from((await reader.readAttachmentContext(attachmentPath)).nodeIds), []);
  }
  scopePayload = { data: { conversation: ordinaryPayload } };
  assert.equal((await reader.readAttachmentContext(attachmentPath)).ordinary, true);
  const beforeInvalid = scopeRequests.length;
  for (const value of ['/c/invalid', '/g/g-p-fixture' + attachmentPath, attachmentPath + '?temporary-chat=true']) {
    await assert.rejects(reader.readAttachmentContext(value), /attachment_context_unavailable/);
  }
  scope.window.location.pathname = '/';
  await assert.rejects(reader.readAttachmentContext(attachmentPath), /attachment_context_unavailable/);
  assert.equal(scopeRequests.length, beforeInvalid);

  for (const backgroundFailure of ['timeout', 'network', 'parse', 'empty', 'http']) {
    const isolatedStorage = new MemoryStorage();
    const module = require('../android/app/src/main/assets/chatgpt_web_private_transport_policy.js');
    const background = module.create({ enabled: true, storage: isolatedStorage });
    background.recordFailure(backgroundFailure);
    let reads = 0;
    const isolated = createContext(async () => { reads++; return jsonResponse(ordinaryPayload); },
      false, true, isolatedStorage, [], { canAcquire: () => true,
        acquireRequestHeaders: async () => ({ Authorization: 'synthetic-test-identity' }) });
    const owner = isolated.window.__elonChatGptPrivateTransport;
    const receipts = [], indices = [];
    await owner.listConversationFiles(attachmentPath, 'mcp_isolated', value => indices.push(value),
      (...values) => receipts.push(values));
    assert.deepEqual(receipts, [['list_conversation_files', true, 'private_files_ready']]);
    assert.equal(indices.length, 1);
    assert.equal(reads, 1, 'explicit read dispatches once during background cooldown');
    assert.equal(owner.health().lastOutcome, backgroundFailure, 'successful user read does not clear background protection');
    assert.equal(owner.accountReadHealth().lastOutcome, 'success');
    isolated.window.location.pathname = attachmentPath;
    assert.equal((await owner.readAttachmentContext(attachmentPath)).ordinary, true);
    assert.equal(reads, 2, 'attachment scope is still force-read rather than cached');
  }
  for (const rejection of [401, 403, 429]) {
    let reads = 0;
    const rejected = createContext(async () => {
      reads++;
      return { ok: false, status: rejection, text: async () => '{}' };
    }, false, true, new MemoryStorage(), [], { canAcquire: () => true,
      acquireRequestHeaders: async () => ({ Authorization: 'synthetic-rejected-identity' }) });
    const owner = rejected.window.__elonChatGptPrivateTransport;
    const receipts = [];
    await owner.listConversationFiles(attachmentPath, 'mcp_rejected', () => assert.fail('no index on rejection'),
      (...values) => receipts.push(values));
    const expected = rejection === 429 ? 'rate_limit' : 'auth';
    assert.equal(owner.health().lastOutcome, expected);
    assert.equal(owner.accountReadHealth().lastOutcome, expected);
    await owner.listConversationFiles(attachmentPath, 'mcp_repeated', () => assert.fail('no index on cooldown'),
      (...values) => receipts.push(values));
    assert.equal(receipts.at(-1)[2], 'files_read_cooldown');
    assert.equal(reads, 1, 'auth or rate-limit failures cannot be automatically replayed');
  }
  const pendingReceipts = [];
  let releaseIdentity;
  const pendingIdentity = createContext(async () => jsonResponse(ordinaryPayload), false, true,
    new MemoryStorage(), [], { canAcquire: () => false,
      acquireRequestHeaders: () => new Promise(resolve => { releaseIdentity = resolve; }) });
  const pendingRead = pendingIdentity.window.__elonChatGptPrivateTransport.listConversationFiles(
    attachmentPath, 'mcp_pending', () => {}, (...values) => pendingReceipts.push(values));
  assert.equal(pendingReceipts.length, 0, 'identity acquisition must not produce premature failure');
  releaseIdentity({ Authorization: 'synthetic-ready-identity' });
  await pendingRead;
  assert.equal(pendingReceipts[0][2], 'private_files_ready');
  let cooling = true;
  const identityRecovery = createContext(async () => jsonResponse(ordinaryPayload), false, true,
    new MemoryStorage(), [], { canAcquire: () => !cooling,
      acquireRequestHeaders: async () => {
        if (cooling) throw new Error('auth_cooldown');
        return { Authorization: 'synthetic-recovered-identity' };
      } });
  const recoveryReader = identityRecovery.window.__elonChatGptPrivateTransport;
  const recoveryReceipts = [];
  const readRecovered = () => recoveryReader.listConversationFiles(attachmentPath, 'mcp_recovery',
    () => {}, (...args) => recoveryReceipts.push(args));
  await readRecovered();
  assert.equal(recoveryReceipts[0][2], 'files_identity_unavailable');
  assert.equal(recoveryReader.accountReadHealth().cooldownRemainingMs, 0,
    'short identity preparation cooldown must not turn into a five-minute transport failure');
  cooling = false;
  await readRecovered();
  assert.equal(recoveryReceipts[1][2], 'private_files_ready');

  console.log('CHATGPT_WEB_PRIVATE_TRANSPORT_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
