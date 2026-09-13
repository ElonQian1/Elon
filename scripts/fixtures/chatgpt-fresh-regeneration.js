'use strict';
const { fixture: freshContext, CID, PID } = require('./chatgpt-fresh-text-context');
const { fixture: runtimeRetry } = require('./chatgpt-runtime-regeneration');
const regeneration = require('../../android/app/src/main/assets/chatgpt_web_fresh_regenerate_context');
const contract = require('../../android/app/src/main/assets/chatgpt_web_private_regenerate_contract');
const UID = '44444444-4444-4444-8444-444444444444';
const ROOT = '55555555-5555-4555-8555-555555555555';
const AID = '66666666-6666-4666-8666-666666666666';
const OTHER = '77777777-7777-4777-8777-777777777777';

function fixture() {
  const f = freshContext(), r = runtimeRetry();
  const { page, shared, tree, selected, parent, conversation } = f;
  Object.assign(parent, { content: { content_type: 'text', parts: ['Synthetic previous answer'] },
    metadata: { thinking_effort: 'high' } });
  const user = { id: UID, parent: ROOT, message: { id: UID, author: { role: 'user' },
    content: { content_type: 'text', parts: ['Synthetic original prompt'] }, metadata: {} } };
  tree.leaf = PID; tree.variants = [PID];
  tree.nodes = { [ROOT]: { id: ROOT, parent: null, message: { id: ROOT, author: { role: 'root' } } },
    [UID]: user, [PID]: { id: PID, parent: UID, message: parent } };
  Object.assign(shared.HM, {
    getCurrentLeafId: t => t.leaf, getCurrentMessage: t => t.nodes[t.leaf]?.message,
    getVariantIds: t => t.variants, getNode: (t, id) => t.nodes[id], getNodeIfExists: (t, id) => t.nodes[id],
    getParentNode: (t, id) => t.nodes[t.nodes[id]?.parent],
    getParentPromptNode(t, id) {
      const seen = new Set();
      while (id && !seen.has(id)) {
        seen.add(id); const node = t.nodes[id];
        if (node?.message?.author?.role === 'user') return node;
        id = node?.parent;
      }
      return null;
    }
  });
  for (const key of ['H3', 'F5', 'mq']) shared[key] = r.modules.shared[key];
  shared.canvasQueryClient = () => ({});
  conversation.f8t = r.modules.conversation.f8t;
  conversation.textResolveRequestedModel = ({ requestedModelId }) => requestedModelId;
  Object.assign(r.menu, { conversation: selected, lastMessage: parent });
  Object.assign(r.menuRoot.return.memoizedProps, { conversation: selected, lastMessage: parent });
  r.menu.retryOption.value = 'fixture-model';
  r.modelMenu.conversation = selected;
  r.modelMenu.modelsData.models.set('fixture-model', { id: 'fixture-model', defaultThinkingEffort: 'low' });
  Object.assign(page, {
    __elonChatGptPrivateModelContract: r.page.__elonChatGptPrivateModelContract,
    __elonChatGptPrivateRegenerateContract: contract,
    __elonChatGptFreshRegenerateContext: regeneration,
    __elonChatGptPrivateTransport: r.page.__elonChatGptPrivateTransport,
    performance: r.page.performance
  });
  page.document.querySelector = () => null;
  page.__elonChatGptPrivateRuntimeBindings.observed = () => true;
  const command = { ...r.command, requestId: 'mcp_regen1', composer: f.node,
    prompt: '', expectedDraft: '', readDraft: () => '' };
  const reply = (id = AID, status = 'finished_successfully') => ({ id, author: { role: 'assistant' }, status,
    end_turn: status === 'finished_successfully', content: { content_type: 'text', parts: ['Synthetic regenerated answer'] } });
  function history(id = AID) {
    return { conversation_id: CID, current_node: id, async_status: null,
      mapping: { ...tree.nodes, [id]: { id, parent: UID, message: reply(id) } } };
  }
  function apply(value) { tree.nodes = value.mapping; tree.leaf = value.current_node; f.props.currentLeafId = tree.leaf; }
  const api = regeneration.create(page, f.api);
  return Object.assign(f, { retry: r, user, command, reply, history, apply, capture: () => api.capture(command) });
}

module.exports = { fixture, CID, PID, UID, ROOT, AID, OTHER };
