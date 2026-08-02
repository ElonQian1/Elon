import { useCallback, useEffect, useState } from 'react'
import { RefreshCw } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { ConsumerPreferenceDisclosure } from './openCommerceClientTypes'
import { errorText, formatMicros } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

export default function MerchantPreferenceInbox({
  projectId,
  merchantId,
}: {
  projectId: string
  merchantId: string
}) {
  const [disclosures, setDisclosures] = useState<ConsumerPreferenceDisclosure[]>([])
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listMerchantPreferenceDisclosures(
        projectId,
        merchantId,
      )
      setDisclosures(response.disclosures)
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
        <div><strong>消费者偏好收件箱</strong><small>仅显示仍有效关系的主动披露快照</small></div>
        <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新偏好披露">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={{ ...commerceStyles.sectionBody, ...commerceStyles.list }}>
        {disclosures.map((disclosure) => (
          <article className={base.formCard} style={listItemStyle()} key={disclosure.relationship_id}>
            <header style={commerceStyles.itemHeader}>
              <h3 style={commerceStyles.itemTitle}>{disclosure.subject_alias}</h3>
              <span style={badgeStyle()}>档案第 {disclosure.profile_revision} 版</span>
            </header>
            <p style={commerceStyles.itemText}>{preferenceSummary(disclosure)}</p>
            <code style={commerceStyles.itemMeta}>{disclosure.shared_fields.join(', ')}</code>
          </article>
        ))}
        {disclosures.length === 0 && <p className={base.empty}>尚无可访问的消费者偏好披露。</p>}
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

function preferenceSummary(disclosure: ConsumerPreferenceDisclosure) {
  const { preferences } = disclosure
  return [
    preferences.categories?.length ? `类别：${preferences.categories.join('、')}` : '',
    preferences.tags?.length ? `标签：${preferences.tags.join('、')}` : '',
    preferences.city ? `城市：${preferences.city}` : '',
    preferences.max_unit_price_micros === undefined
      ? ''
      : `价格上限：${formatMicros(preferences.max_unit_price_micros, 'CNY')}`,
  ].filter(Boolean).join(' · ')
}
