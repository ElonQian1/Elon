import { useState, type ReactNode } from 'react'
import {
  AlertTriangle,
  Bot,
  Check,
  CirclePause,
  GitBranch,
  Play,
  RotateCcw,
  ShieldCheck,
  X,
} from 'lucide-react'
import { loadAssignmentArtifact } from './api'
import {
  assignmentStatusLabel,
  canRunAssignment,
  eventHint,
  isDone,
  modeLabel,
  statusLabel,
} from './labels'
import type {
  AssignmentAction,
  AssignmentArtifact,
  MatterAutomationAction,
  ProjectAiEvent,
  ProjectAiMatter,
  ProjectAiMatterAssignment,
} from './types'
import styles from './GroupAiPanel.module.css'

interface Props {
  projectId: string
  matter: ProjectAiMatter
  assignments: ProjectAiMatterAssignment[]
  events: ProjectAiEvent[]
  busy: string
  onAction: (
    matter: ProjectAiMatter,
    action: 'approve' | 'start' | 'request-changes' | 'accept' | 'cancel',
  ) => void
  onAutomationAction: (matter: ProjectAiMatter, action: MatterAutomationAction) => void
  onAssignmentAction: (
    matter: ProjectAiMatter,
    assignment: ProjectAiMatterAssignment,
    action: AssignmentAction,
  ) => void
}

export default function MatterDetailView({
  projectId,
  matter,
  assignments,
  events,
  busy,
  onAction,
  onAutomationAction,
  onAssignmentAction,
}: Props) {
  const [artifact, setArtifact] = useState<AssignmentArtifact | null>(null)
  const [artifactBusy, setArtifactBusy] = useState('')
  const [artifactError, setArtifactError] = useState('')

  async function openArtifact(assignment: ProjectAiMatterAssignment) {
    setArtifactBusy(assignment.id)
    setArtifactError('')
    try {
      const response = await loadAssignmentArtifact(projectId, matter.id, assignment.id)
      setArtifact(response.artifact)
    } catch (err) {
      setArtifactError((err as { message?: string }).message ?? '产物包加载失败')
    } finally {
      setArtifactBusy('')
    }
  }

  return (
    <>
      <header className={styles.detailHeader}>
        <div>
          <h3>{matter.title}</h3>
          <span>{statusLabel(matter.status, matter.final_decision)} · {modeLabel(matter.collaboration_mode)}</span>
        </div>
        <div className={styles.actions}>
          <ActionButton
            icon={<Check size={14} />}
            label="批准"
            disabled={isDone(matter)}
            busy={busy === `approve:${matter.id}`}
            onClick={() => onAction(matter, 'approve')}
          />
          <ActionButton
            icon={<Play size={14} />}
            label="启动"
            disabled={isDone(matter) || matter.status === 'running'}
            busy={busy === `start:${matter.id}`}
            onClick={() => onAction(matter, 'start')}
          />
          <ActionButton
            icon={<Play size={14} />}
            label="执行实现"
            disabled={isDone(matter)}
            busy={busy === `run-all:${matter.id}`}
            onClick={() => onAutomationAction(matter, 'run-all')}
          />
          <ActionButton
            icon={<ShieldCheck size={14} />}
            label="Review"
            disabled={isDone(matter)}
            busy={busy === `review:${matter.id}`}
            onClick={() => onAutomationAction(matter, 'review')}
          />
          <ActionButton
            icon={<CirclePause size={14} />}
            label="打回"
            disabled={isDone(matter)}
            busy={busy === `request-changes:${matter.id}`}
            onClick={() => onAction(matter, 'request-changes')}
          />
          <ActionButton
            icon={<Check size={14} />}
            label="验收"
            disabled={matter.status === 'canceled'}
            busy={busy === `accept:${matter.id}`}
            onClick={() => onAction(matter, 'accept')}
          />
          <ActionButton
            icon={<X size={14} />}
            label="取消"
            disabled={matter.status === 'done'}
            busy={busy === `cancel:${matter.id}`}
            onClick={() => onAction(matter, 'cancel')}
          />
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
              <small>
                {assignment.node_id}
                {assignment.branch_name ? ` · ${assignment.branch_name}` : ''}
                {assignment.worktree_path ? ` · ${assignment.worktree_path}` : ''}
              </small>
              {assignment.result_summary && <p>{assignment.result_summary}</p>}
              <div className={styles.assignmentActions}>
                <ActionButton
                  icon={<Play size={14} />}
                  label="执行"
                  disabled={!canRunAssignment(matter, assignment)}
                  busy={busy === `run:${assignment.id}`}
                  onClick={() => onAssignmentAction(matter, assignment, 'run')}
                />
                <ActionButton
                  icon={<GitBranch size={14} />}
                  label="产物包"
                  busy={artifactBusy === assignment.id}
                  onClick={() => openArtifact(assignment)}
                />
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
          {artifactError && <div className={styles.inlineError}>{artifactError}</div>}
          {artifact && <ArtifactPanel artifact={artifact} />}
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

function ArtifactPanel({ artifact }: { artifact: AssignmentArtifact }) {
  const quality = artifact.node_quality
  const compute = artifact.compute_run
  const session = artifact.execution_session
  return (
    <div className={styles.artifactPanel}>
      <div className={styles.artifactHeader}>
        <strong>{artifact.assignment.role}</strong>
        <span>{artifact.merge.manual_merge_required ? '待人工合并' : '未生成合并产物'}</span>
      </div>
      <p>{artifact.merge.recommended_action}</p>
      <div className={styles.artifactGrid}>
        <ArtifactField label="branch" value={artifact.merge.branch_name} />
        <ArtifactField label="worktree" value={artifact.merge.worktree_path} />
        <ArtifactField label="compute" value={artifact.compute_call_id} />
        <ArtifactField label="merge" value={artifact.merge.merge_status ?? session?.merge_status} />
        <ArtifactField label="run" value={compute?.status} />
        <ArtifactField label="quality" value={quality ? `${quality.success_rate_x1000 / 10}% · ${quality.total_runs} runs` : ''} />
        <ArtifactField label="cost" value={compute ? `${compute.billed_cost_rmb_fen} 分` : ''} />
        <ArtifactField label="earned" value={compute ? `${compute.provider_earned_fen} 分` : ''} />
      </div>
      {artifact.local_diff.available ? (
        <div className={styles.diffBox}>
          {[...artifact.local_diff.status_short, ...artifact.local_diff.diff_stat].map((line) => (
            <code key={line}>{line}</code>
          ))}
        </div>
      ) : (
        <small className={styles.artifactMuted}>{artifact.local_diff.reason}</small>
      )}
    </div>
  )
}

function ArtifactField({ label, value }: { label: string; value?: string | number | null }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value || '-'}</strong>
    </div>
  )
}

function ActionButton({ icon, label, disabled, busy, onClick }: {
  icon: ReactNode
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

function SectionTitle({ icon, title }: { icon: ReactNode; title: string }) {
  return (
    <div className={styles.sectionTitle}>
      {icon}
      <span>{title}</span>
    </div>
  )
}
