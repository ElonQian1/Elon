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
  createCustomSection,
  customSectionKey,
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
  document_metadata: { 'docs/research.md': { order: 7, pinned: true } },
  audit_log: [{ id: 'one', action: 'test', target: 'docs/research.md', summary: '测试审计', at: '2026-07-17T00:00:00Z' }],
}))
assert.equal(manifest.assignments['docs/research.md'], 'custom:research')
assert.equal(manifest.assignments['docs/discussion.md'], undefined)
assert.equal(manifest.governance_overrides['docs/discussion.md'], 'drafts', '旧清单的治理归类应自动迁移')
assert.equal(manifest.governance_overrides['docs/unknown.md'], 'on-demand')
assert.equal(manifest.document_metadata['docs/research.md'].order, 7)
assert.equal(manifest.document_metadata['docs/research.md'].pinned, true)
assert.equal(manifest.audit_log[0].action, 'test')
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
assert(prompt.includes('project_docs_get_issues'))
assert(prompt.includes('权限模式：git_backed_full'))
assert(prompt.includes('authorization_mode=git_backed_full'))
assert(prompt.includes('git_baseline_commit'))
assert(prompt.includes('git_result_commit'))
assert(prompt.includes('project_docs_apply_file_operations'))
assert(prompt.includes('source_revision'))
assert(prompt.includes('document_health'))
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
assert(architectureSource.includes('serverArchitectureHealth'))
assert(architectureSource.includes("DOCUMENT_HEALTH_SECTION = 'document-health'"))
assert(architectureSource.includes('topicSectionForDocument'))
assert(architectureSource.includes('const topics = [...templateSections, ...customSections]'))
assert(architectureSource.includes("CAPABILITY_MAP_SECTION = 'capability-map'"))

const architectureOutput = ts.transpileModule(architectureSource, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const architectureLoaded = { exports: {} }
new Function('module', 'exports', 'require', architectureOutput)(
  architectureLoaded,
  architectureLoaded.exports,
  (request) => request === './projectDocumentSections' ? loaded.exports : require(request),
)

const capabilityGraphSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentCapabilityGraph.ts',
), 'utf8')
const capabilityGraphOutput = ts.transpileModule(capabilityGraphSource, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const capabilityGraphLoaded = { exports: {} }
new Function('module', 'exports', 'require', capabilityGraphOutput)(
  capabilityGraphLoaded,
  capabilityGraphLoaded.exports,
  (request) => {
    if (request === './projectDocumentArchitecture') return architectureLoaded.exports
    if (request === './projectDocumentSections') return loaded.exports
    return require(request)
  },
)

const capabilityManifest = parseSectionManifest(JSON.stringify({
  version: 1,
  profile: 'software-platform',
  home: { title: '测试知识库', summary: '测试', entrypoint: 'README.md', start_here: [] },
  sections: [
    { id: 'product-capabilities', label: '产品能力', detail: '用户可见功能', color: '#4477aa', order: 10, entrypoint: 'README.md' },
    { id: 'knowledge', label: '知识管理', detail: '文档整理能力', color: '#55aa88', order: 10, parent_id: 'product-capabilities' },
  ],
  assignments: {
    'README.md': 'custom:product-capabilities',
    'docs/research.md': 'custom:knowledge',
  },
}))
const capabilityGraph = capabilityGraphLoaded.exports.buildCapabilityGraph('测试项目', catalog, capabilityManifest)
const capabilityRoot = capabilityGraph.nodes.find((node) => node.isRoot)
const capabilityParent = capabilityGraph.nodes.find((node) => node.id === 'custom:product-capabilities')
const capabilityChild = capabilityGraph.nodes.find((node) => node.id === 'custom:knowledge')
assert.equal(capabilityRoot.documentCount, documents.length)
assert.equal(capabilityParent.entrypoint, 'README.md')
assert.equal(capabilityParent.entrypointSource, 'configured')
assert.equal(capabilityParent.childCount, 1)
assert.equal(capabilityChild.parentId, capabilityParent.id)
assert(capabilityChild.depth > capabilityParent.depth)
assert(capabilityGraph.nodes.some((node) => node.status === 'gap'), '无对应文档的能力应显示文档空白')
const collapsedGraph = capabilityGraphLoaded.exports.selectCapabilityGraph(
  capabilityGraph,
  new Set([capabilityParent.id]),
)
assert(!collapsedGraph.nodes.some((node) => node.id === capabilityChild.id), '收起能力后应隐藏后代')
const searchedGraph = capabilityGraphLoaded.exports.selectCapabilityGraph(capabilityGraph, new Set(), 'research')
assert(searchedGraph.nodes.some((node) => node.id === capabilityChild.id))
assert(searchedGraph.nodes.some((node) => node.id === capabilityParent.id), '搜索结果应保留祖先链')
const capabilityPositions = capabilityGraphLoaded.exports.layoutCapabilityGraph(searchedGraph.nodes)
assert(capabilityPositions.get(capabilityChild.id).x > capabilityPositions.get(capabilityParent.id).x)

const workspaceSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentsWorkspace.tsx',
), 'utf8')
assert(!workspaceSource.includes('markSuggestionsRequested'), '启动 AI 前不得在主工作区预写建议占位文件')
assert(workspaceSource.includes('const basePrompt = buildOrganizationPrompt'))
assert(workspaceSource.includes('await onStartAiOrganize(scopeInstruction'))
const editorPaneSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentEditorPane.tsx',
), 'utf8')
assert(editorPaneSource.includes('ProjectDocumentAccessNotice'))
assert(workspaceSource.includes('applyFileOperations'))
assert(workspaceSource.includes('organization.trace?.catalog_revision'))
assert(workspaceSource.includes('ProjectDocumentCapabilityMap'))
assert(workspaceSource.includes('只评估功能节点'))

const capabilityMapSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentCapabilityMap.tsx',
), 'utf8')
assert(capabilityMapSource.includes('ReactFlow'))
assert(capabilityMapSource.includes('MiniMap'))
assert(capabilityMapSource.includes('Controls'))
assert(capabilityMapSource.includes('搜索功能或文档'))
assert(!capabilityMapSource.includes('/docs/file'), '功能图只能消费目录元数据，不应自行读取 Markdown 正文')

const capabilityInspectorSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentCapabilityInspector.tsx',
), 'utf8')
assert(capabilityInspectorSource.includes('对应 Markdown'))
assert(capabilityInspectorSource.includes('让 AI 补齐此功能'))
assert(workspaceSource.includes('<ProjectDocumentHealthCenter'))
const healthCenterSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentHealthCenter.tsx',
), 'utf8')
assert(healthCenterSource.includes('服务端统一真源'))
assert(healthCenterSource.includes('联邦知识架构'))
assert(healthCenterSource.includes('持续维护'))

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

const commandSourcePath = path.join(__dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentCommands.ts')
const commandOutput = ts.transpileModule(fs.readFileSync(commandSourcePath, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
}).outputText
const commandLoaded = { exports: {} }
new Function('module', 'exports', 'require', commandOutput)(
  commandLoaded,
  commandLoaded.exports,
  (request) => request === './projectDocumentSections' ? loaded.exports : require(request),
)
const commands = commandLoaded.exports

let commandManifest = parseSectionManifest(JSON.stringify({
  version: 1,
  sections: [
    createCustomSection('产品', [], ''),
    createCustomSection('接口', [{ ...createCustomSection('产品', [], ''), id: '产品' }], ''),
  ],
  assignments: { 'docs/research.md': 'custom:产品' },
}))
const productKey = customSectionKey(commandManifest.sections[0].id)
const apiKey = customSectionKey(commandManifest.sections[1].id)
commandManifest = commands.updateSectionDefinition(commandManifest, apiKey, { parent_id: commandManifest.sections[0].id })
assert.equal(commandManifest.sections[1].parent_id, commandManifest.sections[0].id)
assert.throws(() => commands.updateSectionDefinition(commandManifest, productKey, { parent_id: commandManifest.sections[1].id }), /子分区/)

const levelOne = createCustomSection('一级', [], '')
const levelTwo = createCustomSection('二级', [levelOne], levelOne.id)
const levelThree = createCustomSection('三级', [levelOne, levelTwo], levelTwo.id)
const levelFour = createCustomSection('四级', [levelOne, levelTwo, levelThree], levelThree.id)
const detached = createCustomSection('待移动', [levelOne, levelTwo, levelThree, levelFour], '')
const depthManifest = parseSectionManifest(JSON.stringify({
  version: 1,
  sections: [levelOne, levelTwo, levelThree, levelFour, detached],
}))
assert.doesNotThrow(() => commands.updateSectionDefinition(
  depthManifest,
  customSectionKey(levelFour.id),
  { parent_id: levelThree.id },
))
assert.throws(() => commands.updateSectionDefinition(
  depthManifest,
  customSectionKey(detached.id),
  { parent_id: levelFour.id },
), /最多支持四层/)
assert.throws(() => commands.createSectionInManifest(depthManifest, '第五级', levelFour.id), /最多支持四层/)

commandManifest = commands.assignDocuments(commandManifest, ['docs/unknown.md'], apiKey, 'knowledge')
commandManifest = commands.assignDocuments(commandManifest, ['docs/unknown.md'], 'current', 'governance')
assert.equal(commandManifest.assignments['docs/unknown.md'], apiKey)
assert.equal(commandManifest.governance_overrides['docs/unknown.md'], 'current')
commandManifest = commands.setRecommendedDocuments(commandManifest, ['docs/unknown.md'], true)
commandManifest = commands.pinDocuments(commandManifest, ['docs/unknown.md'], true)
assert(commandManifest.home.start_here.includes('docs/unknown.md'))
assert.equal(commandManifest.document_metadata['docs/unknown.md'].pinned, true)
assert(commandManifest.audit_log.length >= 5)

const ordered = commands.sortDocuments(documents, commandManifest, 'manual')
assert.equal(ordered[0].path, 'docs/unknown.md', '固定文档必须显示在顶部')
let manualDocumentManifest = parseSectionManifest(JSON.stringify({ version: 1 }))
manualDocumentManifest = commands.reorderDocument(
  manualDocumentManifest,
  ['AGENTS.md', 'README.md'],
  'README.md',
  'top',
)
assert.deepEqual(
  commands.sortDocuments(documents.filter((entry) => ['AGENTS.md', 'README.md'].includes(entry.path)), manualDocumentManifest, 'manual')
    .map((entry) => entry.path),
  ['README.md', 'AGENTS.md'],
)
const authorityDocuments = [
  { ...documents[4], metadata: { ...documents[4].metadata, authority: 'historical' } },
  { ...documents[2], metadata: { ...documents[2].metadata, authority: 'informative' } },
  { ...documents[0], metadata: { ...documents[0].metadata, authority: 'repository_routing' } },
]
assert.deepEqual(commands.sortDocuments(authorityDocuments, parseSectionManifest('{}'), 'authority')
  .map((entry) => entry.metadata.authority), ['repository_routing', 'informative', 'historical'])
const hierarchy = commands.sortHierarchicalSections(buildDocumentSections(commandManifest).filter((section) => section.custom), 'manual', {})
assert.equal(hierarchy.find((section) => section.key === apiKey).depth, 1)
commandManifest = commands.mergeSections(commandManifest, apiKey, productKey)
assert.equal(commandManifest.assignments['docs/unknown.md'], productKey)
assert(!commandManifest.sections.some((section) => customSectionKey(section.id) === apiKey))

const notebookSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentNotebookRail.tsx',
), 'utf8')
assert(notebookSource.includes('onContextMenu'))
assert(notebookSource.includes('menuPointForButton'))
assert(notebookSource.includes('draggable='))
const pageListSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentPageList.tsx',
), 'utf8')
assert(pageListSource.includes('onMoveBefore'))
assert(pageListSource.includes('data-drop-target'))
const commandMenuSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentCommandMenu.tsx',
), 'utf8')
assert(commandMenuSource.includes('Shift+F10'))
assert(commandMenuSource.includes('让 AI 评估提权'))
assert(commandMenuSource.includes('不突破真实路径的权威上限'))
assert(commandMenuSource.includes("'ArrowDown', 'ArrowUp', 'Home', 'End'"))

console.log('project document section model tests passed')
