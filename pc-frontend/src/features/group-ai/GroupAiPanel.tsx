import { useEffect, useMemo, useState } from 'react'
import {
  Bot,
  Check,
  GitBranch,
  RefreshCw,
  ShieldCheck,
} from 'lucide-react'
import type { Channel } from '../conversation/types'
import {
  authorizeGroupAiNode,
  createMatterPlan,
  loadGroupAiBots,
  loadGroupAiMatters,
  loadGroupAiNodes,
  loadMatterDetail,
  loadMatterEvents,
  postAssignmentAction,
  postMatterAutomation,
  postMatterAction,
} from './api'
import { modeLabel, statusLabel } from './labels'
import MatterDetailView from './MatterDetailView'
import type {
  AssignmentAction,
  AvailableGroupAiNode,
  MatterAutomationAction,
  ProjectAiBot,
  ProjectAiEvent,
  ProjectAiMatter,
  ProjectAiMatterAssignment,
} from './types'
import styles from './GroupAiPanel.module.css'

type Mode = 'solo' | 'critic' | 'split'

interface Props {
  projectId: string
  channels: Channel[]
}

export default function GroupAiPanel({ projectId, channels }: Props) {
  const aiChannel = useMemo(
    () => channels.find((channel) => channel.kind === 'ai_development') ?? channels[0],
    [channels],
  )
  const [nodes, setNodes] = useState<AvailableGroupAiNode[]>([])
  const [bots, setBots] = useState<ProjectAiBot[]>([])
  const [matters, setMatters] = useState<ProjectAiMatter[]>([])
  const [assignments, setAssignments] = useState<ProjectAiMatterAssignment[]>([])
  const [events, setEvents] = useState<ProjectAiEvent[]>([])
  const [selectedMatterId, setSelectedMatterId] = useState('')
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState('')
  const [error, setError] = useState('')
  const [title, setTitle] = useState('')
  const [brief, setBrief] = useState('')
  const [mode, setMode] = useState<Mode>('critic')
  const [criteria, setCriteria] = useState('实现前确认范围\n输出验证命令和风险\n审核 Bot 独立检查')

  const selectedMatter = matters.find((matter) => matter.id === selectedMatterId) ?? matters[0]
  const latestEventId = events[events.length - 1]?.id ?? ''
  const detailShouldPoll = Boolean(
    selectedMatter?.id &&
      (selectedMatter.status === 'running' ||
        assignments.some((assignment) => assignment.status === 'running')),
  )

  useEffect(() => {
    void refresh()
  }, [projectId])

  useEffect(() => {
    if (!selectedMatter?.id) {
      setAssignments([])
      setEvents([])
      return
    }
    void loadDetail(selectedMatter.id)
  }, [selectedMatter?.id])

  useEffect(() => {
    if (!selectedMatter?.id || !detailShouldPoll) return
    const timer = window.setInterval(() => {
      void loadEventDelta(selectedMatter.id, latestEventId)
      void loadDetail(selectedMatter.id)
    }, 5000)
    return () => window.clearInterval(timer)
  }, [selectedMatter?.id, detailShouldPoll, projectId, latestEventId])

  async function refresh() {
    if (!projectId) return
    setLoading(true)
    setError('')
    try {
      const [nextNodes, nextBots, nextMatters] = await Promise.all([
        loadGroupAiNodes(projectId),
        loadGroupAiBots(projectId),
        loadGroupAiMatters(projectId),
      ])
      setNodes(nextNodes)
      setBots(nextBots)
      setMatters(nextMatters)
      if (!selectedMatterId && nextMatters[0]) setSelectedMatterId(nextMatters[0].id)
    } catch (err) {
      setError((err as { message?: string }).message ?? '群体 AI 数据加载失败')
    } finally {
      setLoading(false)
    }
  }

  async function loadDetail(matterId: string) {
    try {
      const detail = await loadMatterDetail(projectId, matterId)
      setMatters((current) => current.map((item) => (item.id === detail.matter.id ? detail.matter : item)))
      setAssignments(detail.assignments ?? [])
      setEvents(detail.events ?? [])
    } catch {
      setAssignments([])
      setEvents([])
    }
  }

  async function loadEventDelta(matterId: string, after = latestEventId) {
    try {
      const delta = await loadMatterEvents(projectId, matterId, after)
      const nextEvents = delta.events ?? []
      if (!nextEvents.length) return
      setEvents((current) => {
        const seen = new Set(current.map((event) => event.id))
        return [...current, ...nextEvents.filter((event) => !seen.has(event.id))]
      })
    } catch {
      // Full detail polling remains the fallback for stale event streams.
    }
  }

  async function authorize(node: AvailableGroupAiNode) {
    setBusy(`node:${node.node_id}`)
    setError('')
    try {
      await authorizeGroupAiNode(projectId, node)
      await refresh()
    } catch (err) {
      setError((err as { message?: string }).message ?? '节点授权失败')
    } finally {
      setBusy('')
    }
  }

  async function createPlan(e: React.FormEvent) {
    e.preventDefault()
    if (!aiChannel || !brief.trim()) return
    setBusy('create')
    setError('')
    try {
      const detail = await createMatterPlan(projectId, {
        channelId: aiChannel.id,
        title: title.trim() || undefined,
        brief: brief.trim(),
        collaborationMode: mode,
        acceptanceCriteria: criteria
          .split('\n')
          .map((line) => line.trim())
          .filter(Boolean),
      })
      setTitle('')
      setBrief('')
      await refresh()
      setSelectedMatterId(detail.matter.id)
      setAssignments(detail.assignments ?? [])
      setEvents(detail.events ?? [])
    } catch (err) {
      setError((err as { message?: string }).message ?? 'Matter 计划创建失败')
    } finally {
      setBusy('')
    }
  }

  async function matterAction(
    matter: ProjectAiMatter,
    action: 'approve' | 'start' | 'request-changes' | 'accept' | 'cancel',
  ) {
    setBusy(`${action}:${matter.id}`)
    setError('')
    try {
      const detail = await postMatterAction(projectId, matter.id, action)
      setMatters((current) => current.map((item) => (item.id === detail.matter.id ? detail.matter : item)))
      setSelectedMatterId(detail.matter.id)
      setAssignments(detail.assignments ?? [])
      setEvents(detail.events ?? [])
    } catch (err) {
      setError((err as { message?: string }).message ?? 'Matter 操作失败')
    } finally {
      setBusy('')
    }
  }

  async function matterAutomationAction(matter: ProjectAiMatter, action: MatterAutomationAction) {
    setBusy(`${action}:${matter.id}`)
    setError('')
    try {
      const detail = await postMatterAutomation(projectId, matter.id, action)
      setMatters((current) => current.map((item) => (item.id === detail.matter.id ? detail.matter : item)))
      setSelectedMatterId(detail.matter.id)
      setAssignments(detail.assignments ?? [])
      setEvents(detail.events ?? [])
      if (detail.errors?.length) {
        setError(detail.errors.map((item) => `${item.role}: ${item.reason}`).join('\n'))
      }
    } catch (err) {
      setError((err as { message?: string }).message ?? '群体 AI 自动派发失败')
    } finally {
      setBusy('')
    }
  }

  async function assignmentAction(
    matter: ProjectAiMatter,
    assignment: ProjectAiMatterAssignment,
    action: AssignmentAction,
  ) {
    setBusy(`${action}:${assignment.id}`)
    setError('')
    try {
      const detail = await postAssignmentAction(projectId, matter.id, assignment.id, action)
      setMatters((current) => current.map((item) => (item.id === detail.matter.id ? detail.matter : item)))
      setSelectedMatterId(detail.matter.id)
      setAssignments(detail.assignments ?? [])
      setEvents(detail.events ?? [])
    } catch (err) {
      setError((err as { message?: string }).message ?? 'Assignment 操作失败')
    } finally {
      setBusy('')
    }
  }

  return (
    <div className={styles.panel}>
      <header className={styles.topbar}>
        <div>
          <h2>群体 AI 开发</h2>
          <span>{bots.length} 个 Bot · {nodes.filter((node) => node.authorized).length} 个授权节点</span>
        </div>
        <button className={styles.iconBtn} onClick={refresh} disabled={loading} type="button" title="刷新">
          <RefreshCw size={15} aria-hidden="true" />
        </button>
      </header>

      {error && <div className={styles.error}>{error}</div>}

      <section className={styles.grid}>
        <div className={styles.section}>
          <SectionTitle icon={<ShieldCheck size={16} />} title="节点授权" />
          <div className={styles.nodeList}>
            {nodes.map((node) => (
              <div className={styles.nodeRow} key={node.node_id}>
                <span className={node.online ? styles.dotOnline : styles.dotOffline} />
                <div className={styles.nodeMain}>
                  <strong>{node.display_name}</strong>
                  <span>{node.allowed_clis.length ? node.allowed_clis.join(' / ') : '未上报 AI CLI'}</span>
                </div>
                <button
                  className={node.authorized ? styles.ghostBtn : styles.primaryBtn}
                  disabled={node.authorized || busy === `node:${node.node_id}`}
                  onClick={() => authorize(node)}
                  type="button"
                >
                  {node.authorized ? <Check size={14} /> : <ShieldCheck size={14} />}
                  {node.authorized ? '已授权' : '授权'}
                </button>
              </div>
            ))}
            {!nodes.length && <div className={styles.empty}>暂无可用节点</div>}
          </div>
        </div>

        <form className={styles.section} onSubmit={createPlan}>
          <SectionTitle icon={<Bot size={16} />} title="创建 Matter" />
          <input
            className={styles.input}
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            placeholder="标题"
          />
          <textarea
            className={styles.textarea}
            value={brief}
            onChange={(event) => setBrief(event.target.value)}
            placeholder="要交付的功能、问题或改动"
          />
          <div className={styles.segmented}>
            {(['solo', 'critic', 'split'] as Mode[]).map((item) => (
              <button
                className={mode === item ? styles.segmentActive : ''}
                key={item}
                onClick={() => setMode(item)}
                type="button"
              >
                {modeLabel(item)}
              </button>
            ))}
          </div>
          <textarea
            className={styles.criteria}
            value={criteria}
            onChange={(event) => setCriteria(event.target.value)}
          />
          <button className={styles.primaryBtn} disabled={!aiChannel || !brief.trim() || busy === 'create'} type="submit">
            <GitBranch size={14} />
            生成计划
          </button>
        </form>
      </section>

      <section className={styles.matters}>
        <div className={styles.list}>
          <SectionTitle icon={<GitBranch size={16} />} title="Matter" />
          {matters.map((matter) => (
            <button
              className={[styles.matterItem, selectedMatter?.id === matter.id ? styles.matterActive : ''].join(' ')}
              key={matter.id}
              onClick={() => setSelectedMatterId(matter.id)}
              type="button"
            >
              <span className={styles.status}>{statusLabel(matter.status, matter.final_decision)}</span>
              <strong>{matter.title}</strong>
              <small>{modeLabel(matter.collaboration_mode)} · {new Date(matter.updated_at).toLocaleString('zh-CN')}</small>
            </button>
          ))}
          {!matters.length && <div className={styles.empty}>暂无 Matter</div>}
        </div>

        <div className={styles.detail}>
          {selectedMatter ? (
            <MatterDetailView
              projectId={projectId}
              matter={selectedMatter}
              assignments={assignments}
              events={events}
              busy={busy}
              onAction={matterAction}
              onAutomationAction={matterAutomationAction}
              onAssignmentAction={assignmentAction}
            />
          ) : (
            <div className={styles.empty}>选择一个 Matter</div>
          )}
        </div>
      </section>
    </div>
  )
}

function SectionTitle({ icon, title }: { icon: React.ReactNode; title: string }) {
  return (
    <div className={styles.sectionTitle}>
      {icon}
      <span>{title}</span>
    </div>
  )
}
