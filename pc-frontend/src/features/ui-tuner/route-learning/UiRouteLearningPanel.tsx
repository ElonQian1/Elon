import { useCallback, useEffect, useState } from 'react'
import { BrainCircuit, ChevronDown, ChevronRight, RotateCcw } from 'lucide-react'
import { useProjectStore } from '../../conversation/useProjectStore'
import {
  confirmUiRouteLearning,
  listUiRouteLearning,
  revokeUiRouteLearning,
  type UiLearnedRoute,
  type UiRouteLearningEntry,
} from './routeLearningApi'
import styles from './UiRouteLearningPanel.module.css'

interface Props {
  currentIntent: string
}

export function UiRouteLearningPanel({ currentIntent }: Props) {
  const projectId = useProjectStore((state) => state.activeProjectId)
  const [open, setOpen] = useState(false)
  const [entries, setEntries] = useState<UiRouteLearningEntry[]>([])
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const reload = useCallback(async () => {
    if (!projectId) return
    setBusy(true)
    setMessage('')
    try {
      setEntries(await listUiRouteLearning(projectId))
    } catch (error) {
      setMessage(apiErrorMessage(error, '加载判断经验失败'))
    } finally {
      setBusy(false)
    }
  }, [projectId])

  useEffect(() => {
    if (open) void reload()
  }, [open, reload])

  async function confirm(text: string, route: UiLearnedRoute) {
    if (!projectId || !text.trim()) return
    setBusy(true)
    setMessage('')
    try {
      await confirmUiRouteLearning({
        projectId,
        message: text.trim(),
        route,
        reason: '用户在 PC 微调画布明确确认任务路由',
      })
      setMessage(route === 'ui' ? '已记为 UI 任务；下次将零 Token 命中' : '已记为普通开发；下次不会误入 UI 链路')
      setEntries(await listUiRouteLearning(projectId))
    } catch (error) {
      setMessage(apiErrorMessage(error, '保存判断经验失败'))
    } finally {
      setBusy(false)
    }
  }

  async function revoke(entry: UiRouteLearningEntry) {
    if (!projectId) return
    setBusy(true)
    setMessage('')
    try {
      await revokeUiRouteLearning({
        projectId,
        entryId: entry.id,
        reason: '用户在 PC 微调画布撤销错误经验',
      })
      setMessage('已撤销，后续将重新判断')
      setEntries(await listUiRouteLearning(projectId))
    } catch (error) {
      setMessage(apiErrorMessage(error, '撤销判断经验失败'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={styles.panel}>
      <button className={styles.heading} type="button" onClick={() => setOpen((value) => !value)}>
        {open ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        <BrainCircuit size={15} />
        <span>UI 判断经验库</span>
        <small>{entries.filter((entry) => entry.status === 'active').length} 条已生效</small>
      </button>
      {open && (
        <div className={styles.body}>
          <p>Codex 建议只进入候选；受控近义词由本地规则归簇，只有用户确认或真实执行成功的经验才能被复用。</p>
          <div className={styles.currentActions}>
            <button disabled={busy || !projectId || !currentIntent.trim()} onClick={() => void confirm(currentIntent, 'ui')}>
              当前意图是 UI
            </button>
            <button disabled={busy || !projectId || !currentIntent.trim()} onClick={() => void confirm(currentIntent, 'non_ui')}>
              当前意图是普通开发
            </button>
          </div>
          {message && <div className={styles.notice}>{message}</div>}
          {!projectId && <div className={styles.notice}>请先选择项目</div>}
          {projectId && entries.length === 0 && !busy && <div className={styles.empty}>还没有判断经验</div>}
          <div className={styles.list}>
            {entries.map((entry) => (
              <article key={entry.id} data-status={entry.status}>
                <div className={styles.entryHead}>
                  <strong>{entry.learnedRoute === 'ui' ? 'UI 任务' : '普通开发'}</strong>
                  <span>{statusLabel(entry.status)}</span>
                  <small>{sourceLabel(entry.source)}</small>
                </div>
                <p title={entry.sampleText}>{entry.sampleText}</p>
                {entry.conceptLabel && (
                  <div className={styles.cluster} title={entry.conceptKey}>
                    <strong>受控近义簇</strong>
                    <span>{entry.conceptLabel}</span>
                    <small>规则 v{entry.conceptVersion}</small>
                  </div>
                )}
                {entry.aliases.length > 0 && (
                  <div className={styles.aliases}>
                    {entry.aliases.map((alias) => (
                      <span key={alias.id} title={`本地受控词表命中 ${alias.hitCount} 次`}>
                        {alias.sampleText} · {alias.hitCount}
                      </span>
                    ))}
                  </div>
                )}
                <div className={styles.metrics}>
                  <span>命中 {entry.hitCount}</span>
                  <span>近义命中 {entry.clusterHitCount}</span>
                  <span>别名 {entry.aliasCount}</span>
                  <span>证据 {entry.evidenceCount}</span>
                  <span>冲突 {entry.conflictCount}</span>
                  <span>置信 {Math.round(entry.confidence * 100)}%</span>
                </div>
                <div className={styles.entryActions}>
                  {entry.status !== 'active' && (
                    <>
                      <button disabled={busy} onClick={() => void confirm(entry.sampleText, 'ui')}>确认 UI</button>
                      <button disabled={busy} onClick={() => void confirm(entry.sampleText, 'non_ui')}>确认普通</button>
                    </>
                  )}
                  {entry.status === 'active' && (
                    <button disabled={busy} onClick={() => void revoke(entry)}>
                      <RotateCcw size={12} />撤销
                    </button>
                  )}
                </div>
              </article>
            ))}
          </div>
        </div>
      )}
    </section>
  )
}

function statusLabel(status: UiRouteLearningEntry['status']) {
  if (status === 'active') return '已生效'
  if (status === 'revoked') return '已撤销'
  return '待确认'
}

function sourceLabel(source: UiRouteLearningEntry['source']) {
  if (source === 'codex_proposal') return 'Codex 候选'
  if (source === 'user_override') return '用户确认'
  if (source === 'execution_verified') return '执行验证'
  if (source === 'runtime_verified') return 'Runtime 验证'
  return '管理员'
}

function apiErrorMessage(error: unknown, fallback: string) {
  if (error && typeof error === 'object' && 'message' in error && typeof error.message === 'string') {
    return error.message
  }
  return fallback
}
