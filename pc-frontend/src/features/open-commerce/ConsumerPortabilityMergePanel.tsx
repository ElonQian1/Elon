import { useCallback, useEffect, useMemo, useState } from 'react'
import { GitMerge, RefreshCw, RotateCcw, Save } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type {
  ConsumerPortabilityImportSummary,
  ConsumerPortabilityMergeAdoption,
  ConsumerPortabilityMergePlan,
} from './openCommerceClientTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

interface Props {
  projectId: string
  imports: ConsumerPortabilityImportSummary[]
  onChanged: () => Promise<void>
}

export default function ConsumerPortabilityMergePanel({ projectId, imports, onChanged }: Props) {
  const [selectedImportIds, setSelectedImportIds] = useState<string[]>([])
  const [plan, setPlan] = useState<ConsumerPortabilityMergePlan | null>(null)
  const [selections, setSelections] = useState<Record<string, string>>({})
  const [adoptions, setAdoptions] = useState<ConsumerPortabilityMergeAdoption[]>([])
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const eligibleImports = useMemo(
    () => imports.filter((item) => item.preference_profile_included),
    [imports],
  )

  const refresh = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listConsumerPortabilityMergeAdoptions(projectId)
      setAdoptions(response.adoptions)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  function toggleImport(importId: string) {
    setSelectedImportIds((current) =>
      current.includes(importId)
        ? current.filter((value) => value !== importId)
        : current.length < 10
          ? [...current, importId]
          : current,
    )
    setPlan(null)
    setSelections({})
  }

  async function preview() {
    if (selectedImportIds.length < 2) {
      setMessage('请选择至少两个包含偏好档案的隔离数据包。')
      return
    }
    setBusy(true)
    setMessage('')
    try {
      setPlan(await openCommerceClientApi.getConsumerPortabilityMergePlan(projectId, selectedImportIds))
      setSelections({})
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function applyMerge() {
    if (!plan) return
    const selectedFields = Object.entries(selections)
      .filter(([, importId]) => Boolean(importId))
      .map(([field, import_id]) => ({ field, import_id }))
    if (selectedFields.length === 0) {
      setMessage('请至少为一个发生变化的字段选择数据来源。')
      return
    }
    if (!window.confirm(`采用已选择的 ${selectedFields.length} 个偏好字段？每个字段的来源将被记录。`)) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.applyConsumerPortabilityMerge(
        projectId,
        plan.sources.map((source) => source.import_id),
        plan.current_profile_revision,
        selectedFields,
      )
      setMessage('多来源偏好已合并，字段来源与回滚快照已保存。')
      setPlan(null)
      setSelections({})
      await Promise.all([refresh(), onChanged()])
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function rollback(adoption: ConsumerPortabilityMergeAdoption) {
    if (!window.confirm('回滚到多来源合并前的偏好？若此后档案已修改，服务端会拒绝覆盖。')) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.rollbackConsumerPortabilityMerge(
        projectId,
        adoption.id,
        adoption.resulting_revision,
      )
      setMessage('多来源偏好合并已回滚。')
      await Promise.all([refresh(), onChanged()])
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={base.integrationSection}>
      <header>
        <span>
          <strong>多来源偏好合并</strong>
          <small>冲突由用户逐字段选择来源；关系、订单和 ERP 数据保持隔离。</small>
        </span>
        <button style={actionStyle('icon', busy)} type="button" onClick={refresh} disabled={busy} title="刷新合并记录">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={{ ...commerceStyles.list, padding: 12 }}>
        {eligibleImports.map((item) => (
          <label key={item.id} style={commerceStyles.checkRow}>
            <input
              type="checkbox"
              checked={selectedImportIds.includes(item.id)}
              disabled={busy || (!selectedImportIds.includes(item.id) && selectedImportIds.length >= 10)}
              onChange={() => toggleImport(item.id)}
            />
            <span>{item.source_operator} · {item.source_package_id}</span>
          </label>
        ))}
        <button
          style={actionStyle('secondary', busy || selectedImportIds.length < 2)}
          type="button"
          onClick={preview}
          disabled={busy || selectedImportIds.length < 2}
        >
          <GitMerge size={14} />预览多来源冲突
        </button>
        {plan && (
          <article style={listItemStyle(true)}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>{plan.sources.length} 个数据来源</strong>
              <span style={badgeStyle(plan.fields.some((field) => field.conflict) ? 'warn' : 'neutral')}>
                {plan.fields.filter((field) => field.conflict).length} 项冲突
              </span>
            </header>
            {plan.fields.map((field) => (
              <label key={field.field} style={commerceStyles.checkRow}>
                <span>{preferenceFieldLabel(field.field)}</span>
                <select
                  value={selections[field.field] ?? ''}
                  disabled={busy || field.candidates.every((candidate) => !candidate.differs_from_current)}
                  onChange={(event) =>
                    setSelections((current) => ({ ...current, [field.field]: event.target.value }))
                  }
                >
                  <option value="">保留当前值 {JSON.stringify(field.current_value)}</option>
                  {field.candidates.filter((candidate) => candidate.differs_from_current).map((candidate) => (
                    <option key={candidate.import_id} value={candidate.import_id}>
                      {candidate.source_operator} · {JSON.stringify(candidate.imported_value)}
                    </option>
                  ))}
                </select>
              </label>
            ))}
            <footer style={{ ...commerceStyles.itemHeader, marginTop: 8 }}>
              <span />
              <button
                style={actionStyle('primary', busy || !Object.values(selections).some(Boolean))}
                type="button"
                onClick={applyMerge}
                disabled={busy || !Object.values(selections).some(Boolean)}
              >
                <Save size={13} />采用所选来源
              </button>
            </footer>
          </article>
        )}
        {adoptions.map((adoption) => (
          <article key={adoption.id} style={listItemStyle()}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>{new Date(adoption.applied_at).toLocaleString()}</strong>
              <span style={badgeStyle(adoption.status === 'applied' ? 'neutral' : 'warn')}>
                {adoption.status === 'applied' ? '已合并' : '已回滚'}
              </span>
            </header>
            <small style={commerceStyles.itemMeta}>
              修订 {adoption.before_revision ?? '无'} → {adoption.resulting_revision} ·{' '}
              {adoption.field_sources.map((source) =>
                `${preferenceFieldLabel(source.field)}=${source.source_operator}`
              ).join('、')}
            </small>
            {adoption.status === 'applied' && (
              <footer style={{ ...commerceStyles.itemHeader, marginTop: 8 }}>
                <span />
                <button style={actionStyle('danger', busy)} type="button" onClick={() => rollback(adoption)} disabled={busy}>
                  <RotateCcw size={13} />回滚
                </button>
              </footer>
            )}
          </article>
        ))}
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

function preferenceFieldLabel(field: string) {
  return {
    categories: '类别',
    tags: '标签',
    city: '城市',
    max_unit_price_micros: '价格上限',
    prefer_public: '优先公开商户',
  }[field] ?? field
}
