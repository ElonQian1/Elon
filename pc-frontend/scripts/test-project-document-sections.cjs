const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const ts = require('typescript')

const graphModelPath = path.join(__dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentKnowledgeGraphModel.ts')
const graphModelOutput = ts.transpileModule(fs.readFileSync(graphModelPath, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const graphModelLoaded = { exports: {} }
new Function('module', 'exports', 'require', graphModelOutput)(graphModelLoaded, graphModelLoaded.exports, require)

const governancePath = path.join(__dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentGovernance.ts')
const governanceOutput = ts.transpileModule(fs.readFileSync(governancePath, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const governanceLoaded = { exports: {} }
new Function('module', 'exports', 'require', governanceOutput)(governanceLoaded, governanceLoaded.exports, require)

const sourcePath = path.join(__dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentSections.ts')
const source = fs.readFileSync(sourcePath, 'utf8')
const output = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const loaded = { exports: {} }
new Function('module', 'exports', 'require', output)(
  loaded,
  loaded.exports,
  (request) => {
    if (request === './projectDocumentKnowledgeGraphModel') return graphModelLoaded.exports
    if (request === './projectDocumentGovernance') return governanceLoaded.exports
    return require(request)
  },
)

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
  sections: [
    { id: 'research', label: '研究', detail: '研究笔记', color: '#123456' },
    { id: 'operations', label: '运维', detail: '运行手册', color: '#456789' },
  ],
  assignments: { 'docs/research.md': 'custom:research', 'docs/discussion.md': 'drafts' },
  secondary_assignments: { 'docs/research.md': ['custom:operations', 'custom:research'] },
  governance_facets: { 'docs/research.md': { retrieval: 'excluded', lifecycle: 'draft', authority: 'proposal', document_type: 'discussion' } },
  governance_overrides: { 'docs/unknown.md': 'on-demand' },
  document_metadata: {
    'docs/research.md': { order: 7, pinned: true },
    'docs/unknown.md': { version: '0.9', version_status: 'archived' },
  },
  knowledge_graph: { nodes: [{
    id: 'cap-research', view: 'capabilities', label: '研究能力', document_paths: ['docs/research.md'],
  }], edges: [] },
  audit_log: [{ id: 'one', action: 'test', target: 'docs/research.md', summary: '测试审计', at: '2026-07-17T00:00:00Z' }],
}))
assert.equal(manifest.assignments['docs/research.md'], 'custom:research')
assert.equal(manifest.assignments['docs/discussion.md'], undefined)
assert.equal(manifest.governance_overrides['docs/discussion.md'], 'drafts', '旧清单的治理归类应自动迁移')
assert.equal(manifest.governance_overrides['docs/unknown.md'], 'on-demand')
assert.deepEqual(manifest.secondary_assignments['docs/research.md'], ['custom:operations'])
assert.equal(manifest.governance_facets['docs/research.md'].authority, 'proposal')
assert.equal(manifest.document_metadata['docs/research.md'].order, 7)
assert.equal(manifest.document_metadata['docs/research.md'].pinned, true)
assert.equal(manifest.document_metadata['docs/unknown.md'].version_status, 'archived')
assert.equal(manifest.audit_log[0].action, 'test')
assert.equal(manifest.knowledge_graph.nodes[0].id, 'cap-research')
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
  'archive',
  'custom:research',
])
assert.equal(sections.length, documents.length, '每份文档必须只属于一个分区')
assert(buildDocumentSections(manifest).some((section) => section.key === 'custom:research'))
assert(buildDocumentSections(manifest).some((section) => section.key === 'suggestions'))

const importedConversation = document(
  'docs/inbox/conversations/provider-chat.md',
  'discussion',
  'source_material',
)
importedConversation.metadata.authority = 'none'
importedConversation.metadata.default_retrieval = false
const importedFacets = governanceLoaded.exports.effectiveGovernanceFacets(importedConversation)
assert.equal(importedFacets.retrieval, 'excluded')
assert.equal(importedFacets.lifecycle, 'source_material')
assert.equal(importedFacets.authority, 'none')
assert.equal(governanceLoaded.exports.governanceQuickView(importedFacets), 'drafts')

const suggestions = parseOrganizationSuggestions(JSON.stringify({
  version: 1,
  status: 'ready',
  summary: 'ok',
  proposed_sections: [{ id: 'api', label: 'API', detail: '接口', color: '#abcdef' }],
  assignments: [{ path: 'docs/unknown.md', section_id: 'custom:api', reason: '归类' }],
  section_operations: [{ id: 'merge-api', kind: 'merge', section_id: 'api', target_section_id: 'research', reason: '主题重叠', impact: 'API 文档归入研究主题' }],
  governance_facets: { 'docs/unknown.md': { retrieval: 'excluded', lifecycle: 'draft', authority: 'proposal', document_type: 'discussion' } },
  conflicts: [],
  move_suggestions: [],
  file_operations: [{
    id: 'rename-unknown', kind: 'rename', source_path: 'docs/unknown.md',
    target_path: 'docs/unknown-topic.md', source_revision: 'abc', reason: '名称更可检索',
  }],
  proposed_knowledge_graph: {
    nodes: [{ id: 'cap-api', view: 'capabilities', label: 'API 能力', document_paths: ['docs/unknown.md'] }],
    edges: [],
  },
  documents_read: 1,
  estimated_tokens_used: 20,
}))
assert.equal(suggestions.status, 'ready')
assert.equal(suggestions.assignments.length, 1)
assert.equal(suggestions.section_operations[0].impact, 'API 文档归入研究主题')
assert.equal(suggestions.assignments[0].secondary, false)
assert.equal(suggestions.governance_facets['docs/unknown.md'].lifecycle, 'draft')
assert.equal(suggestions.file_operations.length, 1)
assert.equal(suggestions.file_operations[0].status, 'proposed')
assert.equal(suggestions.proposed_knowledge_graph.nodes[0].id, 'cap-api')

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
assert.equal(boundedSuggestions.assignments.length, 510)

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
assert(prompt.includes('project_docs_update_issue'))
assert(prompt.includes('project_docs_get_health_history'))
assert(prompt.includes('project_docs_get_map'))
assert(prompt.includes('project_docs_review_map'))
assert(prompt.includes('project_docs_plan_context'))
assert(prompt.includes('project_docs_get_federation'))
assert(prompt.includes('project_discussions_get_graph'))
assert(prompt.includes('section_operations'))
assert(prompt.includes('16 分区/500 文档'))
assert(prompt.includes('proposed_knowledge_graph'))
assert(prompt.includes('权限模式：git_backed_full'))
assert(prompt.includes('authorization_mode=git_backed_full'))
assert(prompt.includes('git_baseline_commit'))
assert(prompt.includes('git_result_commit'))
assert(prompt.includes('project_docs_apply_file_operations'))
assert(prompt.includes('source_revision'))
assert(prompt.includes('document_health'))
assert(prompt.includes('主题知识树'))
assert(prompt.includes('governance_facets'))
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
assert(architectureSource.includes('topicSectionsForDocument'))
assert(architectureSource.includes("id: 'unassigned-topic'"))
assert(!architectureSource.includes('sharedWorkspace'))
assert(architectureSource.includes('const topics = [...templateSections, ...customSections]'))
assert(architectureSource.includes("CAPABILITY_MAP_SECTION = 'capability-map'"))
assert(architectureSource.includes("DISCUSSION_MAP_SECTION = 'discussion-map'"))

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

const mapNode = (id, label, parentId, depth, documentPaths, entrypoint = '') => ({
  id, view: 'capabilities', kind: 'capability', label, detail: label, color: '#4477aa',
  parent_id: parentId, section_id: '', depth, child_count: id === 'cap-parent' ? 1 : 0,
  order: 10, document_count: documentPaths.length, document_paths: documentPaths,
  entrypoint, entrypoint_source: entrypoint ? 'configured' : 'missing',
  coverage: [{ key: 'overview', label: '入口', covered: !!entrypoint, count: entrypoint ? 1 : 0 }],
  missing_coverage: entrypoint ? [] : ['入口'], documentation_status: documentPaths.length ? 'documented' : 'undocumented',
  implementation_refs: [{ reference: 'file:src/main.rs', verification: 'exists' }],
  implementation_status: 'verified', source: 'manifest', tags: [],
})
const capabilityMap = {
  version: 1, view: 'capabilities', title: '产品功能图', source: 'manifest', root_id: 'map-capabilities-root',
  nodes: [
    { ...mapNode('map-capabilities-root', '测试知识库', '', 0, documents.map((item) => item.path), 'README.md'), kind: 'project', child_count: 1 },
    mapNode('cap-parent', '产品能力', 'map-capabilities-root', 1, ['README.md'], 'README.md'),
    mapNode('cap-child', '知识管理', 'cap-parent', 2, ['docs/research.md']),
    mapNode('cap-gap', '空白能力', 'map-capabilities-root', 1, []),
  ],
  edges: [
    { id: 'root-parent', source: 'map-capabilities-root', target: 'cap-parent', relation: 'contains', label: '', configured: true },
    { id: 'parent-child', source: 'cap-parent', target: 'cap-child', relation: 'contains', label: '', configured: true },
    { id: 'root-gap', source: 'map-capabilities-root', target: 'cap-gap', relation: 'contains', label: '', configured: true },
  ],
  stats: { nodes: 3, configured_nodes: 3, documented: 2, partial: 0, undocumented: 1, implementation_verified: 3, implementation_declared: 0, implementation_missing: 0 },
  diagnostics: { structural_score: 92, status: 'healthy', findings: [] },
  budget: { classification_model_tokens: 0, markdown_bodies_read: 0, metadata_only: true },
}
const capabilityCatalog = { ...catalog, analysis: { knowledge_maps: { capabilities: capabilityMap } } }
const capabilityGraph = capabilityGraphLoaded.exports.buildCapabilityGraph('测试项目', capabilityCatalog, 'capabilities')
const capabilityRoot = capabilityGraph.nodes.find((node) => node.isRoot)
const capabilityParent = capabilityGraph.nodes.find((node) => node.id === 'cap-parent')
const capabilityChild = capabilityGraph.nodes.find((node) => node.id === 'cap-child')
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

const consistencySourcePath = path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentGraphConsistency.ts',
)
const consistencyOutput = ts.transpileModule(fs.readFileSync(consistencySourcePath, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const consistencyLoaded = { exports: {} }
new Function('module', 'exports', 'require', consistencyOutput)(consistencyLoaded, consistencyLoaded.exports, require)
const diagnoseConsistency = consistencyLoaded.exports.diagnoseProjectDocumentGraphConsistency
const identifiedCatalog = {
  ...capabilityCatalog,
  workspace: 'D:\\repo',
  analysis: {
    ...capabilityCatalog.analysis,
    identity: {
      workspace: 'D:\\repo', canonical_workspace: 'D:\\repo',
      manifest_revision: 'manifest-new', knowledge_map_revision: 'graph-new',
    },
  },
}
assert.equal(diagnoseConsistency({
  catalog: identifiedCatalog, graph: capabilityGraph, expectedWorkspace: '\\\\?\\D:\\repo',
  expectedManifestRevision: 'manifest-new', configuredNodes: 3,
}).status, 'current')
assert.equal(diagnoseConsistency({
  catalog: identifiedCatalog, graph: capabilityGraph, expectedWorkspace: 'D:\\repo',
  expectedManifestRevision: 'manifest-next', configuredNodes: 3,
}).status, 'stale')
assert.equal(diagnoseConsistency({
  catalog: identifiedCatalog, graph: { ...capabilityGraph, source: 'profile_template' },
  expectedWorkspace: 'D:\\repo', expectedManifestRevision: 'manifest-new', configuredNodes: 3,
}).status, 'unexpected_template')
assert.equal(diagnoseConsistency({
  catalog: identifiedCatalog, graph: capabilityGraph, expectedWorkspace: 'D:\\other',
  expectedManifestRevision: 'manifest-new', configuredNodes: 3,
}).status, 'workspace_mismatch')

const workspaceSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentsWorkspace.tsx',
), 'utf8')
assert(!workspaceSource.includes('markSuggestionsRequested'), '启动 AI 前不得在主工作区预写建议占位文件')
assert(workspaceSource.includes('const basePrompt = buildOrganizationPrompt'))
assert(workspaceSource.includes('await onStartAiOrganize(scopeInstruction'))
assert(workspaceSource.includes('useProjectDocumentGraphFreshness'))
assert(workspaceSource.includes('await organization.applySuggestions'))
assert(workspaceSource.includes('expectedWorkspace={organizationTracking.projectRoot}'))
const editorPaneSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentEditorPane.tsx',
), 'utf8')
assert(editorPaneSource.includes('ProjectDocumentAccessNotice'))
assert(workspaceSource.includes('applyFileOperations'))
assert(workspaceSource.includes('organization.trace?.catalog_revision'))
assert(workspaceSource.includes('ProjectDocumentCapabilityMap'))
assert(workspaceSource.includes('knowledgeNodeReviewInstruction'))

const capabilityMapSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentCapabilityMap.tsx',
), 'utf8')
assert(capabilityMapSource.includes('ReactFlow'))
assert(capabilityMapSource.includes('MiniMap'))
assert(capabilityMapSource.includes('Controls'))
assert(capabilityMapSource.includes('搜索节点、文档或实现证据'))
assert(capabilityMapSource.includes('ProjectDocumentKnowledgeMapTabs'))
assert(capabilityMapSource.includes('与 AI 评审此图'))
assert(capabilityMapSource.includes('data-consistency={consistency.status}'))
assert(capabilityMapSource.includes('knowledge_map_revision'))
assert(!capabilityMapSource.includes('/docs/file'), '功能图只能消费目录元数据，不应自行读取 Markdown 正文')

const capabilityInspectorSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentCapabilityInspector.tsx',
), 'utf8')
assert(capabilityInspectorSource.includes('对应 Markdown'))
assert(capabilityInspectorSource.includes('实现证据'))
assert(capabilityInspectorSource.includes('与 AI 讨论此节点'))
assert(workspaceSource.includes('<ProjectDocumentHealthCenter'))
assert(workspaceSource.includes('<ProjectDocumentGovernanceOverview'))
const healthCenterSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentHealthCenter.tsx',
), 'utf8')
assert(healthCenterSource.includes('服务端统一真源'))
assert(healthCenterSource.includes('联邦知识架构'))
assert(healthCenterSource.includes('持续维护'))
assert(healthCenterSource.includes('让 AI 处理选中的'))
assert(healthCenterSource.includes('ProjectDocumentVersionHistory'))

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
commandManifest = commands.setSecondaryTopics(commandManifest, 'docs/unknown.md', [productKey])
commandManifest = commands.setGovernanceFacets(commandManifest, 'docs/unknown.md', {
  retrieval: 'on_demand', lifecycle: 'active', authority: 'guidance', document_type: 'guide',
})
assert.deepEqual(commandManifest.secondary_assignments['docs/unknown.md'], [productKey])
assert.equal(commandManifest.governance_facets['docs/unknown.md'].document_type, 'guide')
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
const federationSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentFederationIndex.tsx',
), 'utf8')
assert(federationSource.includes('/docs/federation?'))
assert(federationSource.includes('data-pagination="server"'))
assert(federationSource.includes('加载下一页'))
assert(federationSource.includes('direct_children'))
assert(!federationSource.includes('.slice('), '联邦节点不得在 catalog 全量数组上客户端切片')

const pagingPath = path.join(__dirname, '..', 'src', 'features', 'project-docs', 'projectDocumentFederationPaging.ts')
const pagingOutput = ts.transpileModule(fs.readFileSync(pagingPath, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const pagingLoaded = { exports: {} }
new Function('module', 'exports', 'require', pagingOutput)(pagingLoaded, pagingLoaded.exports, require)
const paging = pagingLoaded.exports
const node = (id) => ({ id, label: id, parent_id: '', scope_path: '', profile: '', owner: '', document_count: 1, direct_children: 0, score: 100, status: 'healthy', home_configured: true })
let pages = paging.beginFederationPage({}, '', 1, false)
pages = paging.acceptFederationPage(pages, '', 1, { nodes: [node('root')], pagination: { returned: 1, total_matching: 2, has_more: true, next_cursor: 'offset:1' } }, false)
pages = paging.beginFederationPage(pages, '', 2, true)
const stale = paging.acceptFederationPage(pages, '', 1, { nodes: [node('stale')], pagination: { returned: 1, total_matching: 2, has_more: false } }, true)
assert.deepEqual(stale[''].nodes.map((entry) => entry.id), ['root'], '旧请求不得覆盖同一分支的新请求')
pages = paging.acceptFederationPage(stale, '', 2, { nodes: [node('child')], pagination: { returned: 1, total_matching: 2, has_more: false } }, true)
assert.deepEqual(pages[''].nodes.map((entry) => entry.id), ['root', 'child'])
pages = paging.beginFederationPage(pages, 'root', 3, false)
pages = paging.rejectFederationPage(pages, 'root', 3, 'network failed')
assert.equal(pages.root.error, 'network failed')
assert.equal(pages[''].error, '', '子分支错误不得污染根分页')
const editorSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentEditorPane.tsx',
), 'utf8')
assert(editorSource.includes('Markdown 编辑器'))
assert(editorSource.includes('<MarkdownContent'))
const suggestionsSource = fs.readFileSync(path.join(
  __dirname, '..', 'src', 'features', 'project-docs', 'ProjectDocumentSuggestions.tsx',
), 'utf8')
assert(suggestionsSource.includes('AI 分区治理建议'))
assert(suggestionsSource.includes('理由：'))
assert(suggestionsSource.includes('影响：'))

console.log('project document section model tests passed')
