import type { ProjectCapabilityNode } from './projectDocumentCapabilityGraph'
import type { DiscussionNode } from './projectDocumentDiscussionModel'
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

export function discussionSourceInstruction(path: string) {
  return `把已保存的原始聊天“${path}”编译为可继续工作的讨论知识图。` +
    '必须先调用 project_discussions_get_graph，再用 project_docs_plan_context 规划并只读取这一个来源文档；不要全文扫描项目文档。' +
    '把讨论拆成稳定主题、问题、主张、假设、方案、反对意见、证据、风险、决策、需求、功能、任务和结果节点，保留父子推导与 supports、opposes、alternative_to、depends_on、leads_to、spawns 关系。' +
    '每个结论必须保留 source_refs；原始聊天只作为 source_material，不得晋升为项目事实。' +
    '调用 project_discussions_save_proposal 保存增量图，change_kind=import，并填写简短 actor；只对已确认且值得长期维护的结论提出 promotions，避免一节点一文档和重复权威文档。' +
    '按当前权限允许时调用 project_discussions_apply；应用后用 project_discussions_get_graph、get_history 和 review_graph 检查新版本。'
}

export function discussionNodeInstruction(
  node: DiscussionNode,
  mode: 'continue' | 'fork' | 'promote',
) {
  const action = mode === 'fork'
    ? '从该节点创建一条新的备选分支，保留原节点，用 spawns 或 alternative_to 表达关系'
    : mode === 'promote'
      ? '评估该节点是否已经足够稳定，可晋升为需求、决策、功能说明或任务文档；不满足证据条件就只给出缺口'
      : '沿该节点继续讨论，补充问题、证据、风险、方案和下一步，不改写已经确认的历史推理'
  return `围绕讨论节点“${node.title}”（node_id=${node.id}）继续工作：${action}。` +
    '必须先调用 project_discussions_get_node 获取祖先、子节点和交叉关系，再按需规划极少量正文读取。' +
    '新增内容必须携带来源或当前任务引用；把事实、假设、意见和决策分开，不得把讨论自动写成权威事实。' +
    '通过 project_discussions_save_proposal 保存增量变更，继续/分叉使用 change_kind=expand，晋升使用 change_kind=decision；' +
    '按当前权限允许时调用 project_discussions_apply，并用 get_node、trace_node 和 review_graph 回读确认。'
}

export function discussionApplyInstruction() {
  return '用户已在讨论推理页面明确批准待处理的讨论图建议。调用 project_discussions_get_graph、project_discussions_get_suggestions 和 project_discussions_review_graph 核对 revision、摘要与已知风险，再调用 project_discussions_apply；review_all 模式传 reviewed=true。应用一定产生可回看的新版本；之后调用 get_history 和 compare_versions，报告新增节点、状态变化、分支、晋升文档与剩余质量问题。'
}
