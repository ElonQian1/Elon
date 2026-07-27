import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { FileInput, FileText, GitFork, Network, RefreshCw, Search, Sparkles } from 'lucide-react'
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

import { api } from '../../api/client'
import ProjectDocumentDiscussionNode, { type DiscussionFlowNode } from './ProjectDocumentDiscussionNode'
import ProjectDocumentDiscussionTimeline, { type DiscussionVersion } from './ProjectDocumentDiscussionTimeline'
import {
  discussionKindLabel,
  discussionRoots,
  discussionStatusLabel,
  EMPTY_DISCUSSION_GRAPH,
  layoutDiscussionNodes,
  parseDiscussionGraph,
  selectDiscussionSubgraph,
  type DiscussionGraph,
  type DiscussionNode,
} from './projectDocumentDiscussionModel'
import type { DocumentFile } from './projectDocumentModel'
import type { DocumentOrganizationTrackingRuntime } from './projectDocumentOrganizationStatus'
import { projectDocumentErrorMessage } from './projectDocumentWorkspaceHelpers'
import styles from './ProjectDocumentDiscussionMap.module.css'

const GRAPH_PATH = '.elon/discussion-graph.json'
const SUGGESTIONS_PATH = '.elon/discussion-graph-suggestions.json'
const MAX_IMPORT_BYTES = 1_800_000
const nodeTypes = { discussion: ProjectDocumentDiscussionNode }

interface Props {
  projectId: string
  canEdit: boolean
  canStartAi: boolean
  organizing: boolean
  runtime: DocumentOrganizationTrackingRuntime
  onOpenDocument: (path: string) => void
  onStructureSource: (path: string) => void
  onDiscussNode: (node: DiscussionNode, mode: 'continue' | 'fork' | 'promote') => void
  onApplyPending: () => void
  onRunAi: (instruction: string) => void
}

export default function ProjectDocumentDiscussionMap(props: Props) {
  return (
    <ReactFlowProvider>
      <DiscussionMapSurface {...props} />
    </ReactFlowProvider>
  )
}

function DiscussionMapSurface({
  projectId,
  canEdit,
  canStartAi,
  organizing,
  runtime,
  onOpenDocument,
  onStructureSource,
  onDiscussNode,
  onApplyPending,
  onRunAi,
}: Props) {
  const [graph, setGraph] = useState<DiscussionGraph>(EMPTY_DISCUSSION_GRAPH)
  const [currentGraph, setCurrentGraph] = useState<DiscussionGraph>(EMPTY_DISCUSSION_GRAPH)
  const [activeVersion, setActiveVersion] = useState<DiscussionVersion | null>(null)
  const [loading, setLoading] = useState(true)
  const [message, setMessage] = useState('')
  const [query, setQuery] = useState('')
  const [rootId, setRootId] = useState('')
  const [selectedId, setSelectedId] = useState('')
  const [pending, setPending] = useState<{ summary: string; nodes: number; promotions: number } | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const { fitView } = useReactFlow()

  const showGraph = useCallback((next: DiscussionGraph) => {
    setGraph(next)
    setRootId((current) => next.nodes.some((node) => node.id === current) ? current : discussionRoots(next)[0]?.id ?? '')
    setSelectedId((current) => next.nodes.some((node) => node.id === current) ? current : discussionRoots(next)[0]?.id ?? '')
  }, [])

  const load = useCallback(async () => {
    setLoading(true)
    setMessage('')
    const read = (path: string) => api.get<DocumentFile>(
      `/api/projects/${encodeURIComponent(projectId)}/docs/file?path=${encodeURIComponent(path)}`,
    )
    const [graphFile, proposalFile] = await Promise.allSettled([read(GRAPH_PATH), read(SUGGESTIONS_PATH)])
    if (graphFile.status === 'fulfilled') {
      const next = parseDiscussionGraph(graphFile.value.content)
      setCurrentGraph(next)
      setActiveVersion(null)
      showGraph(next)
    } else {
      setCurrentGraph(EMPTY_DISCUSSION_GRAPH)
      setGraph(EMPTY_DISCUSSION_GRAPH)
      setRootId('')
      setSelectedId('')
    }
    if (proposalFile.status === 'fulfilled') setPending(parsePendingProposal(proposalFile.value.content))
    else setPending(null)
    setLoading(false)
  }, [projectId, showGraph])

  useEffect(() => { void load() }, [load])

  const selection = useMemo(
    () => selectDiscussionSubgraph(graph, rootId, query),
    [graph, query, rootId],
  )
  const positions = useMemo(() => layoutDiscussionNodes(selection.nodes), [selection.nodes])
  const childCounts = useMemo(() => {
    const counts = new Map<string, number>()
    graph.nodes.forEach((node) => counts.set(node.parent_id, (counts.get(node.parent_id) ?? 0) + 1))
    return counts
  }, [graph])
  const flowNodes = useMemo<DiscussionFlowNode[]>(() => selection.nodes.map((node) => ({
    id: node.id,
    type: 'discussion',
    position: positions.get(node.id) ?? { x: 0, y: 0 },
    data: { discussion: node, childCount: childCounts.get(node.id) ?? 0 },
    selected: selectedId === node.id,
    draggable: false,
  })), [childCounts, positions, selectedId, selection.nodes])
  const flowEdges = useMemo<Edge[]>(() => selection.edges.map((edge) => {
    const color = edge.relation === 'opposes' ? '#d66f78'
      : edge.relation === 'supports' ? '#55b989'
        : edge.relation === 'alternative_to' ? '#d8a950' : '#8270ad'
    return {
      ...edge,
      type: 'smoothstep',
      label: edge.label || relationLabel(edge.relation),
      animated: ['spawns', 'leads_to'].includes(edge.relation),
      markerEnd: { type: MarkerType.ArrowClosed, width: 12, height: 12, color },
      style: { stroke: color, strokeWidth: 1.5 },
    }
  }), [selection.edges])
  const roots = discussionRoots(graph)
  const activeRoot = roots.find((root) => root.id === rootId)
  const selectedNode = graph.nodes.find((node) => node.id === selectedId)
  const visibleKey = selection.nodes.map((node) => node.id).join('|')

  useEffect(() => {
    const timer = window.setTimeout(() => { void fitView({ padding: .2, duration: 250, maxZoom: 1.15 }) }, 40)
    return () => window.clearTimeout(timer)
  }, [fitView, visibleKey])

  async function importConversation(file: File) {
    if (!canEdit) return
    if (file.size > MAX_IMPORT_BYTES) {
      setMessage('聊天文件超过 1.8 MB；请先按会话或主题拆成较小文件。')
      return
    }
    const raw = await file.text()
    const defaultTitle = file.name.replace(/\.[^.]+$/, '') || '导入聊天'
    const title = window.prompt('这段聊天的来源标题', defaultTitle)?.trim()
    if (!title) return
    const timestamp = new Date().toISOString().replace(/\D/g, '').slice(0, 17)
    const path = `docs/inbox/conversations/${timestamp}-conversation.md`
    const safeTitle = title.replace(/[\r\n]+/g, ' ').slice(0, 120)
    const content = [
      '---',
      'role: discussion',
      'lifecycle: source_material',
      'authority: none',
      'default_retrieval: false',
      'source_type: imported_conversation',
      `source_file: ${JSON.stringify(file.name)}`,
      '---',
      `# ${safeTitle}`,
      '',
      '> 原始聊天来源，仅用于追溯。未经确认的内容不是项目事实。',
      '',
      raw,
      '',
    ].join('\n')
    try {
      await api.put(`/api/projects/${encodeURIComponent(projectId)}/docs/file`, { path, content })
      setMessage('原始聊天已保存到项目 Git 范围，正在交给当前 Windows 登录账号的 AI CLI 结构化。')
      onStructureSource(path)
    } catch (error) {
      setMessage(projectDocumentErrorMessage(error, '导入聊天失败'))
    } finally {
      if (inputRef.current) inputRef.current.value = ''
    }
  }

  return (
    <main className={styles.mapShell}>
      <header className={styles.toolbar}>
        <div className={styles.title}><span><Network size={18} /></span><div><strong>讨论推理图</strong><small>{activeRoot?.title || '来源 → 分支 → 决策 → 功能与文档'}</small></div></div>
        <select value={rootId} onChange={(event) => { setRootId(event.target.value); setSelectedId(event.target.value) }} aria-label="讨论主题">
          {!roots.length && <option value="">暂无讨论主题</option>}
          {roots.map((root) => <option key={root.id} value={root.id}>{root.title}</option>)}
        </select>
        <label className={styles.search}><Search size={14} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索观点、证据或功能" /></label>
        <button type="button" title="刷新讨论图" onClick={() => { void load() }}><RefreshCw size={14} /></button>
        <input ref={inputRef} hidden type="file" accept=".md,.txt,.json,text/plain,text/markdown,application/json"
          onChange={(event) => { const file = event.target.files?.[0]; if (file) void importConversation(file) }} />
        <button className={styles.importButton} type="button" disabled={!canEdit || organizing || !!activeVersion} onClick={() => inputRef.current?.click()}>
          <FileInput size={14} />导入聊天并整理
        </button>
      </header>
      <div className={styles.notice}>
        <span>{loading ? '正在读取讨论图…' : `${graph.sources.length} 个来源 · ${graph.nodes.length} 个节点 · ${roots.length} 个主题`}</span>
        <small>{activeVersion
          ? `历史版本只读 · ${activeVersion.summary || activeVersion.commit.slice(0, 8)}`
          : message || graph.evolution.summary || '原文保留且默认不检索；AI 只读取当前来源和命中节点，稳定结论才晋升。'}</small>
        {pending && !activeVersion && <button type="button" title={pending.summary} disabled={!canStartAi || organizing} onClick={onApplyPending}>
          待审核：{pending.nodes} 节点 / {pending.promotions} 文档
        </button>}
      </div>
      <ProjectDocumentDiscussionTimeline
        runtime={runtime}
        activeVersion={activeVersion}
        selectedNodeId={selectedId}
        canStartAi={canStartAi}
        organizing={organizing}
        onSelectVersion={(next, version) => { setActiveVersion(version); showGraph(next) }}
        onSelectCurrent={() => { setActiveVersion(null); showGraph(currentGraph) }}
        onRunAi={onRunAi}
      />
      <div className={styles.content}>
        <div className={styles.canvas}>
          {flowNodes.length ? (
            <ReactFlow nodes={flowNodes} edges={flowEdges} nodeTypes={nodeTypes} fitView colorMode="dark"
              minZoom={.25} maxZoom={1.8} nodesConnectable={false} nodesDraggable={false}
              onNodeClick={(_, node) => setSelectedId(node.id)} proOptions={{ hideAttribution: true }}>
              <Background variant={BackgroundVariant.Dots} gap={22} size={1} color="#343943" />
              <MiniMap className={styles.miniMap} pannable zoomable
                nodeColor={(node) => (node.data.discussion as DiscussionNode).color}
                maskColor="rgba(17, 18, 22, .76)" />
              <Controls className={styles.controls} showInteractive={false} />
            </ReactFlow>
          ) : (
            <div className={styles.empty}>
              <GitFork size={28} />
              <strong>还没有讨论知识图</strong>
              <p>导入 ChatGPT、Codex 或其他供应商的聊天导出文件。原文先保存，再由 Windows 节点上的登录 AI CLI 拆成可追溯节点。</p>
              <button type="button" disabled={!canEdit || organizing || !!activeVersion} onClick={() => inputRef.current?.click()}><FileInput size={14} />选择聊天文件</button>
            </div>
          )}
          {selection.truncated && <div className={styles.truncated}>当前只显示前 400 个节点，请选择根主题或搜索。</div>}
        </div>
        <aside className={styles.inspector}>
          {selectedNode ? (
            <>
              <header style={{ '--node-color': selectedNode.color } as React.CSSProperties}>
                <span><GitFork size={16} /></span>
                <div><small>{discussionKindLabel(selectedNode.kind)}</small><strong>{selectedNode.title}</strong></div>
                <em>{discussionStatusLabel(selectedNode.status)}</em>
              </header>
              <p>{selectedNode.summary || '尚无摘要。'}</p>
              <section><strong>可追溯关系</strong>
                <dl>
                  <div><dt>来源</dt><dd>{selectedNode.source_refs.length || '无'}</dd></div>
                  <div><dt>会话</dt><dd>{selectedNode.conversation_refs.length || '无'}</dd></div>
                  <div><dt>正式文档</dt><dd>{selectedNode.document_paths.length || '无'}</dd></div>
                  <div><dt>功能节点</dt><dd>{selectedNode.feature_node_ids.length || '无'}</dd></div>
                </dl>
              </section>
              {!!selectedNode.document_paths.length && <section className={styles.documents}><strong>关联文档</strong>
                {selectedNode.document_paths.map((path) => <button key={path} type="button" onClick={() => onOpenDocument(path)}><FileText size={13} /><span>{path}</span></button>)}
              </section>}
              <div className={styles.actions}>
                <button type="button" disabled={!canStartAi || organizing || !!activeVersion} onClick={() => onDiscussNode(selectedNode, 'continue')}><Sparkles size={14} />继续讨论</button>
                <button type="button" disabled={!canStartAi || organizing || !!activeVersion} onClick={() => onDiscussNode(selectedNode, 'fork')}><GitFork size={14} />创建备选分支</button>
                <button type="button" disabled={!canStartAi || organizing || !!activeVersion} onClick={() => onDiscussNode(selectedNode, 'promote')}><FileText size={14} />晋升为正式文档</button>
              </div>
            </>
          ) : <p className={styles.inspectorEmpty}>选择一个节点查看来源、分支和晋升状态。</p>}
        </aside>
      </div>
    </main>
  )
}

function relationLabel(relation: string) {
  return {
    supports: '支持', opposes: '反对', alternative_to: '备选', depends_on: '依赖',
    leads_to: '导向', spawns: '分叉', resolves: '解决', related_to: '相关',
  }[relation] ?? relation
}

function parsePendingProposal(content: string) {
  try {
    const value = JSON.parse(content)
    if (value?.status !== 'ready') return null
    return {
      summary: String(value.summary ?? ''),
      nodes: Array.isArray(value.graph?.nodes) ? value.graph.nodes.length : 0,
      promotions: Array.isArray(value.promotions) ? value.promotions.length : 0,
    }
  } catch {
    return null
  }
}
