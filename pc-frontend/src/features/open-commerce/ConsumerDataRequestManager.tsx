import { useCallback, useEffect, useMemo, useState } from 'react'
import { BellRing, RefreshCw, Trash2, TriangleAlert, Undo2 } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { ConsumerDataErasureEvidence, ConsumerDataRequest, ConsumerRelationship } from './openCommerceClientTypes'
import DataErasureEvidenceList from './DataErasureEvidenceList'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

export default function ConsumerDataRequestManager({
  projectId,
  relationships,
  onRelationshipChanged,
}: {
  projectId: string
  relationships: ConsumerRelationship[]
  onRelationshipChanged: () => Promise<void>
}) {
  const [requests, setRequests] = useState<ConsumerDataRequest[]>([])
  const [evidence, setEvidence] = useState<ConsumerDataErasureEvidence[]>([])
  const [relationshipId, setRelationshipId] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const [requestResponse, evidenceResponse] = await Promise.all([
        openCommerceClientApi.listConsumerDataRequests(projectId),
        openCommerceClientApi.listConsumerDataErasureEvidence(projectId),
      ])
      setRequests(requestResponse.requests)
      setEvidence(evidenceResponse.evidence)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => { refresh() }, [refresh])
  useEffect(() => {
    setRelationshipId((current) => {
      if (relationships.some((relationship) => relationship.id === current)) return current
      return relationships[0]?.id ?? ''
    })
  }, [relationships])

  const relationshipById = useMemo(
    () => new Map(relationships.map((relationship) => [relationship.id, relationship])),
    [relationships],
  )
  const openRelationshipIds = useMemo(
    () => new Set(requests
      .filter((request) => ['requested', 'in_progress'].includes(request.status))
      .map((request) => request.relationship_id)),
    [requests],
  )

  async function createRequest(event: React.FormEvent) {
    event.preventDefault()
    if (!relationshipId || !window.confirm('发起删除请求会立即撤销这段关系。平台会记录请求，但不能自动证明商户外部系统已删除数据。确定继续吗？')) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.createConsumerDataErasureRequest(projectId, relationshipId)
      setMessage('删除请求已提交，原关系已撤销；等待商户处理。')
      await Promise.all([refresh(), onRelationshipChanged()])
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function withdraw(request: ConsumerDataRequest) {
    if (!window.confirm('撤回请求不会恢复此前已撤销的关系。确定撤回吗？')) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.withdrawConsumerDataRequest(projectId, request.id)
      setMessage('删除请求已撤回；如需恢复关系，请重新建立新的关系凭证。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function followUp(request: ConsumerDataRequest, action: 'reminder' | 'escalate_attention') {
    if (action === 'escalate_attention' && !window.confirm('升级关注只会提高商户收件箱优先级，不代表平台仲裁、处罚或认定违法。确定继续吗？')) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.followUpConsumerDataRequest(
        projectId,
        request.id,
        action,
        `consumer-data-request-${action}-${crypto.randomUUID()}`,
      )
      setMessage(action === 'reminder' ? '催办记录已发送给商户。' : '该请求已标记为升级关注。')
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
        <div><strong>关联数据删除请求</strong><small>请求回执不等于平台验证外部数据已删除</small></div>
        <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新删除请求"><RefreshCw size={14} /></button>
      </header>
      <form className={base.formCard} style={commerceStyles.sectionBody} onSubmit={createRequest}>
        <label>
          关系凭证
          <select value={relationshipId} onChange={(event) => setRelationshipId(event.target.value)} required>
            <option value="">尚无关系凭证</option>
            {relationships.map((relationship) => (
              <option key={relationship.id} value={relationship.id} disabled={openRelationshipIds.has(relationship.id)}>
                {relationship.subject_alias} · {relationship.status === 'active' ? '有效' : '历史'}
              </option>
            ))}
          </select>
        </label>
        <button style={actionStyle('danger', busy || !relationshipId || openRelationshipIds.has(relationshipId))} type="submit" disabled={busy || !relationshipId || openRelationshipIds.has(relationshipId)}>
          <Trash2 size={13} />请求删除关联数据
        </button>
      </form>
      <div style={{ ...commerceStyles.sectionBody, ...commerceStyles.list }}>
        {requests.map((request) => (
          <article className={base.formCard} style={listItemStyle()} key={request.id}>
            <header style={commerceStyles.itemHeader}>
              <h3 style={commerceStyles.itemTitle}>{request.subject_alias}</h3>
              <div style={commerceStyles.headerActions}>
                {request.consumer_escalated_at && <span style={badgeStyle('danger')}>已升级关注</span>}
                {request.is_operationally_overdue && <span style={badgeStyle('warn')}>超过运营目标</span>}
                <span style={badgeStyle(dataRequestTone(request.status))}>{dataRequestStatusLabel(request.status)}</span>
              </div>
            </header>
            <small style={commerceStyles.itemMeta}>关系 {relationshipById.get(request.relationship_id)?.status ?? '历史'} · {new Date(request.requested_at).toLocaleString('zh-CN')}</small>
            {request.operational_target_at && (
              <small style={commerceStyles.itemMeta}>
                内部目标 {new Date(request.operational_target_at).toLocaleString('zh-CN')}
                {' · '}已催办 {request.reminder_count ?? 0}/3 次
              </small>
            )}
            {request.next_reminder_at && !request.can_send_reminder && ['requested', 'in_progress'].includes(request.status) && (
              <small style={commerceStyles.itemMeta}>下次可催办 {new Date(request.next_reminder_at).toLocaleString('zh-CN')}</small>
            )}
            {request.resolution_note && <p style={commerceStyles.itemText}>商户说明：{request.resolution_note}</p>}
            {request.status === 'completed' && (
              <>
                <p style={commerceStyles.itemText}>该状态为商户声明完成，平台未验证外部系统删除结果。</p>
                <DataErasureEvidenceList evidence={evidence.filter((item) => item.data_request_id === request.id)} />
              </>
            )}
            {['requested', 'in_progress'].includes(request.status) && (
              <div style={commerceStyles.headerActions}>
                {request.status === 'requested' && (
                  <button style={actionStyle('secondary', busy)} type="button" onClick={() => withdraw(request)} disabled={busy}><Undo2 size={13} />撤回请求</button>
                )}
                {request.can_send_reminder && (
                  <button style={actionStyle('secondary', busy)} type="button" onClick={() => followUp(request, 'reminder')} disabled={busy}><BellRing size={13} />催办</button>
                )}
                {request.can_escalate_attention && (
                  <button style={actionStyle('danger', busy)} type="button" onClick={() => followUp(request, 'escalate_attention')} disabled={busy}><TriangleAlert size={13} />升级关注</button>
                )}
              </div>
            )}
          </article>
        ))}
        {requests.length === 0 && <p className={base.empty}>尚未发起关联数据删除请求。</p>}
      </div>
      <small style={commerceStyles.sectionBody}>7 天为产品内部运营目标，不是对任何地区法定期限的判断；升级关注也不会自动启动仲裁或处罚。</small>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

function dataRequestStatusLabel(status: ConsumerDataRequest['status']) {
  return { requested: '待商户处理', in_progress: '商户处理中', completed: '商户声明完成', rejected: '商户拒绝', withdrawn: '已撤回' }[status]
}

function dataRequestTone(status: ConsumerDataRequest['status']): 'danger' | 'neutral' | 'warn' {
  if (status === 'rejected') return 'danger'
  if (status === 'requested' || status === 'in_progress') return 'warn'
  return 'neutral'
}
