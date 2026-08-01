import { useEffect, useMemo, useState } from 'react'
import { Cpu, Save } from 'lucide-react'
import { fetchNodeComputeSharing, nodeId, updateNodeComputeSharing } from './nodeHelpers'
import type { NodeComputeSharingResponse, NodeSummary } from './types'
import styles from './NodePage.module.css'

export default function NodeComputeSharingCard({ node }: { node: NodeSummary }) {
  const id = nodeId(node)
  const [response, setResponse] = useState<NodeComputeSharingResponse | null>(
    node.compute_sharing ? { compute_sharing: node.compute_sharing, observed_models: node.models } : null,
  )
  const [enabled, setEnabled] = useState(node.compute_sharing?.policy.enabled ?? false)
  const [allowedModels, setAllowedModels] = useState<string[]>(node.compute_sharing?.policy.allowed_model_ids ?? [])
  const [maxConcurrent, setMaxConcurrent] = useState(node.compute_sharing?.policy.max_concurrent_runs ?? 1)
  const [dailyTokenLimit, setDailyTokenLimit] = useState(node.compute_sharing?.policy.daily_token_limit ?? 0)
  const [busy, setBusy] = useState(false)
  const [notice, setNotice] = useState('')
  const [error, setError] = useState('')

  useEffect(() => {
    let active = true
    fetchNodeComputeSharing(id)
      .then((next) => {
        if (!active) return
        applyResponse(next)
      })
      .catch((reason) => {
        if (active) setError((reason as Error).message || '算力共享策略读取失败')
      })
    return () => { active = false }
  }, [id])

  const observedModels = useMemo(() => {
    const values = (response?.observed_models ?? node.models ?? [])
      .map((model) => String(model.model_id ?? '').trim())
      .filter(Boolean)
    return Array.from(new Set(values))
  }, [node.models, response?.observed_models])

  function applyResponse(next: NodeComputeSharingResponse) {
    setResponse(next)
    setEnabled(next.compute_sharing.policy.enabled)
    setAllowedModels(next.compute_sharing.policy.allowed_model_ids)
    setMaxConcurrent(next.compute_sharing.policy.max_concurrent_runs)
    setDailyTokenLimit(next.compute_sharing.policy.daily_token_limit)
  }

  function changeEnabled(next: boolean) {
    setEnabled(next)
    if (next && allowedModels.length === 0) setAllowedModels(observedModels)
  }

  function toggleModel(modelId: string) {
    setAllowedModels((current) => current.includes(modelId)
      ? current.filter((item) => item !== modelId)
      : [...current, modelId])
  }

  async function save() {
    setBusy(true)
    setNotice('')
    setError('')
    try {
      const next = await updateNodeComputeSharing(id, {
        enabled,
        allowed_model_ids: allowedModels,
        max_concurrent_runs: Math.max(1, Math.min(16, Math.trunc(maxConcurrent || 1))),
        daily_token_limit: Math.max(0, Math.trunc(dailyTokenLimit || 0)),
      })
      applyResponse(next)
      setNotice(next.compute_sharing.policy.enabled ? '模型算力共享策略已生效。' : '模型算力共享已关闭。')
    } catch (reason) {
      setError((reason as Error).message || '算力共享策略保存失败')
    } finally {
      setBusy(false)
    }
  }

  const status = response?.compute_sharing
  return (
    <section className={styles.computeSharingCard}>
      <header>
        <div><Cpu size={16} /><strong>模型算力共享</strong></div>
        <label className={styles.computeSharingToggle}>
          <input type="checkbox" checked={enabled} onChange={(event) => changeEnabled(event.target.checked)} />
          <span>{enabled ? '已开启' : '未开启'}</span>
        </label>
      </header>
      <p>自己的节点始终可自用；只有这里明确选择的本地模型才允许其他用户调度。</p>

      <div className={styles.computeSharingModels}>
        {observedModels.map((modelId) => (
          <label key={modelId}>
            <input
              type="checkbox"
              checked={allowedModels.includes(modelId)}
              onChange={() => toggleModel(modelId)}
            />
            <span>{modelId}</span>
          </label>
        ))}
        {!observedModels.length && <small>节点当前未上报可共享的本地模型。</small>}
      </div>

      <div className={styles.computeSharingLimits}>
        <label>最大并发<input type="number" min={1} max={16} value={maxConcurrent} onChange={(event) => setMaxConcurrent(Number(event.target.value))} /></label>
        <label>每日共享 Token 预算<input type="number" min={0} step={1000} value={dailyTokenLimit} onChange={(event) => setDailyTokenLimit(Number(event.target.value))} /></label>
      </div>
      <div className={styles.computeSharingStatus}>
        <span>当前任务 {status?.active_runs ?? 0}/{maxConcurrent}</span>
        <span>今日实耗 {formatCount(status?.tokens_used_today ?? 0)}{dailyTokenLimit > 0 ? `/${formatCount(dailyTokenLimit)}` : '（不限）'}</span>
        <span>执行中预留 {formatCount(status?.tokens_reserved_today ?? 0)}</span>
      </div>
      {notice && <div className={styles.computeSharingNotice}>{notice}</div>}
      {error && <div className={styles.computeSharingError}>{error}</div>}
      <button className={[styles.btn, styles.primary].join(' ')} type="button" onClick={save} disabled={busy || (enabled && allowedModels.length === 0)}>
        <Save size={14} />{busy ? '保存中' : '保存共享策略'}
      </button>
    </section>
  )
}

function formatCount(value: number) {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`
  return `${value}`
}
