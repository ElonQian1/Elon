import { useCallback, useEffect, useMemo, useState } from 'react'
import { RefreshCw, Trash2, Undo2 } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { ConsumerDataRequest, ConsumerRelationship } from './openCommerceClientTypes'
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
  const [relationshipId, setRelationshipId] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listConsumerDataRequests(projectId)
      setRequests(response.requests)
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
              <span style={badgeStyle(dataRequestTone(request.status))}>{dataRequestStatusLabel(request.status)}</span>
            </header>
            <small style={commerceStyles.itemMeta}>关系 {relationshipById.get(request.relationship_id)?.status ?? '历史'} · {new Date(request.requested_at).toLocaleString('zh-CN')}</small>
            {request.resolution_note && <p style={commerceStyles.itemText}>商户说明：{request.resolution_note}</p>}
            {request.status === 'completed' && <p style={commerceStyles.itemText}>该状态为商户声明完成，平台未验证外部系统删除结果。</p>}
            {request.status === 'requested' && (
              <button style={actionStyle('secondary', busy)} type="button" onClick={() => withdraw(request)} disabled={busy}><Undo2 size={13} />撤回请求</button>
            )}
          </article>
        ))}
        {requests.length === 0 && <p className={base.empty}>尚未发起关联数据删除请求。</p>}
      </div>
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
