const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const ts = require('typescript')

const sourcePath = path.join(__dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentSections.ts')
const source = fs.readFileSync(sourcePath, 'utf8')
const output = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const loaded = { exports: {} }
new Function('module', 'exports', 'require', output)(loaded, loaded.exports, require)

const {
  buildDocumentSections,
  buildOrganizationPrompt,
  parseOrganizationSuggestions,
  parseSectionManifest,
  sectionForDocument,
} = loaded.exports

function document(pathName, role, lifecycle = 'active', ambiguous = false) {
  return {
    path: pathName,
    title: pathName,
    size_bytes: 10,
    source: 'workspace',
    metadata: {
      role,
      lifecycle,
      authority: 'test',
      scope: 'project',
      default_retrieval: role === 'router',
      ambiguous,
      confidence: ambiguous ? 'low' : 'high',
      reason: 'test',
      token_estimate: 3,
      content_hash: 'abc',
      headings: [],
    },
  }
}

const manifest = parseSectionManifest(JSON.stringify({
  version: 1,
  sections: [{ id: 'research', label: '研究', detail: '研究笔记', color: '#123456' }],
  assignments: { 'docs/research.md': 'custom:research' },
}))
const documents = [
  document('AGENTS.md', 'router'),
  document('.github/agents/reviewer.agent.md', 'agent_definition'),
  document('README.md', 'guide'),
  document('docs/discussion.md', 'discussion', 'draft'),
  document('docs/unknown.md', 'note', 'unclassified', true),
  document('docs/research.md', 'note', 'unclassified', true),
]
const sections = documents.map((entry) => sectionForDocument(entry, manifest))
assert.deepEqual(sections, [
  'required',
  'customizations',
  'on-demand',
  'drafts',
  'unclassified',
  'custom:research',
])
assert.equal(sections.length, documents.length, '每份文档必须只属于一个分区')
assert(buildDocumentSections(manifest).some((section) => section.key === 'custom:research'))
assert(buildDocumentSections(manifest).some((section) => section.key === 'suggestions'))

const suggestions = parseOrganizationSuggestions(JSON.stringify({
  version: 1,
  status: 'ready',
  summary: 'ok',
  proposed_sections: [{ id: 'api', label: 'API', detail: '接口', color: '#abcdef' }],
  assignments: [{ path: 'docs/unknown.md', section_id: 'custom:api', reason: '归类' }],
  conflicts: [],
  move_suggestions: [],
  documents_read: 1,
  estimated_tokens_used: 20,
}))
assert.equal(suggestions.status, 'ready')
assert.equal(suggestions.assignments.length, 1)

const boundedSuggestions = parseOrganizationSuggestions(JSON.stringify({
  status: 'ready',
  proposed_sections: Array.from({ length: 12 }, (_, index) => ({
    id: `section-${index}`, label: `分区 ${index}`, color: '#123456',
  })),
  assignments: Array.from({ length: 510 }, (_, index) => ({
    path: `docs/${index}.md`, section_id: 'current', reason: 'test',
  })),
}))
assert.equal(boundedSuggestions.proposed_sections.length, 8)
assert.equal(boundedSuggestions.assignments.length, 500)

const catalog = {
  project_id: 'test', workspace: 'test', revision: '1', source: 'workspace', documents,
  warnings: [], can_edit: true,
  budget: {
    classification_model_tokens: 0,
    estimated_full_read_tokens: 18,
    estimated_default_retrieval_tokens: 3,
    estimated_tokens_avoided: 15,
    ambiguous_documents: 2,
    excluded_by_default: 5,
  },
}
const prompt = buildOrganizationPrompt('测试项目', catalog, manifest)
assert(prompt.includes('.elon/document-organization-suggestions.json'))
assert(prompt.includes('classification_model_tokens=0'))
assert(!prompt.includes('# Reviewer'))

const workspaceSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentsWorkspace.tsx',
), 'utf8')
assert(!workspaceSource.includes('markSuggestionsRequested'), '启动 AI 前不得在主工作区预写建议占位文件')
assert(workspaceSource.includes('onStartAiOrganize(buildOrganizationPrompt'))

console.log('project document section model tests passed')
