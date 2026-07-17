import { useCallback, useEffect, useMemo, useState } from 'react'
import { Expand, Focus, Network, Search, Shrink } from 'lucide-react'
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
import {
  buildCapabilityGraph,
  capabilityStatusLabel,
  layoutCapabilityGraph,
  selectCapabilityGraph,
  type CapabilityStatus,
  type ProjectCapabilityNode,
} from './projectDocumentCapabilityGraph'
import type { DocumentCatalog } from './projectDocumentModel'
import type { DocumentSectionManifest } from './projectDocumentSections'
import styles from './ProjectDocumentCapabilityMap.module.css'

interface Props {
  projectName: string
  catalog: DocumentCatalog | null
  manifest: DocumentSectionManifest
  canStartAi: boolean
  organizing: boolean
  onOpenDocument: (path: string) => void
  onOpenSection: (sectionId: string) => void
  onAiOrganize: (node: ProjectCapabilityNode) => void
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
  manifest,
  canStartAi,
  organizing,
  onOpenDocument,
  onOpenSection,
  onAiOrganize,
}: Props) {
  const graph = useMemo(() => buildCapabilityGraph(projectName, catalog, manifest), [catalog, manifest, projectName])
  const [query, setQuery] = useState('')
  const [status, setStatus] = useState<CapabilityStatus | 'all'>('all')
  const [collapsedIds, setCollapsedIds] = useState<Set<string>>(() => new Set())
  const [selectedId, setSelectedId] = useState(graph.rootId)
  const { fitView } = useReactFlow()

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
    markerEnd: { type: MarkerType.ArrowClosed, width: 12, height: 12, color: '#6f7787' },
    style: { stroke: '#596171', strokeWidth: 1.3 },
  })), [selection.edges])
  const selectedNode = graph.nodes.find((node) => node.id === selectedId) ?? graph.nodes[0]
  const visibleKey = selection.nodes.map((node) => node.id).join('|')

  useEffect(() => {
    const timer = window.setTimeout(() => { void fitView({ padding: 0.18, duration: 280, maxZoom: 1.1 }) }, 40)
    return () => window.clearTimeout(timer)
  }, [fitView, visibleKey])

  const collapseBranches = () => setCollapsedIds(new Set(
    graph.nodes.filter((node) => !node.isRoot && node.childCount > 0).map((node) => node.id),
  ))

  return (
    <main className={styles.mapShell}>
      <header className={styles.mapToolbar}>
        <div className={styles.mapTitle}>
          <span><Network size={18} /></span>
          <div><strong>项目功能地图</strong><small>从目录元数据派生，不读取 Markdown 正文</small></div>
        </div>
        <div className={styles.statusSummary}>
          {(['healthy', 'partial', 'gap'] as CapabilityStatus[]).map((item) => (
            <button key={item} type="button" data-status={item} data-active={status === item || undefined} onClick={() => setStatus(status === item ? 'all' : item)}>
              <i />{capabilityStatusLabel(item)} <b>{graph.stats[item]}</b>
            </button>
          ))}
        </div>
        <label className={styles.mapSearch}>
          <Search size={14} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索功能或文档" />
        </label>
        <div className={styles.mapActions}>
          <button type="button" title="展开全部" onClick={() => setCollapsedIds(new Set())}><Expand size={14} /></button>
          <button type="button" title="收起子能力" onClick={collapseBranches}><Shrink size={14} /></button>
          <button type="button" title="适应画布" onClick={() => { void fitView({ padding: .18, duration: 280 }) }}><Focus size={14} /></button>
        </div>
      </header>

      <div className={styles.mapContent}>
        <div className={styles.canvas} aria-label="项目功能脑图">
          <ReactFlow
            nodes={flowNodes}
            edges={flowEdges}
            nodeTypes={nodeTypes}
            minZoom={0.25}
            maxZoom={1.8}
            fitView
            colorMode="dark"
            nodesConnectable={false}
            nodesDraggable={false}
            onNodeClick={(_, node) => setSelectedId(node.id)}
            proOptions={{ hideAttribution: true }}
          >
            <Background variant={BackgroundVariant.Dots} gap={22} size={1} color="#343943" />
            <MiniMap
              className={styles.miniMap}
              pannable
              zoomable
              nodeColor={(node) => (node.data.capability as ProjectCapabilityNode).color}
              maskColor="rgba(17, 18, 22, .75)"
            />
            <Controls className={styles.flowControls} showInteractive={false} />
          </ReactFlow>
          {selection.nodes.length === 1 && (query || status !== 'all') && (
            <div className={styles.noResult}>没有匹配的功能节点，请调整搜索或状态筛选。</div>
          )}
        </div>
        {selectedNode && (
          <ProjectDocumentCapabilityInspector
            node={selectedNode}
            canStartAi={canStartAi}
            organizing={organizing}
            onOpenDocument={onOpenDocument}
            onOpenSection={onOpenSection}
            onAiOrganize={onAiOrganize}
          />
        )}
      </div>
    </main>
  )
}
