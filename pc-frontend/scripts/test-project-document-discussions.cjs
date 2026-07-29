const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const ts = require('typescript')

const modelPath = path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentDiscussionModel.ts',
)
const output = ts.transpileModule(fs.readFileSync(modelPath, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const loaded = { exports: {} }
new Function('module', 'exports', 'require', output)(loaded, loaded.exports, require)

const {
  discussionRoots,
  layoutDiscussionNodes,
  parseDiscussionGraph,
  selectDiscussionSubgraph,
} = loaded.exports

const graph = parseDiscussionGraph(JSON.stringify({
  version: 1,
  sources: [{
    id: 'chat',
    title: '开放商业讨论',
    kind: 'chat',
    reference: 'docs/inbox/conversations/chat.md',
    source_format: 'chatgpt-export',
    message_count: 36,
    chunk_count: 4,
    processed_chunk_ids: ['chunk-0001', 'chunk-0002'],
    compilation_status: 'partial',
  }],
  nodes: [
    { id: 'root', title: '开放商业网络', kind: 'topic', status: 'exploring', source_refs: ['chat#1'] },
    { id: 'option-a', root_id: 'root', parent_id: 'root', title: '商户 AI', kind: 'option', status: 'accepted', source_refs: ['chat#2'] },
    { id: 'risk-a', root_id: 'root', parent_id: 'option-a', title: '巨头复制', kind: 'risk', status: 'open', source_refs: ['chat#3'] },
    { id: 'option-b', root_id: 'root', parent_id: 'root', title: '开放协议', kind: 'option', status: 'exploring', source_refs: ['chat#4'] },
  ],
  edges: [
    { id: 'root-a', source: 'root', target: 'option-a', relation: 'spawns' },
    { id: 'a-b', source: 'option-a', target: 'option-b', relation: 'alternative_to' },
  ],
}))

assert.equal(graph.sources.length, 1)
assert.equal(graph.sources[0].chunk_count, 4)
assert.deepEqual(graph.sources[0].processed_chunk_ids, ['chunk-0001', 'chunk-0002'])
assert.equal(graph.sources[0].compilation_status, 'partial')
assert.equal(graph.nodes.length, 4)
assert.equal(discussionRoots(graph)[0].id, 'root')
assert.equal(selectDiscussionSubgraph(graph, 'option-a', '').nodes.length, 2)
const searched = selectDiscussionSubgraph(graph, 'root', '巨头')
assert.deepEqual(new Set(searched.nodes.map((node) => node.id)), new Set(['root', 'option-a', 'risk-a']))
assert.equal(searched.edges.length, 1)
const positions = layoutDiscussionNodes(graph.nodes)
assert(positions.get('risk-a').x > positions.get('option-a').x)

const invalid = parseDiscussionGraph('{broken')
assert.equal(invalid.nodes.length, 0)

const proposalPath = path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentDiscussionProposal.ts',
)
const proposalOutput = ts.transpileModule(fs.readFileSync(proposalPath, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const proposalModule = { exports: {} }
const proposalRequire = (request) => (
  request === './projectDocumentDiscussionModel' ? loaded.exports : require(request)
)
new Function('module', 'exports', 'require', proposalOutput)(
  proposalModule, proposalModule.exports, proposalRequire,
)
const { discussionProposalDiff, parseDiscussionProposal } = proposalModule.exports
const proposalGraph = JSON.parse(JSON.stringify(graph))
proposalGraph.sources[0].processed_chunk_ids.push('chunk-0003')
proposalGraph.sources[0].compilation_status = 'partial'
proposalGraph.nodes.push({
  id: 'feature-market',
  root_id: 'root',
  parent_id: 'option-b',
  title: '跨 App 商业能力调用',
  kind: 'feature',
  status: 'accepted',
  source_refs: ['chat#turn-0031'],
})
const proposal = parseDiscussionProposal(JSON.stringify({
  status: 'ready',
  summary: '新增跨 App 调用功能，并继续处理来源分块。',
  change_kind: 'extend',
  actor: 'codex-cli',
  graph: proposalGraph,
  promotions: [{
    id: 'promotion-feature-market',
    node_id: 'feature-market',
    path: 'docs/features/open-market.md',
    title: '开放商业网络',
    document_type: 'feature',
    section_id: 'product',
  }],
  documents_read: 3,
  estimated_tokens_used: 1860,
}))
assert(proposal)
assert.equal(proposal.promotions.length, 1)
assert.equal(proposal.documentsRead, 3)
const proposalDiff = discussionProposalDiff(graph, proposal)
assert.deepEqual(proposalDiff.newNodes.map((node) => node.id), ['feature-market'])
assert.deepEqual(proposalDiff.changedSources.map((source) => source.id), ['chat'])
assert.equal(parseDiscussionProposal('{"status":"draft"}'), null)

const workspaceSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentsWorkspace.tsx',
), 'utf8')
assert(workspaceSource.includes('ProjectDocumentDiscussionMap'))
assert(workspaceSource.includes('discussionSourceInstruction'))
assert(workspaceSource.includes('discussionNodeInstruction'))
const mapSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentDiscussionMap.tsx',
), 'utf8')
assert(mapSource.includes('ProjectDocumentDiscussionProposalPanel'))
assert(mapSource.includes('ProjectDocumentDiscussionSources'))

const promptsPath = path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentKnowledgeMapPrompts.ts',
)
const promptsOutput = ts.transpileModule(fs.readFileSync(promptsPath, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const promptsModule = { exports: {} }
new Function('module', 'exports', 'require', promptsOutput)(
  promptsModule, promptsModule.exports, require,
)
const sourcePrompt = promptsModule.exports.discussionSourceInstruction({
  path: 'docs/inbox/conversations/chat.md',
  source_id: 'conversation-test',
  source_revision: 'revision-test',
  source_format: 'conversation_json',
  message_count: 24,
})
assert(sourcePrompt.includes('manifest.source_revision'))
assert(sourcePrompt.includes('1 至 3 句话的可复用 summary'))
assert(sourcePrompt.includes('不能使用 proposed'))
assert(sourcePrompt.includes('根节点自身 root_id=id'))
assert(sourcePrompt.includes('decomposes_to'))
assert(sourcePrompt.includes('不能自造 contains'))

console.log('project document discussion graph tests passed')
