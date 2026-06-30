import { useEffect, useMemo, useState } from 'react'
import {
  AlertTriangle,
  Bot,
  Check,
  CirclePause,
  GitBranch,
  Play,
  RefreshCw,
  RotateCcw,
  ShieldCheck,
  X,
} from 'lucide-react'
import type { Channel } from '../conversation/types'
import {
  authorizeGroupAiNode,
  createMatterPlan,
  loadGroupAiBots,
  loadGroupAiMatters,
  loadGroupAiNodes,
  loadMatterDetail,
  postAssignmentAction,
  postMatterAction,
} from './api'
import type {
  AssignmentAction,
  AvailableGroupAiNode,
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
      setAssignments(detail.assignments ?? [])
      setEvents(detail.events ?? [])
    } catch {
      setAssignments([])
      setEvents([])
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
            <MatterDetail
              matter={selectedMatter}
              assignments={assignments}
              events={events}
              busy={busy}
              onAction={matterAction}
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

function MatterDetail({
  matter,
  assignments,
  events,
  busy,
  onAction,
  onAssignmentAction,
}: {
  matter: ProjectAiMatter
  assignments: ProjectAiMatterAssignment[]
  events: ProjectAiEvent[]
  busy: string
  onAction: (matter: ProjectAiMatter, action: 'approve' | 'start' | 'request-changes' | 'accept' | 'cancel') => void
  onAssignmentAction: (
    matter: ProjectAiMatter,
    assignment: ProjectAiMatterAssignment,
    action: AssignmentAction,
  ) => void
}) {
  return (
    <>
      <header className={styles.detailHeader}>
        <div>
          <h3>{matter.title}</h3>
          <span>{statusLabel(matter.status, matter.final_decision)} · {modeLabel(matter.collaboration_mode)}</span>
        </div>
        <div className={styles.actions}>
          <ActionButton icon={<Check size={14} />} label="批准" disabled={isDone(matter)} busy={busy === `approve:${matter.id}`} onClick={() => onAction(matter, 'approve')} />
          <ActionButton icon={<Play size={14} />} label="启动" disabled={isDone(matter) || matter.status === 'running'} busy={busy === `start:${matter.id}`} onClick={() => onAction(matter, 'start')} />
          <ActionButton icon={<CirclePause size={14} />} label="打回" disabled={isDone(matter)} busy={busy === `request-changes:${matter.id}`} onClick={() => onAction(matter, 'request-changes')} />
          <ActionButton icon={<Check size={14} />} label="验收" disabled={matter.status === 'canceled'} busy={busy === `accept:${matter.id}`} onClick={() => onAction(matter, 'accept')} />
          <ActionButton icon={<X size={14} />} label="取消" disabled={matter.status === 'done'} busy={busy === `cancel:${matter.id}`} onClick={() => onAction(matter, 'cancel')} />
        </div>
      </header>

      <p className={styles.brief}>{matter.brief}</p>
      <div className={styles.criteriaList}>
        {matter.acceptance_criteria.map((item) => <span key={item}>{item}</span>)}
      </div>

      <div className={styles.columns}>
        <div>
          <SectionTitle icon={<Bot size={16} />} title="Assignments" />
          {assignments.map((assignment) => (
            <div className={styles.assignment} key={assignment.id}>
              <div className={styles.assignmentTop}>
                <strong>{assignment.role}</strong>
                <span>{assignmentStatusLabel(assignment.status)}</span>
              </div>
              <span>{assignment.cli_name} · {assignment.runtime_route}</span>
              <small>{assignment.node_id}{assignment.branch_name ? ` · ${assignment.branch_name}` : ''}</small>
              {assignment.result_summary && <p>{assignment.result_summary}</p>}
              <div className={styles.assignmentActions}>
                <ActionButton
                  icon={<Check size={14} />}
                  label="完成"
                  disabled={assignment.status === 'completed' || assignment.status === 'settled'}
                  busy={busy === `complete:${assignment.id}`}
                  onClick={() => onAssignmentAction(matter, assignment, 'complete')}
                />
                <ActionButton
                  icon={<AlertTriangle size={14} />}
                  label="失败"
                  disabled={assignment.status === 'failed'}
                  busy={busy === `fail:${assignment.id}`}
                  onClick={() => onAssignmentAction(matter, assignment, 'fail')}
                />
                <ActionButton
                  icon={<RotateCcw size={14} />}
                  label="重试"
                  disabled={matter.status === 'done' || matter.status === 'canceled'}
                  busy={busy === `retry:${assignment.id}`}
                  onClick={() => onAssignmentAction(matter, assignment, 'retry')}
                />
              </div>
            </div>
          ))}
          {!assignments.length && <div className={styles.empty}>尚未分配</div>}
        </div>

        <div>
          <SectionTitle icon={<GitBranch size={16} />} title="Events" />
          {events.map((event) => (
            <div className={styles.event} key={event.id}>
              <strong>{event.event_type}</strong>
              <span>{new Date(event.created_at).toLocaleString('zh-CN')}</span>
              {eventHint(event) && <small>{eventHint(event)}</small>}
            </div>
          ))}
          {!events.length && <div className={styles.empty}>暂无事件</div>}
        </div>
      </div>
    </>
  )
}

function ActionButton({ icon, label, disabled, busy, onClick }: {
  icon: React.ReactNode
  label: string
  disabled?: boolean
  busy?: boolean
  onClick: () => void
}) {
  return (
    <button className={styles.ghostBtn} disabled={disabled || busy} onClick={onClick} type="button">
      {icon}
      {busy ? '处理中' : label}
    </button>
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

function modeLabel(mode: string) {
  if (mode === 'split') return 'Split'
  if (mode === 'critic') return 'Critic'
  return 'Solo'
}

function statusLabel(status: string, decision?: string | null) {
  if (status === 'done') return '已完成'
  if (status === 'review_ready') return '待验收'
  if (status === 'running') return '运行中'
  if (status === 'canceled') return '已取消'
  if (status === 'failed') return '失败'
  if (decision === 'approved') return '已批准'
  if (decision === 'changes_requested') return '待调整'
  return '计划就绪'
}

function assignmentStatusLabel(status: string) {
  if (status === 'completed') return '已完成'
  if (status === 'settled') return '已结算'
  if (status === 'settled_no_provider') return '无提供者结算'
  if (status === 'failed') return '失败'
  if (status === 'running') return '执行中'
  return '待执行'
}

function eventHint(event: ProjectAiEvent) {
  const payload = event.payload ?? {}
  const computeCallId = stringPayload(payload, 'compute_call_id')
  const assignmentId = stringPayload(payload, 'assignment_id')
  const accountingStatus = stringPayload(payload, 'accounting_status')
  const parts = [computeCallId && `compute ${computeCallId}`, assignmentId && `assignment ${assignmentId}`, accountingStatus]
    .filter((item): item is string => Boolean(item))
    .slice(0, 2)
  return parts.join(' · ')
}

function stringPayload(payload: Record<string, unknown>, key: string) {
  const value = payload[key]
  return typeof value === 'string' && value.trim() ? value.trim() : ''
}

function isDone(matter: ProjectAiMatter) {
  return matter.status === 'done' || matter.status === 'canceled'
}
