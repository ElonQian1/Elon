import { useCallback, useEffect, useMemo, useState } from 'react'
import { Link2, RefreshCw, Search, Send, ShieldCheck, Unlink } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type {
  ConsumerPortabilityAdoptionPlan,
  ConsumerPortabilityImportSummary,
  DirectoryMerchant,
  OpenCommerceDeveloperApp,
  PortabilityRelationshipMapping,
} from './openCommerceClientTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

export default function ConsumerPortabilityReauthorization({ projectId }: { projectId: string }) {
  const [imports, setImports] = useState<ConsumerPortabilityImportSummary[]>([])
  const [mappings, setMappings] = useState<PortabilityRelationshipMapping[]>([])
  const [apps, setApps] = useState<OpenCommerceDeveloperApp[]>([])
  const [selectedImportId, setSelectedImportId] = useState('')
  const [plan, setPlan] = useState<ConsumerPortabilityAdoptionPlan | null>(null)
  const [sourceRelationshipId, setSourceRelationshipId] = useState('')
  const [targetMerchantId, setTargetMerchantId] = useState('')
  const [targetQuery, setTargetQuery] = useState('')
  const [targetMerchants, setTargetMerchants] = useState<DirectoryMerchant[]>([])
  const [requesterAppId, setRequesterAppId] = useState('')
  const [scopesText, setScopesText] = useState('')
  const [purpose, setPurpose] = useState('迁移后重新授权')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const [importResponse, mappingResponse, appResponse] = await Promise.all([
        openCommerceClientApi.listConsumerPortabilityImports(projectId),
        openCommerceClientApi.listPortabilityRelationshipMappings(projectId),
        openCommerceClientApi.listApps(projectId),
      ])
      setImports(importResponse.imports)
      setMappings(mappingResponse.mappings)
      setApps(appResponse.apps.filter((app) => app.status === 'active'))
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  const selectedCandidate = useMemo(
    () => plan?.relationship_candidates.find((item) => item.source_relationship_id === sourceRelationshipId),
    [plan, sourceRelationshipId],
  )

  async function loadRelationships(importId: string) {
    setSelectedImportId(importId)
    setSourceRelationshipId('')
    setPlan(null)
    if (!importId) return
    setBusy(true)
    try {
      setPlan(await openCommerceClientApi.getConsumerPortabilityAdoptionPlan(projectId, importId))
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  function selectRelationship(relationshipId: string) {
    setSourceRelationshipId(relationshipId)
    setTargetMerchantId('')
    setTargetQuery('')
    setTargetMerchants([])
    const candidate = plan?.relationship_candidates.find((item) => item.source_relationship_id === relationshipId)
    setScopesText(candidate?.requested_scopes.join(', ') ?? '')
    setPurpose(candidate?.purpose ?? '迁移后重新授权')
  }

  async function searchTargetMerchants() {
    setBusy(true)
    setMessage('')
    try {
      const response = await openCommerceClientApi.searchDirectoryMerchants(targetQuery)
      setTargetMerchants(response.merchants.map((item) => item.merchant))
      if (response.merchants.length === 0) setMessage('未找到已发布的目标商户。')
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  function selectTargetMerchant(merchant: DirectoryMerchant) {
    setTargetMerchantId(merchant.id)
    setTargetQuery(merchant.display_name)
    setMessage(`已选择目标商户：${merchant.display_name}。请仍由本人核对是否为同一业务主体。`)
  }

  function selectIdentityCandidate(merchantId: string) {
    setTargetMerchantId(merchantId)
    setTargetQuery(`指纹匹配候选 ${merchantId}`)
    setMessage('已选择公钥指纹一致的候选商户。它仍需要本人确认和目标商户重新审批。')
  }

  async function createMapping() {
    if (!selectedImportId || !sourceRelationshipId || !targetMerchantId.trim()) {
      setMessage('请选择来源关系并填写目标开放目录商户 ID。')
      return
    }
    if (!window.confirm('确认来源关系与目标商户是同一业务主体？该确认不会恢复旧授权。')) return
    setBusy(true)
    try {
      await openCommerceClientApi.createPortabilityRelationshipMapping(projectId, {
        import_id: selectedImportId,
        source_relationship_id: sourceRelationshipId,
        target_merchant_id: targetMerchantId.trim(),
      })
      setMessage('关系映射已保存，现在可以向目标商户重新申请授权。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function reauthorize(mapping: PortabilityRelationshipMapping) {
    const scopes = scopesText.split(',').map((value) => value.trim()).filter(Boolean)
    if (!requesterAppId || scopes.length === 0 || !purpose.trim()) {
      setMessage('请选择开发者 App，并填写申请范围和用途。')
      return
    }
    if (!window.confirm('向目标商户提交新的授权申请？旧 Grant 不会恢复。')) return
    setBusy(true)
    try {
      const result = await openCommerceClientApi.createPortabilityReauthorization(
        projectId,
        mapping.id,
        { requester_app_id: requesterAppId, scopes, purpose: purpose.trim() },
      )
      setMessage(`新的授权申请已提交：${result.authorization_request.id}`)
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function revoke(mapping: PortabilityRelationshipMapping) {
    if (!window.confirm('撤销该人工关系映射？已提交的授权申请不会被自动撤回。')) return
    setBusy(true)
    try {
      await openCommerceClientApi.revokePortabilityRelationshipMapping(projectId, mapping.id)
      setMessage('关系映射已撤销。')
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
        <span>
          <strong>关系重新授权</strong>
          <small>先由消费者确认来源与目标商户映射，再提交全新的授权申请；旧 Grant 永不复制。</small>
        </span>
        <button style={actionStyle('icon', busy)} type="button" onClick={refresh} disabled={busy} title="刷新关系映射">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={{ ...commerceStyles.list, padding: 12 }}>
        <select value={selectedImportId} onChange={(event) => loadRelationships(event.target.value)} disabled={busy}>
          <option value="">选择隔离数据包</option>
          {imports.map((item) => <option key={item.id} value={item.id}>{item.source_operator} · {item.source_package_id}</option>)}
        </select>
        <select value={sourceRelationshipId} onChange={(event) => selectRelationship(event.target.value)} disabled={busy || !plan}>
          <option value="">选择来源关系</option>
          {plan?.relationship_candidates.map((item) => (
            <option key={item.source_relationship_id} value={item.source_relationship_id}>
              {item.source_merchant_id} · {item.requested_scopes.join(',')}
            </option>
          ))}
        </select>
        {selectedCandidate?.verified_target_merchant_ids.map((merchantId) => (
          <button
            key={merchantId}
            style={actionStyle(targetMerchantId === merchantId ? 'primary' : 'secondary', busy)}
            type="button"
            onClick={() => selectIdentityCandidate(merchantId)}
            disabled={busy}
          >
            <ShieldCheck size={14} />指纹匹配候选 · {merchantId}
          </button>
        ))}
        <span style={{ display: 'flex', gap: 8 }}>
          <input
            value={targetQuery}
            onChange={(event) => setTargetQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault()
                void searchTargetMerchants()
              }
            }}
            placeholder="按名称搜索已发布商户"
            disabled={busy}
            style={{ flex: 1 }}
          />
          <button style={actionStyle('secondary', busy)} type="button" onClick={searchTargetMerchants} disabled={busy}>
            <Search size={14} />搜索
          </button>
        </span>
        {targetMerchants.map((merchant) => (
          <button
            key={merchant.id}
            style={actionStyle(targetMerchantId === merchant.id ? 'primary' : 'secondary', busy)}
            type="button"
            onClick={() => selectTargetMerchant(merchant)}
            disabled={busy}
          >
            {merchant.display_name} · {merchant.slug}
          </button>
        ))}
        <input value={targetMerchantId} readOnly placeholder="选中后写入目标商户 ID" />
        <button style={actionStyle('primary', busy || !selectedCandidate)} type="button" onClick={createMapping} disabled={busy || !selectedCandidate}>
          <Link2 size={14} />确认映射
        </button>
        <select value={requesterAppId} onChange={(event) => setRequesterAppId(event.target.value)} disabled={busy}>
          <option value="">选择独立开发者 App</option>
          {apps.map((app) => <option key={app.id} value={app.app_id}>{app.display_name}</option>)}
        </select>
        <input value={scopesText} onChange={(event) => setScopesText(event.target.value)} placeholder="授权范围，逗号分隔" disabled={busy} />
        <input value={purpose} onChange={(event) => setPurpose(event.target.value)} placeholder="授权用途" disabled={busy} />
        {mappings.map((mapping) => (
          <article key={mapping.id} style={listItemStyle()}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>{mapping.source_merchant_id} → {mapping.target_merchant_id}</strong>
              <span style={badgeStyle(mapping.identity_match_status === 'trusted_operator_key_match' ? 'neutral' : 'warn')}>
                {mapping.identity_match_status === 'trusted_operator_key_match' ? '指纹匹配' : '仅人工确认'}
              </span>
              <span style={badgeStyle(mapping.status === 'active' ? 'neutral' : 'warn')}>{mapping.status === 'active' ? '有效映射' : '已撤销'}</span>
            </header>
            {mapping.status === 'active' && (
              <footer style={{ ...commerceStyles.itemHeader, marginTop: 8 }}>
                <button style={actionStyle('danger', busy)} type="button" onClick={() => revoke(mapping)} disabled={busy}><Unlink size={13} />撤销映射</button>
                <button style={actionStyle('primary', busy)} type="button" onClick={() => reauthorize(mapping)} disabled={busy}><Send size={13} />重新申请</button>
              </footer>
            )}
          </article>
        ))}
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}
