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
  sources: [{ id: 'chat', title: '开放商业讨论', kind: 'chat', reference: 'docs/inbox/conversations/chat.md' }],
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

const workspaceSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentsWorkspace.tsx',
), 'utf8')
assert(workspaceSource.includes('ProjectDocumentDiscussionMap'))
assert(workspaceSource.includes('discussionSourceInstruction'))
assert(workspaceSource.includes('discussionNodeInstruction'))

console.log('project document discussion graph tests passed')
