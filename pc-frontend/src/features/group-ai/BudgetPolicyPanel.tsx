import { Save, Wallet } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { updateMatterBudgetPolicy } from './api'
import type { MatterGovernanceSummary } from './types'
import styles from './BudgetPolicyPanel.module.css'

interface Props {
  projectId: string
  matterId: string
  governance: MatterGovernanceSummary
  onChanged: () => void
}

export default function BudgetPolicyPanel({ projectId, matterId, governance, onChanged }: Props) {
  const budget = useMemo(() => policyBudget(governance), [governance])
  const [maxFen, setMaxFen] = useState('')
  const [pause, setPause] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    setMaxFen(typeof budget.max === 'number' && budget.max > 0 ? String(budget.max) : '')
    setPause(budget.pause)
  }, [budget.max, budget.pause])

  async function save() {
    setBusy(true)
    setError('')
    try {
      await updateMatterBudgetPolicy(projectId, matterId, {
        maxBilledCostRmbFen: maxFen.trim() ? Math.max(0, Number(maxFen.trim())) : null,
        pauseOnBudgetExceeded: pause,
      })
      onChanged()
    } catch (err) {
      setError((err as { message?: string }).message ?? '预算策略保存失败')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className={styles.panel}>
      <div className={styles.title}>
        <Wallet size={15} />
        <span>预算策略</span>
      </div>
      <div className={styles.controls}>
        <label>
          <span>上限</span>
          <input
            inputMode="numeric"
            min={0}
            onChange={(event) => setMaxFen(event.target.value.replace(/[^\d]/g, ''))}
            placeholder="不限"
            type="text"
            value={maxFen}
          />
        </label>
        <label className={styles.check}>
          <input checked={pause} onChange={(event) => setPause(event.target.checked)} type="checkbox" />
          <span>超限暂停派发</span>
        </label>
        <button disabled={busy} onClick={save} type="button">
          <Save size={13} />
          {busy ? '保存中' : '保存'}
        </button>
      </div>
      <small>
        已用 {governance.budget.billed_cost_rmb_fen} 分
        {typeof governance.budget.remaining_billed_cost_rmb_fen === 'number'
          ? ` · 剩余 ${governance.budget.remaining_billed_cost_rmb_fen} 分`
          : ''}
      </small>
      {error && <p>{error}</p>}
    </div>
  )
}

function policyBudget(governance: MatterGovernanceSummary) {
  const budget = governance.policy.node_policy?.budget
  const record = isRecord(budget) ? budget : {}
  return {
    max: numberValue(record.max_billed_cost_rmb_fen),
    pause: typeof record.pause_on_budget_exceeded === 'boolean'
      ? record.pause_on_budget_exceeded
      : true,
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function numberValue(value: unknown) {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}
