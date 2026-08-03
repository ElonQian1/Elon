import { useCallback, useEffect, useMemo, useState } from 'react'
import { KeyRound, Play, RefreshCw, Search } from 'lucide-react'
import { openCommerceApi } from './openCommerceApi'
import { openCommerceClientApi } from './openCommerceClientApi'
import ConsumerRelationshipManager from './ConsumerRelationshipManager'
import ConsumerPortabilityExports from './ConsumerPortabilityExports'
import ConsumerPortabilityImports from './ConsumerPortabilityImports'
import ConsumerPortabilityTrustKeys from './ConsumerPortabilityTrustKeys'
import ConsumerPortabilityAdoptions from './ConsumerPortabilityAdoptions'
import ConsumerPortabilityReauthorization from './ConsumerPortabilityReauthorization'
import ConsumerDataVaultPanel from './ConsumerDataVaultPanel'
import ConsumerPreferenceProfilePanel from './ConsumerPreferenceProfilePanel'
import ConsumerInvocationReceipts from './ConsumerInvocationReceipts'
import CapabilityInvocationComposer from './CapabilityInvocationComposer'
import type {
  ConsumerDiscoveryMatch,
  ConsumerDiscoveryResponse,
  ConsumerPreferences,
  ConsumerRankingPolicyKey,
  OpenCommerceDeveloperApp,
} from './openCommerceClientTypes'
import { errorText, formatMicros, splitValues } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
  listItemStyle,
} from './openCommerceStyles'

export default function ConsumerCommerceSandbox({ projectId }: { projectId: string }) {
  const [apps, setApps] = useState<OpenCommerceDeveloperApp[]>([])
  const [appId, setAppId] = useState('pc-web')
  const [query, setQuery] = useState('')
  const [capabilityKey, setCapabilityKey] = useState('')
  const [city, setCity] = useState('')
  const [categories, setCategories] = useState('')
  const [tags, setTags] = useState('')
  const [maxPrice, setMaxPrice] = useState('')
  const [rankingPolicy, setRankingPolicy] = useState<ConsumerRankingPolicyKey>('transparent_preference_match.v1')
  const [result, setResult] = useState<ConsumerDiscoveryResponse | null>(null)
  const [invocation, setInvocation] = useState<Record<string, unknown> | null>(null)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const [receiptRefreshKey, setReceiptRefreshKey] = useState(0)
  const [selectedMatch, setSelectedMatch] = useState<ConsumerDiscoveryMatch | null>(null)

  const loadApps = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listApps(projectId)
      setApps(response.apps)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    loadApps()
  }, [loadApps])

  const discoveredMerchants = useMemo(
    () => result?.matches.map((match) => match.merchant) ?? [],
    [result],
  )
  const activeApps = useMemo(
    () => apps.filter((app) => app.status === 'active'),
    [apps],
  )

  useEffect(() => {
    if (appId !== 'pc-web' && !activeApps.some((app) => app.app_id === appId)) {
      setAppId('pc-web')
    }
  }, [activeApps, appId])

  const discover = useCallback(async () => {
    setBusy(true)
    setMessage('')
    setInvocation(null)
    setSelectedMatch(null)
    try {
      const response = await openCommerceClientApi.discover({
        query: query.trim() || undefined,
        capability_key: capabilityKey.trim() || undefined,
        requester_app_id: appId,
        ranking_policy: rankingPolicy,
        preferences: {
          categories: splitValues(categories),
          tags: splitValues(tags),
          city: city.trim() || undefined,
          max_unit_price_micros: maxPrice
            ? Math.round(Number(maxPrice) * 1_000_000)
            : undefined,
          prefer_public: true,
        },
        limit: 20,
      })
      setResult(response)
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }, [appId, capabilityKey, categories, city, maxPrice, query, rankingPolicy, tags])

  const applyProfile = useCallback((preferences: ConsumerPreferences) => {
    setCategories(preferences.categories.join(', '))
    setTags(preferences.tags.join(', '))
    setCity(preferences.city ?? '')
    setMaxPrice(preferences.max_unit_price_micros === undefined
      ? ''
      : String(preferences.max_unit_price_micros / 1_000_000))
    setMessage('偏好档案已带入本次发现条件。')
  }, [])

  async function requestAuthorization(match: ConsumerDiscoveryMatch) {
    if (appId === 'pc-web') {
      setMessage('申请授权前请在“开发者”页签注册独立测试 App。')
      return
    }
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.requestAuthorization({
        merchant_id: match.merchant.id,
        requester_app_id: appId,
        scopes: [match.capability.capability_key],
        purpose: `消费者沙盒调用 ${match.capability.display_name}`,
      })
      setMessage('授权申请已提交商户项目。')
      await discover()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function invoke(
    match: ConsumerDiscoveryMatch,
    input: Record<string, unknown>,
    idempotencyKey: string,
  ) {
    const request = {
      merchant_id: match.merchant.id,
      capability_key: match.capability.capability_key,
      requester_app_id: appId,
      grant_id: match.authorization.grant_id,
      idempotency_key: idempotencyKey,
      input,
    }
    setBusy(true)
    setMessage('')
    try {
      let actionConfirmationId: string | undefined
      if (match.capability.kind === 'action') {
        const prepared = appId === 'pc-web'
          ? await openCommerceApi.prepareActionConfirmation(request)
          : await openCommerceClientApi.prepareActionConfirmation(appId, request)
        const confirmed = appId === 'pc-web'
          ? await openCommerceApi.confirmActionConfirmation(prepared.id)
          : await openCommerceClientApi.confirmActionConfirmation(appId, prepared.id)
        actionConfirmationId = confirmed.id
      }
      const response = appId === 'pc-web'
        ? await openCommerceApi.invoke({ ...request, action_confirmation_id: actionConfirmationId })
        : await openCommerceClientApi.invokeAsApp(appId, {
            ...request,
            action_confirmation_id: actionConfirmationId,
          })
      setInvocation(response)
      setReceiptRefreshKey((value) => value + 1)
      setMessage('调用已完成，结果和本人调用凭证已更新。')
      return true
    } catch (error) {
      setMessage(errorText(error))
      return false
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className={base.panel}>
      <header className={base.hero} style={commerceStyles.workspaceHeader}>
        <div>
          <h2>消费者 AI 商业发现沙盒</h2>
          <p>按公开资料和个人偏好透明排序，不接受付费排名；授权能力先申请、后调用。</p>
        </div>
        <button style={actionStyle('icon')} type="button" onClick={loadApps} title="刷新应用">
          <RefreshCw size={15} />
        </button>
      </header>

      <div style={commerceStyles.grid}>
        <section className={base.integrationSection}>
          <header><strong>发现条件</strong><span style={badgeStyle()}>SANDBOX</span></header>
          <form className={base.formCard} style={commerceStyles.sectionBody} onSubmit={(event) => { event.preventDefault(); discover() }}>
            <label>
              请求应用
              <select value={appId} onChange={(event) => { setAppId(event.target.value); setSelectedMatch(null) }}>
                <option value="pc-web">公共网页身份（仅公开能力）</option>
                {activeApps.map((app) => <option key={app.id} value={app.app_id}>{app.display_name}</option>)}
              </select>
            </label>
            <label>
              排序器
              <select value={rankingPolicy} onChange={(event) => setRankingPolicy(event.target.value as ConsumerRankingPolicyKey)}>
                <option value="transparent_preference_match.v1">偏好匹配</option>
                <option value="lowest_unit_price.v1">最低调用价</option>
                <option value="public_access_first.v1">公开能力优先</option>
                <option value="recently_updated.v1">最近更新</option>
                <option value="merchant_name.v1">商户名称</option>
              </select>
            </label>
            <label>搜索词<input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="咖啡、维修、零售" /></label>
            <label>能力 Key<input value={capabilityKey} onChange={(event) => setCapabilityKey(event.target.value)} placeholder="menu.preview" /></label>
            <label>城市<input value={city} onChange={(event) => setCity(event.target.value)} placeholder="Ji'an" /></label>
            <label>经营类别<input value={categories} onChange={(event) => setCategories(event.target.value)} placeholder="cafe, retail" /></label>
            <label>偏好标签<input value={tags} onChange={(event) => setTags(event.target.value)} placeholder="quiet, coffee" /></label>
            <label>单位价格上限（CNY）<input type="number" min="0" step="0.01" value={maxPrice} onChange={(event) => setMaxPrice(event.target.value)} /></label>
            <button style={actionStyle('primary', busy)} type="submit" disabled={busy}>
              <Search size={14} />{busy ? '查询中…' : '查询商户能力'}
            </button>
          </form>
        </section>

        <section className={base.integrationSection}>
          <header>
            <strong>匹配结果</strong>
            <div style={commerceStyles.headerActions}>
              {result?.capability_contract_profile && <span style={badgeStyle()}>契约校验</span>}
              {result?.ranking_policy_label && <span style={badgeStyle()}>{result.ranking_policy_label}</span>}
              <span
                style={badgeStyle(result?.ranking_is_paid ? 'danger' : 'neutral')}
                data-tone={result?.ranking_is_paid ? 'danger' : 'neutral'}
              >
                {result?.ranking_is_paid ? '存在付费排序' : '非付费排序'}
              </span>
            </div>
          </header>
          <div className={base.formCard} style={{ ...commerceStyles.sectionBody, ...commerceStyles.scrollArea }}>
            {result?.ranking_explanation && <small style={commerceStyles.itemMeta}>{result.ranking_explanation}</small>}
            {result?.matches.map((match) => (
              <article className={base.formCard} style={listItemStyle()} key={`${match.merchant.id}:${match.capability.capability_key}`}>
                <header style={commerceStyles.itemHeader}>
                  <h3 style={commerceStyles.itemTitle}>{match.merchant.display_name} · {match.capability.display_name}</h3>
                  <span style={badgeStyle()}>{match.score} 分</span>
                </header>
                <p style={commerceStyles.itemText}>{match.reasons.join(' · ')}</p>
                <small style={commerceStyles.itemMeta}>{match.capability.capability_key} · {formatMicros(match.capability.unit_price_micros, match.capability.currency)}</small>
                <footer style={commerceStyles.itemHeader}>
                  <span style={badgeStyle(authorizationTone(match.authorization.status))} data-tone={authorizationTone(match.authorization.status)}>
                    {authorizationLabel(match.authorization.status)}
                  </span>
                  <div style={commerceStyles.headerActions}>
                    {match.authorization.status === 'request_required' && (
                      <button style={actionStyle('secondary', busy)} type="button" onClick={() => requestAuthorization(match)} disabled={busy}>
                        <KeyRound size={13} />申请授权
                      </button>
                    )}
                    {['not_required', 'granted'].includes(match.authorization.status) && (
                      <button style={actionStyle('primary', busy)} type="button" onClick={() => setSelectedMatch(match)} disabled={busy}>
                        <Play size={13} />填写并调用
                      </button>
                    )}
                  </div>
                </footer>
              </article>
            ))}
            {result && result.matches.length === 0 && <p className={base.empty}>没有符合当前条件的商户能力。</p>}
            {!result && <p className={base.empty}>尚未执行查询。</p>}
          </div>
        </section>
      </div>

      {selectedMatch && (
        <CapabilityInvocationComposer
          key={`${selectedMatch.merchant.id}:${selectedMatch.capability.capability_key}:${appId}`}
          match={selectedMatch}
          busy={busy}
          onCancel={() => setSelectedMatch(null)}
          onInvoke={invoke}
        />
      )}

      <ConsumerPreferenceProfilePanel
        projectId={projectId}
        merchants={discoveredMerchants}
        onApply={applyProfile}
      />

      <ConsumerRelationshipManager
        projectId={projectId}
        sourceAppId={appId}
        merchants={discoveredMerchants}
      />

      <ConsumerDataVaultPanel projectId={projectId} />

      <ConsumerPortabilityExports projectId={projectId} />

      <ConsumerPortabilityImports projectId={projectId} />

      <ConsumerPortabilityTrustKeys projectId={projectId} />

      <ConsumerPortabilityAdoptions projectId={projectId} />

      <ConsumerPortabilityReauthorization projectId={projectId} />

      <ConsumerInvocationReceipts refreshKey={receiptRefreshKey} />

      {invocation && <pre className={base.result}>{JSON.stringify(invocation, null, 2)}</pre>}
      {message && <div style={{ ...commerceStyles.message, ...(message.includes('失败') ? errorMessageStyle : {}) }}>{message}</div>}
    </div>
  )
}

function authorizationLabel(status: string) {
  return {
    not_required: '无需授权',
    request_required: '需要申请',
    pending: '等待商户',
    granted: '已授权',
    app_registration_required: '请先注册应用',
  }[status] ?? status
}

function authorizationTone(status: string): 'danger' | 'neutral' | 'warn' {
  if (status === 'app_registration_required') return 'danger'
  if (status === 'request_required' || status === 'pending') return 'warn'
  return 'neutral'
}
