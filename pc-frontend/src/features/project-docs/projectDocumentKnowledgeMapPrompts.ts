import type { ProjectCapabilityNode } from './projectDocumentCapabilityGraph'
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
