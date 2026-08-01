import { useMemo, useState } from 'react'
import { openCommerceApi } from './openCommerceApi'
import {
  grantBudgetLabel,
  optionalPositiveInteger,
  optionalYuanMicros,
} from './openCommerceGrantBudget'
import {
  grantExpiresAt,
  grantExpiryLabel,
  grantExpiryOptions,
  isGrantExpired,
  type GrantExpiryPreset,
} from './openCommerceGrantExpiry'
import type {
  OpenCommerceCapability,
  OpenCommerceGrant,
  OpenCommerceMerchantDetail,
} from './openCommerceTypes'
import { commerceStyles } from './openCommerceStyles'
import styles from './OpenCommercePanel.module.css'

interface Props {
  projectId: string
  merchant: OpenCommerceMerchantDetail
  grants: OpenCommerceGrant[]
  canEdit: boolean
  onChanged: () => Promise<void>
}

export default function OpenCommerceMerchantEditor({
  projectId,
  merchant,
  grants,
  canEdit,
  onChanged,
}: Props) {
  const [capabilityName, setCapabilityName] = useState('')
  const [capabilityKey, setCapabilityKey] = useState('')
  const [capabilityKind, setCapabilityKind] = useState<'query' | 'action'>('query')
  const [accessLevel, setAccessLevel] = useState<'public' | 'authorized' | 'owner_only'>('public')
  const [handlerType, setHandlerType] = useState<'merchant_profile' | 'static_json' | 'merchant_runtime'>('merchant_profile')
  const [staticResponse, setStaticResponse] = useState('{"message":"hello"}')
  const [unitPriceMicros, setUnitPriceMicros] = useState('0')
  const [grantAppId, setGrantAppId] = useState('pc-web')
  const [grantScopes, setGrantScopes] = useState('')
  const [grantPurpose, setGrantPurpose] = useState('允许消费者 AI 调用指定商业能力')
  const [grantMaxInvocations, setGrantMaxInvocations] = useState('')
  const [grantMaxAmountYuan, setGrantMaxAmountYuan] = useState('')
  const [grantExpiryPreset, setGrantExpiryPreset] = useState<GrantExpiryPreset>('30')
  const [invokeCapability, setInvokeCapability] = useState('')
  const [invokeGrantId, setInvokeGrantId] = useState('')
  const [invokeInput, setInvokeInput] = useState('{}')
  const [result, setResult] = useState('')
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState('')

  const activeCapabilities = useMemo(
    () => merchant.capabilities.filter((capability) => capability.status === 'active'),
    [merchant.capabilities],
  )
  const merchantGrants = useMemo(
    () => grants.filter((grant) => grant.merchant_id === merchant.merchant.id),
    [grants, merchant.merchant.id],
  )
  const selectedCapability = activeCapabilities.find(
    (capability) => capability.capability_key === invokeCapability,
  )

  async function publishCapability(event: React.FormEvent) {
    event.preventDefault()
    setMessage('')
    setBusy('capability')
    try {
      const response = handlerType === 'static_json' ? parseObject(staticResponse, '静态响应') : undefined
      await openCommerceApi.createCapability(projectId, merchant.merchant.id, {
        capability_key: capabilityKey,
        display_name: capabilityName,
        description: '',
        kind: capabilityKind,
        access_level: accessLevel,
        input_schema: {},
        output_schema: {},
        handler_type: handlerType,
        handler_config: response ? { response } : undefined,
        unit_price_micros: Math.max(0, Number.parseInt(unitPriceMicros, 10) || 0),
        currency: 'CNY',
        freshness_seconds: 0,
      })
      setCapabilityName('')
      setCapabilityKey('')
      setMessage('商业能力已发布，网页和 MCP 会读取同一份契约。')
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  async function createGrant(event: React.FormEvent) {
    event.preventDefault()
    setMessage('')
    setBusy('grant')
    try {
      const scopes = grantScopes.split(',').map((scope) => scope.trim()).filter(Boolean)
      if (scopes.length === 0) throw new Error('至少填写一个能力键')
      const maxInvocations = optionalPositiveInteger(grantMaxInvocations, '总调用次数')
      const maxAmountMicros = optionalYuanMicros(grantMaxAmountYuan)
      await openCommerceApi.createGrant(projectId, {
        merchant_id: merchant.merchant.id,
        grantee_app_id: grantAppId,
        scopes,
        purpose: grantPurpose,
        expires_at: grantExpiresAt(grantExpiryPreset),
        max_invocations: maxInvocations,
        max_amount_micros: maxAmountMicros,
        budget_currency: 'CNY',
      })
      setMessage('授权已创建；调用方仍需同时提供自己的应用身份和 grant_id。')
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  async function invoke(event: React.FormEvent) {
    event.preventDefault()
    setMessage('')
    setResult('')
    setBusy('invoke')
    try {
      if (!invokeCapability) throw new Error('请选择要调用的能力')
      const response = await openCommerceApi.invoke({
        merchant_id: merchant.merchant.id,
        capability_key: invokeCapability,
        requester_app_id: 'pc-web',
        grant_id: invokeGrantId || undefined,
        idempotency_key: newInvocationIdempotencyKey(),
        input: parseObject(invokeInput, '调用输入'),
      })
      setResult(JSON.stringify(response, null, 2))
      setMessage('调用成功；本次只记录计量账本，不真实扣款。')
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  async function revokeGrant(grantId: string) {
    setBusy(grantId)
    setMessage('')
    try {
      await openCommerceApi.revokeGrant(projectId, grantId)
      setMessage('授权已撤销。')
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  return (
    <div className={styles.editor}>
      <div className={styles.editorHeader}>
        <div>
          <span>当前商户节点</span>
          <h2>{merchant.merchant.display_name}</h2>
          <p>{merchant.merchant.description || '尚未填写商户说明'}</p>
        </div>
        <code>{merchant.merchant.slug}</code>
      </div>

      <section className={styles.capabilityList}>
        <header><strong>已发布能力</strong><span>{activeCapabilities.length} 项</span></header>
        {activeCapabilities.length === 0 && <p className={styles.empty}>尚未发布能力。</p>}
        {activeCapabilities.map((capability) => (
          <CapabilityRow key={capability.id} capability={capability} />
        ))}
      </section>

      <div className={styles.formGrid}>
        <form className={styles.formCard} onSubmit={publishCapability}>
          <header><strong>发布商业能力</strong><small>可被网页与 MCP 同时发现</small></header>
          <label>能力名称<input value={capabilityName} onChange={(e) => setCapabilityName(e.target.value)} required disabled={!canEdit} /></label>
          <label>能力键<input value={capabilityKey} onChange={(e) => setCapabilityKey(e.target.value)} placeholder="menu.lookup" required disabled={!canEdit} /></label>
          <div className={styles.twoColumns}>
            <label>能力类型<select value={capabilityKind} onChange={(e) => setCapabilityKind(e.target.value as typeof capabilityKind)} disabled={!canEdit}>
              <option value="query">查询</option>
              <option value="action">动作</option>
            </select></label>
            <label>访问级别<select value={accessLevel} onChange={(e) => setAccessLevel(e.target.value as typeof accessLevel)} disabled={!canEdit}>
              <option value="public">公开</option>
              <option value="authorized">需要授权</option>
              <option value="owner_only">仅项目编辑者</option>
            </select></label>
          </div>
          <label>处理器<select value={handlerType} onChange={(e) => setHandlerType(e.target.value as typeof handlerType)} disabled={!canEdit}>
            <option value="merchant_profile">商户公开资料</option>
            <option value="static_json">静态 JSON</option>
            <option value="merchant_runtime">已验证商户运行时</option>
          </select></label>
          {handlerType === 'static_json' && <label>静态响应<textarea value={staticResponse} onChange={(e) => setStaticResponse(e.target.value)} disabled={!canEdit} /></label>}
          <label>单次计量（微元）<input type="number" min="0" value={unitPriceMicros} onChange={(e) => setUnitPriceMicros(e.target.value)} disabled={!canEdit} /></label>
          <button type="submit" disabled={!canEdit || busy === 'capability'}>{busy === 'capability' ? '发布中…' : '发布能力'}</button>
        </form>

        <form className={styles.formCard} onSubmit={createGrant}>
          <header><strong>创建应用授权</strong><small>授权可撤销、可审计</small></header>
          <label>应用 ID<input value={grantAppId} onChange={(e) => setGrantAppId(e.target.value)} required disabled={!canEdit} /></label>
          <label>能力键（逗号分隔）<input value={grantScopes} onChange={(e) => setGrantScopes(e.target.value)} placeholder="menu.lookup,booking.create" required disabled={!canEdit} /></label>
          <label>授权用途<textarea value={grantPurpose} onChange={(e) => setGrantPurpose(e.target.value)} required disabled={!canEdit} /></label>
          <label>授权有效期<select value={grantExpiryPreset} onChange={(e) => setGrantExpiryPreset(e.target.value as GrantExpiryPreset)} disabled={!canEdit}>
            {grantExpiryOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
          </select></label>
          <div className={styles.twoColumns}>
            <label>总调用次数<input type="number" min="1" value={grantMaxInvocations} onChange={(e) => setGrantMaxInvocations(e.target.value)} placeholder="留空不限" disabled={!canEdit} /></label>
            <label>总预算（元）<input type="number" min="0.000001" step="0.000001" value={grantMaxAmountYuan} onChange={(e) => setGrantMaxAmountYuan(e.target.value)} placeholder="留空不限" disabled={!canEdit} /></label>
          </div>
          <button type="submit" disabled={!canEdit || busy === 'grant'}>{busy === 'grant' ? '授权中…' : '创建授权'}</button>
          <div className={styles.grants}>
            {merchantGrants.map((grant) => (
              <div key={grant.id}>
                <span><strong>{grant.grantee_app_id}</strong><small>{grant.scopes.join('、')} · {grantExpiryLabel(grant.expires_at)} · {grantBudgetLabel(grant)}</small></span>
                {grant.revoked_at
                  ? <em>已撤销</em>
                  : isGrantExpired(grant.expires_at)
                    ? <em>已过期</em>
                    : <button type="button" onClick={() => revokeGrant(grant.id)} disabled={!canEdit || busy === grant.id}>撤销</button>}
              </div>
            ))}
          </div>
        </form>

        <form className={styles.formCard} onSubmit={invoke}>
          <header><strong>测试能力调用</strong><small>使用 PC 应用身份与全新幂等键</small></header>
          <label>能力<select value={invokeCapability} onChange={(e) => setInvokeCapability(e.target.value)} required>
            <option value="">请选择</option>
            {activeCapabilities.map((capability) => <option key={capability.id} value={capability.capability_key}>{capability.display_name}</option>)}
          </select></label>
          {selectedCapability?.access_level === 'authorized' && <label>授权<select value={invokeGrantId} onChange={(e) => setInvokeGrantId(e.target.value)} required>
            <option value="">请选择授权</option>
            {merchantGrants.filter((grant) => !grant.revoked_at && !isGrantExpired(grant.expires_at)).map((grant) => <option key={grant.id} value={grant.id}>{grant.grantee_app_id} · {grant.scopes.join(', ')}</option>)}
          </select></label>}
          <label>调用输入<textarea value={invokeInput} onChange={(e) => setInvokeInput(e.target.value)} /></label>
          <button type="submit" disabled={busy === 'invoke'}>{busy === 'invoke' ? '调用中…' : '调用并记录计量'}</button>
          {result && <pre className={styles.result}>{result}</pre>}
        </form>
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </div>
  )
}

function CapabilityRow({ capability }: { capability: OpenCommerceCapability }) {
  return (
    <div className={styles.capabilityRow}>
      <span><strong>{capability.display_name}</strong><code>{capability.capability_key}</code></span>
      <span><em>{capability.kind === 'action' ? '动作' : '查询'} · {accessLabel(capability.access_level)}</em><small>v{capability.version} · {capability.unit_price_micros} 微元</small></span>
    </div>
  )
}

function accessLabel(access: OpenCommerceCapability['access_level']) {
  if (access === 'authorized') return '授权'
  if (access === 'owner_only') return '项目内'
  return '公开'
}

function parseObject(value: string, label: string): Record<string, unknown> {
  const parsed = JSON.parse(value) as unknown
  if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') throw new Error(`${label}必须是 JSON object`)
  return parsed as Record<string, unknown>
}

function errorMessage(error: unknown) {
  if (error instanceof Error) return error.message
  if (error && typeof error === 'object' && 'message' in error) return String(error.message)
  return '操作失败，请稍后重试'
}

function newInvocationIdempotencyKey(): string {
  if (typeof crypto.randomUUID === 'function') return `pc-${crypto.randomUUID()}`
  return `pc-${Date.now()}-${Math.random().toString(36).slice(2, 11)}`
}
