import { useCallback, useEffect, useState } from 'react'
import { ArrowRightLeft, Eye, RefreshCw } from 'lucide-react'
import { openCommerceApi } from './openCommerceApi'
import type {
  MerchantBusinessEvidenceDetail,
  MerchantBusinessEvidenceList,
  MerchantBusinessEvidenceSummary,
  OpenCommerceIntegration,
} from './openCommerceTypes'
import MerchantBusinessHandoffPanel from './MerchantBusinessHandoffPanel'
import MerchantBusinessHandoffQueue from './MerchantBusinessHandoffQueue'
import { errorText, formatMicros } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
  listItemStyle,
} from './openCommerceStyles'

export default function MerchantBusinessEvidencePanel({
  projectId,
  merchantId,
  integrations,
  canEdit,
}: {
  projectId: string
  merchantId: string
  integrations: OpenCommerceIntegration[]
  canEdit: boolean
}) {
  const [list, setList] = useState<MerchantBusinessEvidenceList | null>(null)
  const [detail, setDetail] = useState<MerchantBusinessEvidenceDetail | null>(null)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const [handoffEvidence, setHandoffEvidence] = useState<MerchantBusinessEvidenceSummary | null>(null)
  const [handoffRevision, setHandoffRevision] = useState(0)

  const refresh = useCallback(async () => {
    if (!projectId || !merchantId) return
    setBusy(true)
    setMessage('')
    try {
      setList(await openCommerceApi.listMerchantBusinessEvidence(projectId, merchantId))
      setDetail(null)
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }, [merchantId, projectId])

  useEffect(() => {
    void refresh()
  }, [refresh])

  async function inspect(invocationId: string) {
    setBusy(true)
    setMessage('')
    try {
      setDetail(await openCommerceApi.getMerchantBusinessEvidence(
        projectId,
        merchantId,
        invocationId,
      ))
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  const availableIntegrations = integrations.filter(
    (item) => item.merchant_id === merchantId && item.status !== 'disabled',
  )

  return (
    <>
      <section className={base.integrationSection}>
      <header>
        <span>
          <strong>AI 业务调用证据</strong>
          <small>
            {list?.erp_binding
              ? `已关联 ERP 实例 ${list.erp_binding.instance_key}，仍需适配器明确入库。`
              : '当前项目未绑定 ERP 实例；证据仍保留在开放商业调用主干。'}
          </small>
        </span>
        <button style={actionStyle('icon', busy)} type="button" onClick={refresh} disabled={busy} title="刷新业务证据">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={{ ...commerceStyles.list, padding: 12 }}>
        {(list?.evidence ?? []).map((item) => (
          <article key={item.invocation_id} style={listItemStyle()}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>{item.capability_key}</strong>
              <span style={badgeStyle(receiptTone(item.receipt_state))}>
                {receiptLabel(item.receipt_state)}
              </span>
            </header>
            <p style={commerceStyles.itemText}>
              {item.business_receipt
                ? `${item.business_receipt.entity_type} · ${item.business_receipt.reference_id} · ${item.business_receipt.state}`
                : `${item.requester_app_id} · ${item.source_authority === 'merchant_runtime_asserted' ? '商户运行时返回' : '平台处理器返回'}`}
            </p>
            <small style={commerceStyles.itemMeta}>
              {formatMicros(item.amount_micros, item.currency)} · 未扣真实资金 · {new Date(item.completed_at).toLocaleString('zh-CN')}
            </small>
            <footer style={{ ...commerceStyles.itemHeader, marginTop: 8 }}>
              <span style={commerceStyles.itemMeta}>
                {item.result_sha256 ? `结果摘要 ${item.result_sha256.slice(0, 12)}…` : item.error_code ?? '无结果'}
              </span>
              <button style={actionStyle('secondary', busy)} type="button" onClick={() => inspect(item.invocation_id)} disabled={busy}>
                <Eye size={13} />查看证据
              </button>
              {canEdit && item.result_sha256 && availableIntegrations.length > 0 && (
                <button style={actionStyle('secondary', busy)} type="button" onClick={() => setHandoffEvidence(item)} disabled={busy}>
                  <ArrowRightLeft size={13} />记录衔接
                </button>
              )}
            </footer>
          </article>
        ))}
        {(list?.evidence.length ?? 0) === 0 && (
          <p className={base.empty}>暂无终态调用。消费者 AI 或第三方 App 调用商户能力后会在这里形成证据。</p>
        )}
        {detail && <pre className={base.result}>{JSON.stringify(detail, null, 2)}</pre>}
      </div>
      {message && (
        <div style={{ ...commerceStyles.message, ...errorMessageStyle }}>{message}</div>
      )}
      </section>
      <MerchantBusinessHandoffQueue
        projectId={projectId}
        merchantId={merchantId}
        integrations={integrations}
        canEdit={canEdit}
        revision={handoffRevision}
        onSelect={(evidence) => setHandoffEvidence(evidence)}
      />
      <MerchantBusinessHandoffPanel
        projectId={projectId}
        merchantId={merchantId}
        evidence={handoffEvidence}
        integrations={integrations}
        canEdit={canEdit}
        onClose={() => setHandoffEvidence(null)}
        onRecorded={() => setHandoffRevision((value) => value + 1)}
      />
    </>
  )
}

function receiptLabel(state: MerchantBusinessEvidenceList['evidence'][number]['receipt_state']) {
  const labels = {
    valid: '标准业务回执',
    digest_only: '仅结果摘要',
    invalid_legacy: '历史回执无效',
    not_available: '调用失败',
    not_applicable: '平台结果',
  }
  return labels[state]
}

function receiptTone(
  state: MerchantBusinessEvidenceList['evidence'][number]['receipt_state'],
): 'neutral' | 'warn' | 'danger' {
  if (state === 'invalid_legacy') return 'danger'
  if (state === 'digest_only' || state === 'not_available') return 'warn'
  return 'neutral'
}
