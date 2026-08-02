import { useCallback, useEffect, useMemo, useState } from 'react'
import { RefreshCw, Save, Send, Sparkles, Trash2 } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type {
  ConsumerPreferenceDisclosure,
  ConsumerPreferenceField,
  ConsumerPreferences,
  ConsumerRelationship,
  DirectoryMerchant,
} from './openCommerceClientTypes'
import { errorText, splitValues } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  listItemStyle,
} from './openCommerceStyles'

const fieldOptions: Array<{ value: ConsumerPreferenceField; label: string }> = [
  { value: 'categories', label: '经营类别' },
  { value: 'tags', label: '偏好标签' },
  { value: 'city', label: '城市' },
  { value: 'max_unit_price_micros', label: '价格上限' },
]

export default function ConsumerPreferenceProfilePanel({
  projectId,
  merchants,
  onApply,
}: {
  projectId: string
  merchants: DirectoryMerchant[]
  onApply: (preferences: ConsumerPreferences) => void
}) {
  const [categories, setCategories] = useState('')
  const [tags, setTags] = useState('')
  const [city, setCity] = useState('')
  const [maxPrice, setMaxPrice] = useState('')
  const [preferPublic, setPreferPublic] = useState(true)
  const [revision, setRevision] = useState<number | null>(null)
  const [relationships, setRelationships] = useState<ConsumerRelationship[]>([])
  const [disclosures, setDisclosures] = useState<ConsumerPreferenceDisclosure[]>([])
  const [relationshipId, setRelationshipId] = useState('')
  const [sharedFields, setSharedFields] = useState<ConsumerPreferenceField[]>(['tags', 'city'])
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const shareableRelationships = useMemo(
    () => relationships.filter((relationship) =>
      relationship.status === 'active'
      && relationship.scopes.includes('preference.remember')),
    [relationships],
  )

  const merchantName = useCallback(
    (merchantId: string) => merchants.find((merchant) => merchant.id === merchantId)?.display_name ?? merchantId,
    [merchants],
  )

  const refresh = useCallback(async () => {
    try {
      const [profileResponse, relationshipResponse, disclosureResponse] = await Promise.all([
        openCommerceClientApi.getConsumerPreferenceProfile(projectId),
        openCommerceClientApi.listConsumerRelationships(projectId),
        openCommerceClientApi.listConsumerPreferenceDisclosures(projectId),
      ])
      const profile = profileResponse.profile
      if (profile) {
        setCategories(profile.preferences.categories.join(', '))
        setTags(profile.preferences.tags.join(', '))
        setCity(profile.preferences.city ?? '')
        setMaxPrice(profile.preferences.max_unit_price_micros === undefined
          ? ''
          : String(profile.preferences.max_unit_price_micros / 1_000_000))
        setPreferPublic(profile.preferences.prefer_public)
        setRevision(profile.revision)
      } else {
        clearEditor()
      }
      setRelationships(relationshipResponse.relationships)
      setDisclosures(disclosureResponse.disclosures)
      setMessage('')
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  useEffect(() => {
    setRelationshipId((current) => (
      shareableRelationships.some((relationship) => relationship.id === current)
        ? current
        : shareableRelationships[0]?.id ?? ''
    ))
  }, [shareableRelationships])

  function currentPreferences(): ConsumerPreferences {
    return {
      categories: splitValues(categories),
      tags: splitValues(tags),
      city: city.trim() || undefined,
      max_unit_price_micros: maxPrice
        ? Math.round(Number(maxPrice) * 1_000_000)
        : undefined,
      prefer_public: preferPublic,
    }
  }

  async function saveProfile(event: React.FormEvent) {
    event.preventDefault()
    setBusy(true)
    setMessage('')
    try {
      const profile = await openCommerceClientApi.upsertConsumerPreferenceProfile(
        projectId,
        currentPreferences(),
      )
      setRevision(profile.revision)
      setMessage(`偏好档案已保存为第 ${profile.revision} 版。`)
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function deleteProfile() {
    if (!window.confirm('删除偏好档案会同时移除本项目中的全部偏好披露快照。是否继续？')) return
    setBusy(true)
    try {
      const result = await openCommerceClientApi.deleteConsumerPreferenceProfile(projectId)
      clearEditor()
      setDisclosures([])
      setMessage(`偏好档案已删除，同时移除 ${result.removed_disclosures} 条披露快照。`)
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function shareProfile() {
    if (!relationshipId || sharedFields.length === 0) {
      setMessage('请选择一条有效关系和至少一个披露字段。')
      return
    }
    setBusy(true)
    try {
      await openCommerceClientApi.upsertConsumerPreferenceDisclosure(
        projectId,
        relationshipId,
        sharedFields,
      )
      setMessage('偏好披露快照已更新。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function removeDisclosure(disclosure: ConsumerPreferenceDisclosure) {
    setBusy(true)
    try {
      await openCommerceClientApi.deleteConsumerPreferenceDisclosure(
        projectId,
        disclosure.relationship_id,
      )
      setMessage('偏好披露已撤回。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  function toggleField(field: ConsumerPreferenceField) {
    setSharedFields((current) => current.includes(field)
      ? current.filter((value) => value !== field)
      : [...current, field])
  }

  function clearEditor() {
    setCategories('')
    setTags('')
    setCity('')
    setMaxPrice('')
    setPreferPublic(true)
    setRevision(null)
  }

  return (
    <section className={base.integrationSection}>
      <header>
        <div><strong>我的偏好档案</strong><small>{revision ? `第 ${revision} 版` : '尚未保存'}</small></div>
        <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新偏好档案">
          <RefreshCw size={14} />
        </button>
      </header>
      <form className={base.formCard} style={commerceStyles.sectionBody} onSubmit={saveProfile}>
        <div style={commerceStyles.grid}>
          <label>经营类别<input value={categories} onChange={(event) => setCategories(event.target.value)} placeholder="cafe, retail" /></label>
          <label>偏好标签<input value={tags} onChange={(event) => setTags(event.target.value)} placeholder="quiet, coffee" /></label>
          <label>城市<input value={city} onChange={(event) => setCity(event.target.value)} placeholder="Ji'an" /></label>
          <label>单位价格上限（CNY）<input type="number" min="0" step="0.01" value={maxPrice} onChange={(event) => setMaxPrice(event.target.value)} /></label>
        </div>
        <label><input type="checkbox" checked={preferPublic} onChange={(event) => setPreferPublic(event.target.checked)} />优先无需额外授权的公开能力</label>
        <div style={commerceStyles.headerActions}>
          <button style={actionStyle('primary', busy)} type="submit" disabled={busy}><Save size={13} />保存</button>
          <button style={actionStyle('secondary')} type="button" onClick={() => onApply(currentPreferences())}><Sparkles size={13} />用于本次发现</button>
          {revision && <button style={actionStyle('danger', busy)} type="button" onClick={deleteProfile} disabled={busy}><Trash2 size={13} />删除</button>}
        </div>
      </form>

      <div className={base.formCard} style={commerceStyles.sectionBody}>
        <header style={commerceStyles.itemHeader}><strong>按关系分享</strong><span style={badgeStyle()}>快照</span></header>
        <label>
          有效关系
          <select value={relationshipId} onChange={(event) => setRelationshipId(event.target.value)}>
            <option value="">请选择</option>
            {shareableRelationships.map((relationship) => (
              <option key={relationship.id} value={relationship.id}>{merchantName(relationship.merchant_id)} · {relationship.subject_alias}</option>
            ))}
          </select>
        </label>
        <div style={commerceStyles.headerActions}>
          {fieldOptions.map((field) => (
            <label key={field.value}><input type="checkbox" checked={sharedFields.includes(field.value)} onChange={() => toggleField(field.value)} />{field.label}</label>
          ))}
        </div>
        <button style={actionStyle('primary', busy || !relationshipId)} type="button" onClick={shareProfile} disabled={busy || !relationshipId}>
          <Send size={13} />更新披露快照
        </button>
      </div>

      <div style={{ ...commerceStyles.sectionBody, ...commerceStyles.list }}>
        {disclosures.map((disclosure) => (
          <article className={base.formCard} style={listItemStyle()} key={disclosure.relationship_id}>
            <header style={commerceStyles.itemHeader}>
              <h3 style={commerceStyles.itemTitle}>{merchantName(disclosure.merchant_id)}</h3>
              <span style={badgeStyle(disclosure.relationship_status === 'active' ? 'neutral' : 'warn')}>{relationshipLabel(disclosure.relationship_status)}</span>
            </header>
            <p style={commerceStyles.itemText}>{disclosure.shared_fields.map(fieldLabel).join(' · ')}</p>
            <code style={commerceStyles.itemMeta}>{disclosure.subject_alias} · 档案第 {disclosure.profile_revision} 版</code>
            <button style={actionStyle('danger', busy)} type="button" onClick={() => removeDisclosure(disclosure)} disabled={busy}><Trash2 size={13} />撤回披露</button>
          </article>
        ))}
        {disclosures.length === 0 && <p className={base.empty}>尚未向商户披露偏好。</p>}
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

function fieldLabel(field: ConsumerPreferenceField) {
  return fieldOptions.find((option) => option.value === field)?.label ?? field
}

function relationshipLabel(status: ConsumerPreferenceDisclosure['relationship_status']) {
  return { active: '有效', expired: '已过期', revoked: '已撤销' }[status]
}
