import { EyeOff, Globe2 } from 'lucide-react'
import { useState } from 'react'
import { openCommerceApi } from './openCommerceApi'
import type { OpenCommerceDirectoryPublication, OpenCommerceMerchantDetail } from './openCommerceTypes'
import { actionStyle, badgeStyle, commerceStyles } from './openCommerceStyles'
import styles from './OpenCommercePanel.module.css'

interface Props {
  projectId: string
  merchant: OpenCommerceMerchantDetail
  publication?: OpenCommerceDirectoryPublication
  canEdit: boolean
  onChanged: () => Promise<void>
}

export default function OpenCommerceDirectoryPublisher({
  projectId,
  merchant,
  publication,
  canEdit,
  onChanged,
}: Props) {
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const published = publication?.status === 'published'
  const discoverableCapabilities = merchant.capabilities.filter(
    (capability) => capability.status === 'active' && capability.access_level !== 'owner_only',
  )

  async function setPublished(next: boolean) {
    setBusy(true)
    setMessage('')
    try {
      await openCommerceApi.setDirectoryPublication(projectId, merchant.merchant.id, next)
      setMessage(next ? '商户已进入开放目录。' : '商户已从开放目录撤回。')
      await onChanged()
    } catch (error) {
      setMessage(error instanceof Error ? error.message : '目录状态更新失败')
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={styles.capabilityList}>
      <header>
        <strong>开放目录</strong>
        <span style={badgeStyle(published ? 'neutral' : 'warn')}>{published ? '已发布' : '未发布'}</span>
      </header>
      <p>
        发布后，其他项目和消费者 AI 只能看到商户公开资料与 {discoverableCapabilities.length} 项可发现能力契约；
        项目 ID、所有者、运行地址、密钥和处理器配置不会公开。
      </p>
      <div style={commerceStyles.headerActions}>
        {published ? (
          <button style={actionStyle('secondary', busy)} type="button" disabled={!canEdit || busy} onClick={() => setPublished(false)}>
            <EyeOff size={14} />撤回目录
          </button>
        ) : (
          <button style={actionStyle('primary', busy)} type="button" disabled={!canEdit || busy || discoverableCapabilities.length === 0} onClick={() => setPublished(true)}>
            <Globe2 size={14} />发布到目录
          </button>
        )}
        {publication && <small>修订 {publication.revision}</small>}
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}
