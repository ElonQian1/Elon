'use strict';
const contextModule = require('../../android/app/src/main/assets/chatgpt_web_fresh_text_context');
const CID = '11111111-1111-4111-8111-111111111111';
const PID = '22222222-2222-4222-8222-222222222222';
function fixture() {
  let identity = 'fixture-account';
  const node = { isConnected: true };
  const page = { document: {}, __elonChatGptDocumentToken: 'doc_fixture_context',
    location: { href: 'https://chatgpt.com/c/' + CID } };
  const selected = { id: 'fixture-local-thread', serverId$: () => CID, config: {} };
  const controller = { conversation: selected };
  const props = { conversation: selected, composerController: controller, isDisabled: false,
    isConsumerLockdownModeLoadingForConversation: false, shouldBlockConsumerLockdownModeActionsForConversation: false,
    structuredInputMessageId: null, currentLeafId: PID, isComposerSubmissionReady: false };
  const tree = { mode: { kind: 'primary_assistant' } };
  const parent = { id: PID, author: { role: 'assistant' }, status: 'finished_successfully', end_turn: true };
  const files = { files$: () => [], readyFiles$: () => [], hasUploadInProgress$: () => false };
  const hints = { locked: false, activeSystemHintType: null, activeConnectorSystemHintTypes: new Set(),
    activeCustomAgentSystemHintType: null, coldStartCampaignCreativeId: null };
  const shared = { wV: fn => fn(), SV: { isPersonalWorkspace: () => true },
    textApi: { safePost() {} }, textSecurityHeaders() {}, textHistoryDisabled: () => false,
    textModelOverride: () => ({ model_slug: 'another-model' }), XM: () => tree,
    HM: { getGizmoId: () => null, getRequestId: () => null, getCurrentMessage: () => parent },
    cX: () => false, uo: () => false, Fx: () => null, Fl: () => false, v7: { STREAMING: 3, UNREAD: 4 },
    canvasConversations: () => [selected] };
  const conversation = { textSecurity() {}, textStream() {}, textHydrateHistory() {},
    textPrepareEnabled: () => true, textReviewAck: () => null,
    Nrn: () => ({ id: 'fixture-model' }), yRt: () => ({ conversationThinkingEffort$: () => 'high' }),
    l0: () => ({ getServiceTierForSubmission$: () => 'standard' }) };
  const binding = { href: page.location.href, newThread: false, temporary: false, conversation: selected,
    controller, shared: { getSharedProps: () => props }, files, serverId: CID, token: page.__elonChatGptDocumentToken };
  page.__elonChatGptPrivateModelContract = { create: () => ({ withRuntimeIdentity: () => ({ account: identity }) }) };
  page.__elonChatGptPrivateTextRuntimeSubmit = { captureConversation: () => ({ ...binding, account: identity }), state: () => ({ pending: false }) };
  page.__elonChatGptPrivateRuntimeBindings = { state: () => ({ profile_id: 'web_20260912' }),
    peek: () => shared,
    load: async role => ({ shared, conversation, composer: { Ng: () => hints } })[role] };
  page.__elonChatGptPrivateComposerToolContext = { capture: () => ({ ...binding, document: page.document,
    account: identity, model: conversation.Nrn(selected).id, allowed: 'search,picture_v2' }) };
  return { page, node, selected, props, tree, parent, files, hints, shared, conversation, binding,
    identity: value => { identity = value; }, api: contextModule.create(page) };
}
module.exports = { fixture, CID, PID };
