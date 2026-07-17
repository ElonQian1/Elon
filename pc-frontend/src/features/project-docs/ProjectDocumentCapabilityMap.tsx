import { useCallback, useEffect, useMemo, useState } from 'react'
import { Expand, Focus, Network, Search, Shrink, Sparkles } from 'lucide-react'
import {
  Background,
  BackgroundVariant,
  Controls,
  MarkerType,
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type Edge,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'

import ProjectDocumentCapabilityInspector from './ProjectDocumentCapabilityInspector'
import ProjectDocumentCapabilityNode, { type CapabilityFlowNode } from './ProjectDocumentCapabilityNode'
import ProjectDocumentKnowledgeMapTabs from './ProjectDocumentKnowledgeMapTabs'
import {
  buildCapabilityGraph,
  capabilityStatusLabel,
  layoutCapabilityGraph,
  selectCapabilityGraph,
  type CapabilityStatus,
  type ProjectCapabilityNode,
} from './projectDocumentCapabilityGraph'
import type { DocumentCatalog } from './projectDocumentModel'
import type { ProjectKnowledgeMapView } from './projectDocumentKnowledgeGraphModel'
import styles from './ProjectDocumentCapabilityMap.module.css'

interface Props {
  projectName: string
  catalog: DocumentCatalog | null
  canStartAi: boolean
  organizing: boolean
  onOpenDocument: (path: string) => void
  onOpenSection: (sectionId: string) => void
  onAiOrganize: (node: ProjectCapabilityNode) => void
  onAiReview: (view: ProjectKnowledgeMapView) => void
}

const nodeTypes = { capability: ProjectDocumentCapabilityNode }

export default function ProjectDocumentCapabilityMap(props: Props) {
  return (
    <ReactFlowProvider>
      <ProjectDocumentCapabilityMapSurface {...props} />
    </ReactFlowProvider>
  )
}

function ProjectDocumentCapabilityMapSurface({
  projectName,
  catalog,
  canStartAi,
  organizing,
  onOpenDocument,
  onOpenSection,
  onAiOrganize,
  onAiReview,
}: Props) {
  const [view, setView] = useState<ProjectKnowledgeMapView>('capabilities')
  const graph = useMemo(() => buildCapabilityGraph(projectName, catalog, view), [catalog, projectName, view])
  const [query, setQuery] = useState('')
  const [status, setStatus] = useState<CapabilityStatus | 'all'>('all')
  const [collapsedIds, setCollapsedIds] = useState<Set<string>>(() => defaultCollapsed(graph))
  const [selectedId, setSelectedId] = useState(graph.rootId)
  const { fitView } = useReactFlow()

  useEffect(() => {
    setCollapsedIds(defaultCollapsed(graph))
    setSelectedId(graph.rootId)
    setQuery('')
    setStatus('all')
  }, [graph])

  useEffect(() => {
    if (!graph.nodes.some((node) => node.id === selectedId)) setSelectedId(graph.rootId)
  }, [graph, selectedId])

  const toggleNode = useCallback((id: string) => {
    setCollapsedIds((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }, [])
  const selection = useMemo(
    () => selectCapabilityGraph(graph, collapsedIds, query, status),
    [collapsedIds, graph, query, status],
  )
  const positions = useMemo(() => layoutCapabilityGraph(selection.nodes), [selection.nodes])
  const flowNodes = useMemo<CapabilityFlowNode[]>(() => selection.nodes.map((node) => ({
    id: node.id,
    type: 'capability',
    position: positions.get(node.id) ?? { x: 0, y: 0 },
    data: { capability: node, collapsed: collapsedIds.has(node.id), onToggle: toggleNode },
    selected: node.id === selectedId,
    draggable: false,
  })), [collapsedIds, positions, selectedId, selection.nodes, toggleNode])
  const flowEdges = useMemo<Edge[]>(() => selection.edges.map((edge) => ({
    ...edge,
    type: 'smoothstep',
    label: edge.label || undefined,
    animated: edge.relation !== 'contains',
    markerEnd: { type: MarkerType.ArrowClosed, width: 12, height: 12, color: edge.relation === 'contains' ? '#6f7787' : '#8b79b8' },
    style: { stroke: edge.relation === 'contains' ? '#596171' : '#75639a', strokeWidth: edge.relation === 'contains' ? 1.3 : 1.7 },
  })), [selection.edges])
  const selectedNode = graph.nodes.find((node) => node.id === selectedId) ?? graph.nodes[0]
  const visibleKey = selection.nodes.map((node) => node.id).join('|')

  useEffect(() => {
    const timer = window.setTimeout(() => { void fitView({ padding: 0.2, duration: 280, maxZoom: 1.2 }) }, 40)
    return () => window.clearTimeout(timer)
  }, [fitView, visibleKey])

  return (
    <main className={styles.mapShell}>
      <header className={styles.mapToolbar}>
        <div className={styles.mapTitle}>
          <span><Network size={18} /></span>
          <div><strong>项目知识图谱</strong><small>Rust 后端统一生成 · 0 次正文读取</small></div>
        </div>
        <ProjectDocumentKnowledgeMapTabs value={view} onChange={setView} />
        <div className={styles.statusSummary}>
          {(['healthy', 'partial', 'gap'] as CapabilityStatus[]).map((item) => (
            <button key={item} type="button" data-status={item} data-active={status === item || undefined} onClick={() => setStatus(status === item ? 'all' : item)}>
              <i />{capabilityStatusLabel(item)} <b>{graph.stats[item]}</b>
            </button>
          ))}
        </div>
        <label className={styles.mapSearch}>
          <Search size={14} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索节点、文档或实现证据" />
        </label>
        <div className={styles.mapActions}>
          <button type="button" title="展开全部" onClick={() => setCollapsedIds(new Set())}><Expand size={14} /></button>
          <button type="button" title="收起到一级" onClick={() => setCollapsedIds(defaultCollapsed(graph))}><Shrink size={14} /></button>
          <button type="button" title="适应画布" onClick={() => { void fitView({ padding: .2, duration: 280 }) }}><Focus size={14} /></button>
        </div>
        <button className={styles.aiReviewButton} type="button" disabled={!canStartAi || organizing} onClick={() => onAiReview(view)}>
          <Sparkles size={14} />{organizing ? '正在创建任务…' : '与 AI 评审此图'}
        </button>
      </header>

      <section className={styles.mapDiagnostic} data-status={graph.diagnosticStatus}>
        <strong>结构分 {graph.structuralScore}</strong>
        <span>{graph.source === 'manifest' ? '项目已固化' : graph.source === 'profile_template' ? '当前为模板推导，建议与 AI 核对后固化' : '节点尚未返回统一图谱，请刷新或升级 Windows 节点'}</span>
        <em>{graph.findings.length} 条确定性发现</em>
      </section>

      <div className={styles.mapContent}>
        <div className={styles.canvas} aria-label="项目知识图谱">
          <ReactFlow
            nodes={flowNodes}
            edges={flowEdges}
            nodeTypes={nodeTypes}
            minZoom={0.3}
            maxZoom={1.8}
            fitView
            colorMode="dark"
            nodesConnectable={false}
            nodesDraggable={false}
            onNodeClick={(_, node) => setSelectedId(node.id)}
            proOptions={{ hideAttribution: true }}
          >
            <Background variant={BackgroundVariant.Dots} gap={22} size={1} color="#343943" />
            <MiniMap className={styles.miniMap} pannable zoomable
              nodeColor={(node) => (node.data.capability as ProjectCapabilityNode).color}
              maskColor="rgba(17, 18, 22, .75)" />
            <Controls className={styles.flowControls} showInteractive={false} />
          </ReactFlow>
          {selection.nodes.length === 1 && (query || status !== 'all') && (
            <div className={styles.noResult}>没有匹配节点，请调整搜索或文档状态筛选。</div>
          )}
        </div>
        {selectedNode && (
          <ProjectDocumentCapabilityInspector node={selectedNode} canStartAi={canStartAi} organizing={organizing}
            onOpenDocument={onOpenDocument} onOpenSection={onOpenSection} onAiOrganize={onAiOrganize} />
        )}
      </div>
    </main>
  )
}

function defaultCollapsed(graph: ReturnType<typeof buildCapabilityGraph>) {
  return new Set(graph.nodes.filter((node) => node.depth === 1 && node.childCount > 0).map((node) => node.id))
}
