import { useCallback, useEffect, useState } from 'react'
import { RefreshCw } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { ConsumerRelationship } from './openCommerceClientTypes'
import { relationshipExpiryLabel } from './openCommerceRelationshipExpiry'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  listItemStyle,
} from './openCommerceStyles'

export default function MerchantRelationshipInbox({
  projectId,
  merchantId,
}: {
  projectId: string
  merchantId: string
}) {
  const [relationships, setRelationships] = useState<ConsumerRelationship[]>([])
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listMerchantRelationships(
        projectId,
        merchantId,
      )
      setRelationships(response.relationships)
      setMessage('')
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [merchantId, projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  return (
    <section className={base.integrationSection}>
      <header>
        <div><strong>消费者关系凭证</strong><small>只显示匿名关系标识，不披露消费者账号或项目</small></div>
        <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新关系凭证">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={{ ...commerceStyles.sectionBody, ...commerceStyles.list }}>
        {relationships.map((relationship) => (
          <article className={base.formCard} style={listItemStyle()} key={relationship.id}>
            <header style={commerceStyles.itemHeader}>
              <h3 style={commerceStyles.itemTitle}>{relationship.subject_alias}</h3>
              <span style={badgeStyle(relationship.status === 'active' ? 'neutral' : 'warn')}>{relationshipStatusLabel(relationship.status)}</span>
            </header>
            <p style={commerceStyles.itemText}>{relationship.purpose}</p>
            <code style={commerceStyles.itemMeta}>{relationship.scopes.join(', ')}</code>
            <small style={commerceStyles.itemMeta}>{relationship.source_app_id} · {relationshipExpiryLabel(relationship.expires_at)}</small>
          </article>
        ))}
        {relationships.length === 0 && <p className={base.empty}>尚无消费者主动建立关系。</p>}
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

function relationshipStatusLabel(status: ConsumerRelationship['status']) {
  return { active: '有效', expired: '已过期', revoked: '已撤销' }[status]
}
