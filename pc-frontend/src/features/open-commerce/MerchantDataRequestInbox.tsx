import { useCallback, useEffect, useState } from 'react'
import { Check, RefreshCw, ShieldCheck, TriangleAlert, X } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { ConsumerDataErasureEvidence, ConsumerDataRequest } from './openCommerceClientTypes'
import DataErasureEvidenceList from './DataErasureEvidenceList'
import MerchantDataErasureEvidenceForm from './MerchantDataErasureEvidenceForm'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

export default function MerchantDataRequestInbox({
  projectId,
  merchantId,
  canEdit,
}: {
  projectId: string
  merchantId: string
  canEdit: boolean
}) {
  const [requests, setRequests] = useState<ConsumerDataRequest[]>([])
  const [evidence, setEvidence] = useState<ConsumerDataErasureEvidence[]>([])
  const [notes, setNotes] = useState<Record<string, string>>({})
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const [requestResponse, evidenceResponse] = await Promise.all([
        openCommerceClientApi.listMerchantDataRequests(projectId, merchantId),
        openCommerceClientApi.listMerchantDataErasureEvidence(projectId, merchantId),
      ])
      setRequests(requestResponse.requests)
      setEvidence(evidenceResponse.evidence)
      setMessage('')
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [merchantId, projectId])

  useEffect(() => { refresh() }, [refresh])

  async function decide(request: ConsumerDataRequest, action: 'accept' | 'complete' | 'reject') {
    const note = notes[request.id]?.trim() ?? ''
    if ((action === 'complete' || action === 'reject') && !note) {
      setMessage('声明完成或拒绝时必须填写处理说明。')
      return
    }
    if (action === 'complete' && !window.confirm('“完成”只会记录商户声明，不代表平台已验证外部系统删除结果。确认提交声明吗？')) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.decideConsumerDataRequest(projectId, merchantId, request.id, { action, note })
      setMessage(action === 'accept' ? '请求已接单。' : action === 'complete' ? '商户完成声明已记录。' : '拒绝说明已记录。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={base.integrationSection}>
      <header>
        <div><strong>关联数据删除请求</strong><small>完成状态是商户声明，不是平台删除证明</small></div>
        <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新删除请求"><RefreshCw size={14} /></button>
      </header>
      <div style={{ ...commerceStyles.sectionBody, ...commerceStyles.list }}>
        {requests.map((request) => (
          <article className={base.formCard} style={listItemStyle()} key={request.id}>
            <header style={commerceStyles.itemHeader}>
              <h3 style={commerceStyles.itemTitle}>{request.subject_alias}</h3>
              <div style={commerceStyles.headerActions}>
                {request.consumer_escalated_at && <span style={badgeStyle('danger')}><TriangleAlert size={12} />消费者升级关注</span>}
                {request.is_operationally_overdue && <span style={badgeStyle('warn')}>超过运营目标</span>}
                <span style={badgeStyle(dataRequestTone(request.status))}>{dataRequestStatusLabel(request.status)}</span>
              </div>
            </header>
            <small style={commerceStyles.itemMeta}>匿名关系 · {new Date(request.requested_at).toLocaleString('zh-CN')}</small>
            {request.operational_target_at && (
              <small style={commerceStyles.itemMeta}>
                内部目标 {new Date(request.operational_target_at).toLocaleString('zh-CN')}
                {' · '}消费者已催办 {request.reminder_count ?? 0}/3 次
              </small>
            )}
            {request.resolution_note && <p style={commerceStyles.itemText}>处理说明：{request.resolution_note}</p>}
            {canEdit && ['requested', 'in_progress'].includes(request.status) && (
              <label>
                处理说明
                <input value={notes[request.id] ?? ''} onChange={(event) => setNotes((current) => ({ ...current, [request.id]: event.target.value }))} maxLength={500} placeholder="完成或拒绝时必填" />
              </label>
            )}
            {request.status === 'completed' && (
              <>
                <p style={commerceStyles.itemText}><ShieldCheck size={13} /> 商户已声明完成，平台未核验外部系统。</p>
                <DataErasureEvidenceList evidence={evidence.filter((item) => item.data_request_id === request.id)} />
                {canEdit && (
                  <MerchantDataErasureEvidenceForm
                    projectId={projectId}
                    merchantId={merchantId}
                    requestId={request.id}
                    onCreated={refresh}
                  />
                )}
              </>
            )}
            {canEdit && request.status === 'requested' && (
              <div style={commerceStyles.headerActions}>
                <button style={actionStyle('primary', busy)} type="button" onClick={() => decide(request, 'accept')} disabled={busy}><Check size={13} />接单</button>
                <button style={actionStyle('danger', busy)} type="button" onClick={() => decide(request, 'reject')} disabled={busy}><X size={13} />拒绝</button>
              </div>
            )}
            {canEdit && request.status === 'in_progress' && (
              <div style={commerceStyles.headerActions}>
                <button style={actionStyle('primary', busy)} type="button" onClick={() => decide(request, 'complete')} disabled={busy}><ShieldCheck size={13} />声明完成</button>
                <button style={actionStyle('danger', busy)} type="button" onClick={() => decide(request, 'reject')} disabled={busy}><X size={13} />拒绝</button>
              </div>
            )}
          </article>
        ))}
        {requests.length === 0 && <p className={base.empty}>尚无消费者数据删除请求。</p>}
      </div>
      <small style={commerceStyles.sectionBody}>运营目标、催办和升级关注只用于内部排队，不代表法定逾期、平台裁决或外部系统删除证明。</small>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

function dataRequestStatusLabel(status: ConsumerDataRequest['status']) {
  return { requested: '待处理', in_progress: '处理中', completed: '商户声明完成', rejected: '已拒绝', withdrawn: '消费者已撤回' }[status]
}

function dataRequestTone(status: ConsumerDataRequest['status']): 'danger' | 'neutral' | 'warn' {
  if (status === 'rejected') return 'danger'
  if (status === 'requested' || status === 'in_progress') return 'warn'
  return 'neutral'
}
