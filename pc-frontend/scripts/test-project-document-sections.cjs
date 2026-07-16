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
  assignments: { 'docs/research.md': 'custom:research', 'docs/discussion.md': 'drafts' },
  governance_overrides: { 'docs/unknown.md': 'on-demand' },
}))
assert.equal(manifest.assignments['docs/research.md'], 'custom:research')
assert.equal(manifest.assignments['docs/discussion.md'], undefined)
assert.equal(manifest.governance_overrides['docs/discussion.md'], 'drafts', '旧清单的治理归类应自动迁移')
assert.equal(manifest.governance_overrides['docs/unknown.md'], 'on-demand')
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
  'on-demand',
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
  file_operations: [{
    id: 'rename-unknown', kind: 'rename', source_path: 'docs/unknown.md',
    target_path: 'docs/unknown-topic.md', source_revision: 'abc', reason: '名称更可检索',
  }],
  documents_read: 1,
  estimated_tokens_used: 20,
}))
assert.equal(suggestions.status, 'ready')
assert.equal(suggestions.assignments.length, 1)
assert.equal(suggestions.file_operations.length, 1)
assert.equal(suggestions.file_operations[0].status, 'proposed')

const boundedSuggestions = parseOrganizationSuggestions(JSON.stringify({
  status: 'ready',
  proposed_sections: Array.from({ length: 12 }, (_, index) => ({
    id: `section-${index}`, label: `分区 ${index}`, color: '#123456',
  })),
  assignments: Array.from({ length: 510 }, (_, index) => ({
    path: `docs/${index}.md`, section_id: 'current', reason: 'test',
  })),
}))
assert.equal(boundedSuggestions.proposed_sections.length, 12)
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
assert(prompt.includes('project_docs_analyze'))
assert(prompt.includes('project_docs_get_status'))
assert(prompt.includes('权限模式：git_backed_full'))
assert(prompt.includes('authorization_mode=git_backed_full'))
assert(prompt.includes('git_baseline_commit'))
assert(prompt.includes('git_result_commit'))
assert(prompt.includes('project_docs_apply_file_operations'))
assert(prompt.includes('source_revision'))
assert(prompt.includes('knowledge_architecture'))
assert(prompt.includes('主题知识树'))
assert(prompt.includes('不得越界、操作非 Markdown、修改代码或自动 push'))
assert(prompt.length < 3000, '整理任务 Prompt 不应内嵌完整文档目录')
assert(!prompt.includes('# Reviewer'))

const architectureSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentArchitecture.ts',
), 'utf8')
assert(architectureSource.includes("DocumentNavigationMode = 'knowledge' | 'governance'"))
assert(architectureSource.includes("key: 'software-platform'"))
assert(architectureSource.includes('analyzeKnowledgeArchitecture'))
assert(architectureSource.includes('topicSectionForDocument'))
assert(architectureSource.includes('const topics = [...templateSections, ...customSections]'))

const workspaceSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentsWorkspace.tsx',
), 'utf8')
assert(!workspaceSource.includes('markSuggestionsRequested'), '启动 AI 前不得在主工作区预写建议占位文件')
assert(workspaceSource.includes('onStartAiOrganize(buildOrganizationPrompt'))
assert(workspaceSource.includes('ProjectDocumentAccessNotice'))
assert(workspaceSource.includes('applyFileOperations'))
assert(workspaceSource.includes('organization.trace?.catalog_revision'))

const fileOperationsSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentFileOperations.tsx',
), 'utf8')
assert(fileOperationsSource.includes('已开放安全可恢复权限'))
assert(fileOperationsSource.includes('不覆盖、不删除、不改正文'))

const policySource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentAutomationPolicy.ts',
), 'utf8')
assert(policySource.includes("DEFAULT_DOCUMENT_AUTOMATION_MODE: DocumentAutomationMode = 'git_backed_full'"))
assert(policySource.includes("value: 'review_all'"))
assert(policySource.includes("value: 'suggestions_only'"))

const statusSourcePath = path.join(__dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentOrganizationStatus.ts')
const statusOutput = ts.transpileModule(fs.readFileSync(statusSourcePath, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const statusLoaded = { exports: {} }
new Function('module', 'exports', 'require', statusOutput)(statusLoaded, statusLoaded.exports, require)
const trace = statusLoaded.exports.parseDocumentOrganizationTrace({
  version: 1,
  operation_id: 'docs_test',
  status: 'running',
  current_stage: 'catalog_analyzed',
  created_at: 1,
  updated_at: 2,
  documents_cataloged: 119,
  ambiguous_documents: 57,
  documents_read: 3,
  estimated_tokens_used: 240,
  events: [{ stage: 'catalog_analyzed', status: 'running', label: '目录分析完成', detail: 'metadata only', at: 2 }],
})
assert.equal(trace.documents_cataloged, 119)
assert.equal(statusLoaded.exports.shouldPollDocumentOrganization(trace), true)
assert.equal(statusLoaded.exports.shouldPollDocumentOrganization({ ...trace, status: 'failed' }), false)

const channelSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentsChannel.tsx',
), 'utf8')
assert(channelSource.includes('/organization/start') === false, '状态 API 应封装在 organization hook 中')
assert(!channelSource.includes('await projectStore.selectChannel(aiChannel.id)'), '发起整理后应停留在文档工作台观察进度')

console.log('project document section model tests passed')
