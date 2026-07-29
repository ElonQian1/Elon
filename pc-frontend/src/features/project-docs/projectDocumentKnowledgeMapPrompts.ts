import type { ProjectCapabilityNode } from './projectDocumentCapabilityGraph'
import type {
  DiscussionActionRequest,
  DiscussionNode,
  ImportedDiscussionSource,
} from './projectDocumentDiscussionModel'
import type { ProjectKnowledgeMapView } from './projectDocumentKnowledgeGraphModel'

export function knowledgeNodeReviewInstruction(node: ProjectCapabilityNode) {
  const paths = node.documentPaths.slice(0, 24).join(', ')
  const viewLabel = node.view === 'architecture' ? '技术架构' : node.view === 'topics' ? '文档主题' : '产品功能'
  return `只评估${viewLabel}节点“${node.label}”（node_id=${node.id}）。必须直接调用 project_docs_get_node，` +
    `再调用 project_docs_plan_context 规划少量必要阅读；当前关联 ${node.documentCount} 份文档${paths ? `：${paths}` : '，尚无文档'}。` +
    '分别判断文档覆盖和实现证据，不得把“有文档”表述为“功能已实现”；复用现有权威文档，不为填满指标创建重复文档。' +
    '如需调整节点、父子关系、文档映射或实现引用，在 proposed_knowledge_graph 中给出可审核变更。'
}

export function knowledgeMapReviewInstruction(view: ProjectKnowledgeMapView) {
  const label = view === 'architecture' ? '技术架构图' : view === 'topics' ? '文档主题图' : '产品功能图'
  return `请与用户评审当前${label}是否真实、清晰且适合 AI 快速理解项目。必须调用 project_docs_get_map(view=${view}) 和 ` +
    `project_docs_review_map(view=${view})；只在诊断命中具体节点时再调用 project_docs_get_node。` +
    '必须区分产品功能、技术组件、文档主题和治理状态，核对父子关系、交叉依赖、入口文档和实现证据。' +
    '先给出“合理之处 / 不足 / 建议变更 / 证据”结论，再把确认有价值的结构变更写入 proposed_knowledge_graph。'
}

export function discussionSourceInstruction(source: ImportedDiscussionSource) {
  return `把已保存的原始聊天“${source.path}”编译为可继续工作的讨论知识图。来源 ID=${source.source_id}，revision=${source.source_revision}。` +
    '必须先调用 project_discussions_get_graph，再调用 project_discussions_get_source_manifest；不得用 project_docs_read 全文读取聊天，也不要扫描其他项目文档。' +
    '核对 manifest 的 source_id、source_revision 与图中同一来源的 processed_chunk_ids，只按顺序调用 project_discussions_read_source_chunk 读取尚未处理的 chunk，并传 expected_source_revision。' +
    '把讨论拆成稳定主题、问题、主张、假设、方案、反对意见、证据、风险、决策、需求、功能、任务和结果节点，保留父子推导与 supports、opposes、alternative_to、depends_on、leads_to、spawns 关系。' +
    '每个结论必须使用返回的 source_id#turn-xxxx 作为 source_refs；原始聊天只作为 source_material，不得晋升为项目事实。保留现有稳定节点 ID，只增量合并，禁止每次重建整张图。' +
    `调用 project_discussions_save_proposal 保存增量图，change_kind=import，并填写简短 actor；source.reference 必须保持精确路径“${source.path}”，并记录 content_revision、source_format、message_count、chunk_count、processed_chunk_ids 和 compilation_status。` +
    '如果本次不能读完，先保存 partial 进度并明确下一个 chunk；全部处理后才能标记 complete。只对已确认且值得长期维护的结论提出 promotions，避免一节点一文档和重复权威文档。' +
    '按当前权限允许时调用 project_discussions_apply；应用后用 project_discussions_get_graph、get_history 和 review_graph 检查新版本。'
}

export function discussionNodeInstruction(
  node: DiscussionNode,
  mode: 'continue' | 'fork' | 'promote',
  request: DiscussionActionRequest,
  source?: ImportedDiscussionSource,
) {
  const action = mode === 'fork'
    ? '从该节点创建一条新的备选分支，保留原节点，用 spawns 或 alternative_to 表达关系'
    : mode === 'promote'
      ? '评估该节点是否已经足够稳定，可晋升为需求、决策、功能说明或任务文档；不满足证据条件就只给出缺口'
      : '沿该节点继续讨论，补充问题、证据、风险、方案和下一步，不改写已经确认的历史推理'
  const userDetails = request.details.slice(0, 20_000)
  const promotion = mode === 'promote'
    ? `用户期望的文档类型=${request.documentType || 'requirement'}；目标路径=${request.targetPath || '由治理规则选择'}。`
    : ''
  const sourceInstruction = source
    ? `本次新增内容已先保存为来源“${source.path}”（source_id=${source.source_id}, revision=${source.source_revision}）。` +
      '调用 project_discussions_get_source_manifest 和 project_discussions_read_source_chunk 读取它，新增节点必须引用返回的 turn 锚点；不要把下面的摘要文字当作唯一来源。'
    : ''
  return `围绕讨论节点“${node.title}”（node_id=${node.id}）继续工作：${action}。` +
    `${promotion}${sourceInstruction}用户本次明确输入如下：\n---\n${userDetails || '没有补充正文，仅请求评估现有节点。'}\n---\n` +
    '必须先调用 project_discussions_get_node 获取祖先、子节点和交叉关系，再按需规划极少量正文读取。' +
    '新增内容必须携带来源或当前任务引用；把事实、假设、意见和决策分开，不得把讨论自动写成权威事实。' +
    '通过 project_discussions_save_proposal 保存增量变更，继续/分叉使用 change_kind=expand，晋升使用 change_kind=decision；' +
    '按当前权限允许时调用 project_discussions_apply，并用 get_node、trace_node 和 review_graph 回读确认。'
}

export function discussionApplyInstruction() {
  return '用户已在讨论推理页面明确批准待处理的讨论图建议。调用 project_discussions_get_graph、project_discussions_get_suggestions 和 project_discussions_review_graph 核对 revision、摘要与已知风险，再调用 project_discussions_apply；review_all 模式传 reviewed=true。应用一定产生可回看的新版本；之后调用 get_history 和 compare_versions，报告新增节点、状态变化、分支、晋升文档与剩余质量问题。'
}
