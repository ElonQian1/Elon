import { useCallback, useEffect, useState } from 'react'
import { GitCompareArrows, RefreshCw, RotateCcw, Save } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type {
  ConsumerPortabilityAdoption,
  ConsumerPortabilityAdoptionPlan,
  ConsumerPortabilityImportSummary,
} from './openCommerceClientTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'
import ConsumerPortabilityMergePanel from './ConsumerPortabilityMergePanel'

export default function ConsumerPortabilityAdoptions({ projectId }: { projectId: string }) {
  const [imports, setImports] = useState<ConsumerPortabilityImportSummary[]>([])
  const [adoptions, setAdoptions] = useState<ConsumerPortabilityAdoption[]>([])
  const [selectedImportId, setSelectedImportId] = useState('')
  const [plan, setPlan] = useState<ConsumerPortabilityAdoptionPlan | null>(null)
  const [selectedFields, setSelectedFields] = useState<string[]>([])
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const [importResponse, adoptionResponse] = await Promise.all([
        openCommerceClientApi.listConsumerPortabilityImports(projectId),
        openCommerceClientApi.listConsumerPortabilityAdoptions(projectId),
      ])
      setImports(importResponse.imports)
      setAdoptions(adoptionResponse.adoptions)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function preview() {
    if (!selectedImportId) {
      setMessage('请选择一个隔离数据包。')
      return
    }
    setBusy(true)
    setMessage('')
    try {
      setPlan(await openCommerceClientApi.getConsumerPortabilityAdoptionPlan(projectId, selectedImportId))
      setSelectedFields([])
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function applyPreferences() {
    if (!plan?.imported_profile_available) return
    if (selectedFields.length === 0) {
      setMessage('请至少选择一个发生变化的偏好字段。')
      return
    }
    if (!window.confirm(`采用已选择的 ${selectedFields.length} 个低敏偏好字段？现有关系和授权不会恢复。`)) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.applyConsumerPortabilityPreferences(
        projectId,
        plan.import_id,
        plan.current_profile_revision,
        selectedFields,
      )
      setMessage('导入偏好已采用，并保存了可回滚快照。')
      setPlan(null)
      setSelectedFields([])
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function rollback(adoption: ConsumerPortabilityAdoption) {
    if (!window.confirm('回滚到采用前的偏好？若采用后已修改档案，服务端会拒绝覆盖。')) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.rollbackConsumerPortabilityAdoption(
        projectId,
        adoption.id,
        adoption.resulting_revision,
      )
      setMessage('偏好采用已回滚。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <>
    <section className={base.integrationSection}>
      <header>
        <span>
          <strong>数据迁移预演</strong>
          <small>只允许明确采用低敏偏好；商户关系始终需要重新授权，订单和 ERP 不自动写入。</small>
        </span>
        <button style={actionStyle('icon', busy)} type="button" onClick={refresh} disabled={busy} title="刷新迁移记录">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={{ ...commerceStyles.list, padding: 12 }}>
        <select
          value={selectedImportId}
          onChange={(event) => {
            setSelectedImportId(event.target.value)
            setPlan(null)
            setSelectedFields([])
          }}
          disabled={busy}
        >
          <option value="">选择隔离数据包</option>
          {imports.map((item) => (
            <option key={item.id} value={item.id}>{item.source_operator} · {item.source_package_id}</option>
          ))}
        </select>
        <button style={actionStyle('secondary', busy)} type="button" onClick={preview} disabled={busy || !selectedImportId}>
          <GitCompareArrows size={14} />生成预演
        </button>
        {plan && (
          <article style={listItemStyle(true)}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>偏好差异</strong>
              <span style={badgeStyle(plan.import_trust_status === 'trusted_operator_signature_verified' ? 'neutral' : 'warn')}>
                {plan.import_trust_status === 'trusted_operator_signature_verified' ? '签名可信' : '来源未认证'}
              </span>
            </header>
            {plan.preference_changes.map((change) => (
              <label key={change.field} style={commerceStyles.checkRow}>
                <input
                  type="checkbox"
                  checked={selectedFields.includes(change.field)}
                  disabled={busy || !change.changed}
                  onChange={() =>
                    setSelectedFields((current) => toggleField(current, change.field))
                  }
                />
                <span>
                  {preferenceFieldLabel(change.field)}: {JSON.stringify(change.current_value)} →{' '}
                  {JSON.stringify(change.imported_value)}
                </span>
              </label>
            ))}
            <small style={commerceStyles.itemMeta}>
              待重新授权关系 {plan.relationship_candidates.length} · 自动恢复 关闭 · 业务写入 关闭
            </small>
            <footer style={{ ...commerceStyles.itemHeader, marginTop: 8 }}>
              <span />
              <button
                style={actionStyle(
                  'primary',
                  busy || !plan.imported_profile_available || selectedFields.length === 0,
                )}
                type="button"
                onClick={applyPreferences}
                disabled={
                  busy || !plan.imported_profile_available || selectedFields.length === 0
                }
              >
                <Save size={13} />采用所选字段
              </button>
            </footer>
          </article>
        )}
        {adoptions.map((adoption) => (
          <article key={adoption.id} style={listItemStyle()}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>{new Date(adoption.applied_at).toLocaleString()}</strong>
              <span style={badgeStyle(adoption.status === 'applied' ? 'neutral' : 'warn')}>
                {adoption.status === 'applied' ? '已采用' : '已回滚'}
              </span>
            </header>
            <small style={commerceStyles.itemMeta}>
              修订 {adoption.before_revision ?? '无'} → {adoption.resulting_revision} ·{' '}
              {adoption.selected_fields.map(preferenceFieldLabel).join('、') || '整包采用'}
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
    <ConsumerPortabilityMergePanel projectId={projectId} imports={imports} onChanged={refresh} />
    </>
  )
}

function toggleField(fields: string[], field: string) {
  return fields.includes(field) ? fields.filter((value) => value !== field) : [...fields, field]
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
