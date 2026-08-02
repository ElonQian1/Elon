import { useCallback, useEffect, useMemo, useState } from 'react'
import { Link2, RefreshCw, RotateCw, Unlink } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import ConsumerDataRequestManager from './ConsumerDataRequestManager'
import type {
  ConsumerRelationship,
  DirectoryMerchant,
} from './openCommerceClientTypes'
import {
  relationshipExpiresAt,
  relationshipExpiryLabel,
  relationshipExpiryOptions,
  relationshipExpiryState,
  type RelationshipExpiryPreset,
} from './openCommerceRelationshipExpiry'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  listItemStyle,
} from './openCommerceStyles'

const preferenceScope = 'preference.remember'
const membershipScope = 'membership.link'

export default function ConsumerRelationshipManager({
  projectId,
  sourceAppId,
  merchants,
}: {
  projectId: string
  sourceAppId: string
  merchants: DirectoryMerchant[]
}) {
  const [relationships, setRelationships] = useState<ConsumerRelationship[]>([])
  const [merchantId, setMerchantId] = useState('')
  const [rememberPreferences, setRememberPreferences] = useState(true)
  const [linkMembership, setLinkMembership] = useState(false)
  const [purpose, setPurpose] = useState('允许商户在有效期内关联我主动提供的偏好')
  const [expiryPreset, setExpiryPreset] = useState<RelationshipExpiryPreset>('90')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const merchantOptions = useMemo(() => {
    const unique = new Map(merchants.map((merchant) => [merchant.id, merchant]))
    return [...unique.values()]
  }, [merchants])

  const latestRelationshipIds = useMemo(() => {
    const merchantIds = new Set<string>()
    const relationshipIds = new Set<string>()
    relationships.forEach((relationship) => {
      if (merchantIds.has(relationship.merchant_id)) return
      merchantIds.add(relationship.merchant_id)
      relationshipIds.add(relationship.id)
    })
    return relationshipIds
  }, [relationships])

  const refresh = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listConsumerRelationships(projectId)
      setRelationships(response.relationships)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  useEffect(() => {
    setMerchantId((current) => {
      if (merchantOptions.some((merchant) => merchant.id === current)) return current
      return merchantOptions[0]?.id ?? ''
    })
  }, [merchantOptions])

  async function createRelationship(event: React.FormEvent) {
    event.preventDefault()
    const scopes = [
      rememberPreferences ? preferenceScope : '',
      linkMembership ? membershipScope : '',
    ].filter(Boolean)
    if (scopes.length === 0) {
      setMessage('至少选择一个关系授权范围。')
      return
    }
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.createConsumerRelationship(projectId, {
        merchant_id: merchantId,
        source_app_id: sourceAppId,
        scopes,
        purpose,
        expires_at: relationshipExpiresAt(expiryPreset),
      })
      setMessage('关系凭证已建立；商户只能看到匿名关系标识和本次授权范围。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function revokeRelationship(relationship: ConsumerRelationship) {
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.revokeConsumerRelationship(
        projectId,
        relationship.id,
      )
      setMessage('关系凭证已撤销，不会自动恢复。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function renewRelationship(relationship: ConsumerRelationship) {
    const confirmed = window.confirm(
      '续期会生成新的匿名关系标识并撤销旧凭证；旧记录仍保留用于审计，已有删除请求也会继续处理。是否继续？',
    )
    if (!confirmed) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.renewConsumerRelationship(projectId, relationship.id, {
        source_app_id: sourceAppId,
        expires_at: relationshipExpiresAt(expiryPreset),
      })
      setMessage('关系凭证已安全续期，匿名标识已轮换。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  const merchantName = (id: string) =>
    merchantOptions.find((merchant) => merchant.id === id)?.display_name ?? id

  return (
    <>
      <section className={base.integrationSection}>
      <header>
        <strong>我的商户关系</strong>
        <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新关系凭证">
          <RefreshCw size={14} />
        </button>
      </header>
      <form className={base.formCard} style={commerceStyles.sectionBody} onSubmit={createRelationship}>
        <div style={commerceStyles.grid}>
          <label>
            已发现商户
            <select value={merchantId} onChange={(event) => setMerchantId(event.target.value)} required>
              <option value="">请先查询商户</option>
              {merchantOptions.map((merchant) => (
                <option key={merchant.id} value={merchant.id}>{merchant.display_name}</option>
              ))}
            </select>
          </label>
          <label>
            有效期
            <select value={expiryPreset} onChange={(event) => setExpiryPreset(event.target.value as RelationshipExpiryPreset)}>
              {relationshipExpiryOptions.map((option) => (
                <option key={option.value} value={option.value}>{option.label}</option>
              ))}
            </select>
          </label>
          <label style={commerceStyles.wideField}>用途<input value={purpose} onChange={(event) => setPurpose(event.target.value)} required /></label>
        </div>
        <div style={commerceStyles.headerActions}>
          <label><input type="checkbox" checked={rememberPreferences} onChange={(event) => setRememberPreferences(event.target.checked)} />记住我主动提供的偏好</label>
          <label><input type="checkbox" checked={linkMembership} onChange={(event) => setLinkMembership(event.target.checked)} />关联商户会员标识</label>
        </div>
        <button style={actionStyle('primary', busy || !merchantId)} type="submit" disabled={busy || !merchantId}>
          <Link2 size={13} />建立新关系
        </button>
      </form>
      <div style={{ ...commerceStyles.sectionBody, ...commerceStyles.list }}>
        {relationships.map((relationship) => {
          const expiryState = relationshipExpiryState(relationship.expires_at)
          const isLatest = latestRelationshipIds.has(relationship.id)
          const canRenew = isLatest
            && (relationship.status !== 'active' || expiryState === 'expiring')
          return <article className={base.formCard} style={listItemStyle()} key={relationship.id}>
            <header style={commerceStyles.itemHeader}>
              <h3 style={commerceStyles.itemTitle}>{merchantName(relationship.merchant_id)}</h3>
              <span style={badgeStyle(relationship.status === 'active' && expiryState === 'healthy' ? 'neutral' : 'warn')}>
                {relationshipStatusLabel(relationship, expiryState)}
              </span>
            </header>
            <p style={commerceStyles.itemText}>{relationship.purpose}</p>
            <code style={commerceStyles.itemMeta}>{relationship.subject_alias} · {relationship.scopes.join(', ')}</code>
            <footer style={commerceStyles.itemHeader}>
              <small style={commerceStyles.itemMeta}>{relationship.source_app_id} · {relationshipExpiryLabel(relationship.expires_at)}</small>
              <div style={commerceStyles.headerActions}>
                {canRenew && (
                  <button style={actionStyle('secondary', busy)} type="button" onClick={() => renewRelationship(relationship)} disabled={busy}>
                    <RotateCw size={13} />续期
                  </button>
                )}
                {relationship.status === 'active' && (
                  <button style={actionStyle('danger', busy)} type="button" onClick={() => revokeRelationship(relationship)} disabled={busy}>
                    <Unlink size={13} />撤销
                  </button>
                )}
              </div>
            </footer>
          </article>
        })}
        {relationships.length === 0 && <p className={base.empty}>尚未建立商户关系。</p>}
      </div>
        {message && <div style={commerceStyles.message}>{message}</div>}
      </section>
      <ConsumerDataRequestManager
        projectId={projectId}
        relationships={relationships}
        onRelationshipChanged={refresh}
      />
    </>
  )
}

function relationshipStatusLabel(
  relationship: ConsumerRelationship,
  expiryState: ReturnType<typeof relationshipExpiryState>,
) {
  if (relationship.status === 'active' && expiryState === 'expiring') return '14 天内到期'
  return { active: '有效', expired: '已过期', revoked: '已撤销' }[relationship.status]
}
