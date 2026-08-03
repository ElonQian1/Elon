import { useCallback, useEffect, useMemo, useState } from 'react'
import { Download, KeyRound, Play, RefreshCw, Search, X } from 'lucide-react'
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
import ConsumerPriceFilterFields from './ConsumerPriceFilterFields'
import ConsumerPreferenceConstraintFields from './ConsumerPreferenceConstraintFields'
import ConsumerSourceFilterFields from './ConsumerSourceFilterFields'
import CapabilityInvocationComposer from './CapabilityInvocationComposer'
import ConsumerCapabilityFilterFields from './ConsumerCapabilityFilterFields'
import ConsumerCandidateScopeSummary from './ConsumerCandidateScopeSummary'
import { downloadConsumerRankingReceipt, verifyConsumerRankingReceipt } from './consumerRankingReceipt'
import type {
  ConsumerDiscoveryMatch,
  ConsumerDiscoveryResponse,
  ConsumerPreferences,
  ConsumerRankingPolicyKey,
  DirectoryCapability,
  OpenCommerceDeveloperApp,
} from './openCommerceClientTypes'
import type { OpenCommerceActionConfirmation } from './openCommerceTypes'
import { errorText, formatMicros, splitValues } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
  listItemStyle,
} from './openCommerceStyles'

interface PendingActionConfirmation {
  confirmation: OpenCommerceActionConfirmation
  appId: string
  merchantLabel: string
  capabilityLabel: string
}

export default function ConsumerCommerceSandbox({ projectId }: { projectId: string }) {
  const [apps, setApps] = useState<OpenCommerceDeveloperApp[]>([])
  const [appId, setAppId] = useState('pc-web')
  const [query, setQuery] = useState('')
  const [capabilityKey, setCapabilityKey] = useState('')
  const [capabilityKind, setCapabilityKind] = useState('')
  const [accessLevel, setAccessLevel] = useState('')
  const [city, setCity] = useState('')
  const [categories, setCategories] = useState('')
  const [tags, setTags] = useState('')
  const [requireCityMatch, setRequireCityMatch] = useState(false)
  const [requireCategoryMatch, setRequireCategoryMatch] = useState(false)
  const [requireAllTagsMatch, setRequireAllTagsMatch] = useState(false)
  const [maxPrice, setMaxPrice] = useState('')
  const [priceCurrency, setPriceCurrency] = useState('CNY')
  const [rankingPolicy, setRankingPolicy] = useState<ConsumerRankingPolicyKey>('transparent_preference_match.v1')
  const [includeRankingReceipt, setIncludeRankingReceipt] = useState(false)
  const [requireCurrentDeclaration, setRequireCurrentDeclaration] = useState(false)
  const [requireInternalSyncReceipt, setRequireInternalSyncReceipt] = useState(false)
  const [sourceProviderKey, setSourceProviderKey] = useState('')
  const [sourceDataDomain, setSourceDataDomain] = useState('')
  const [maxSourceAgeMinutes, setMaxSourceAgeMinutes] = useState('')
  const [result, setResult] = useState<ConsumerDiscoveryResponse | null>(null)
  const [invocation, setInvocation] = useState<Record<string, unknown> | null>(null)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const [receiptRefreshKey, setReceiptRefreshKey] = useState(0)
  const [selectedMatch, setSelectedMatch] = useState<ConsumerDiscoveryMatch | null>(null)
  const [pendingActionConfirmation, setPendingActionConfirmation] = useState<PendingActionConfirmation | null>(null)

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
        capability_kind: capabilityKind === 'query' || capabilityKind === 'action'
          ? capabilityKind
          : undefined,
        access_level: accessLevel === 'public' || accessLevel === 'authorized'
          ? accessLevel
          : undefined,
        require_city_match: requireCityMatch,
        require_category_match: requireCategoryMatch,
        require_all_tags_match: requireAllTagsMatch,
        requester_app_id: appId,
        ranking_policy: rankingPolicy,
        include_ranking_receipt: includeRankingReceipt,
        require_current_declaration: requireCurrentDeclaration,
        require_internal_sync_receipt: requireInternalSyncReceipt,
        source_provider_key: sourceProviderKey.trim() || undefined,
        source_data_domain: sourceDataDomain.trim() || undefined,
        max_source_age_seconds: maxSourceAgeMinutes
          ? Math.round(Number(maxSourceAgeMinutes) * 60)
          : undefined,
        price_currency: maxPrice ? priceCurrency.trim() || undefined : undefined,
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
      if (response.ranking_receipt) await verifyConsumerRankingReceipt(response.ranking_receipt)
      setResult(response)
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }, [accessLevel, appId, capabilityKey, capabilityKind, categories, city, includeRankingReceipt, maxPrice, maxSourceAgeMinutes, priceCurrency, query, rankingPolicy, requireAllTagsMatch, requireCategoryMatch, requireCityMatch, requireCurrentDeclaration, requireInternalSyncReceipt, sourceDataDomain, sourceProviderKey, tags])

  const applyProfile = useCallback((preferences: ConsumerPreferences) => {
    setCategories(preferences.categories.join(', '))
    setTags(preferences.tags.join(', '))
    setCity(preferences.city ?? '')
    setMaxPrice(preferences.max_unit_price_micros === undefined
      ? ''
      : String(preferences.max_unit_price_micros / 1_000_000))
    setPriceCurrency('CNY')
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

  async function downloadRankingReceipt() {
    if (!result?.ranking_receipt) return
    try {
      await downloadConsumerRankingReceipt(result.ranking_receipt)
      setMessage('排序凭证已复核并下载。')
    } catch (error) {
      setMessage(errorText(error))
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
        setPendingActionConfirmation({
          confirmation: prepared,
          appId,
          merchantLabel: match.merchant.display_name,
          capabilityLabel: match.capability.display_name,
        })
        const confirmed = appId === 'pc-web'
          ? await openCommerceApi.confirmActionConfirmation(prepared.id)
          : await openCommerceClientApi.confirmActionConfirmation(appId, prepared.id)
        setPendingActionConfirmation({
          confirmation: confirmed,
          appId,
          merchantLabel: match.merchant.display_name,
          capabilityLabel: match.capability.display_name,
        })
        actionConfirmationId = confirmed.id
      }
      const response = appId === 'pc-web'
        ? await openCommerceApi.invoke({ ...request, action_confirmation_id: actionConfirmationId })
        : await openCommerceClientApi.invokeAsApp(appId, {
            ...request,
            action_confirmation_id: actionConfirmationId,
          })
      setInvocation(response)
      setPendingActionConfirmation(null)
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

  async function cancelPendingActionConfirmation() {
    const pending = pendingActionConfirmation
    if (!pending) return
    setBusy(true)
    setMessage('')
    try {
      if (pending.appId === 'pc-web') {
        await openCommerceApi.cancelActionConfirmation(pending.confirmation.id)
      } else {
        await openCommerceClientApi.cancelActionConfirmation(
          pending.appId,
          pending.confirmation.id,
        )
      }
      setPendingActionConfirmation(null)
      setSelectedMatch(null)
      setMessage('本次经营操作已取消，没有创建新的调用。')
    } catch (error) {
      setMessage(errorText(error))
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
              <select
                value={appId}
                onChange={(event) => { setAppId(event.target.value); setSelectedMatch(null) }}
                disabled={busy || Boolean(pendingActionConfirmation)}
              >
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
            <label>
              <span>保存排序凭证</span>
              <input type="checkbox" checked={includeRankingReceipt} onChange={(event) => setIncludeRankingReceipt(event.target.checked)} />
            </label>
            <label>
              <span>只看声明期内数据</span>
              <input type="checkbox" checked={requireCurrentDeclaration} onChange={(event) => setRequireCurrentDeclaration(event.target.checked)} />
            </label>
            <label>
              <span>只看有内部同步回执的能力</span>
              <input type="checkbox" checked={requireInternalSyncReceipt} onChange={(event) => setRequireInternalSyncReceipt(event.target.checked)} />
            </label>
            <ConsumerSourceFilterFields
              providerKey={sourceProviderKey}
              dataDomain={sourceDataDomain}
              maxAgeMinutes={maxSourceAgeMinutes}
              options={result?.source_filter_options}
              onProviderKeyChange={setSourceProviderKey}
              onDataDomainChange={setSourceDataDomain}
              onMaxAgeMinutesChange={setMaxSourceAgeMinutes}
            />
            <label>搜索词<input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="咖啡、维修、零售" /></label>
            <label>能力 Key<input value={capabilityKey} onChange={(event) => setCapabilityKey(event.target.value)} placeholder="menu.preview" /></label>
            <ConsumerCapabilityFilterFields
              capabilityKind={capabilityKind}
              accessLevel={accessLevel}
              onCapabilityKindChange={setCapabilityKind}
              onAccessLevelChange={setAccessLevel}
            />
            <label>城市<input value={city} onChange={(event) => setCity(event.target.value)} placeholder="Ji'an" /></label>
            <label>经营类别<input value={categories} onChange={(event) => setCategories(event.target.value)} placeholder="cafe, retail" /></label>
            <label>偏好标签<input value={tags} onChange={(event) => setTags(event.target.value)} placeholder="quiet, coffee" /></label>
            <ConsumerPreferenceConstraintFields
              requireCityMatch={requireCityMatch}
              requireCategoryMatch={requireCategoryMatch}
              requireAllTagsMatch={requireAllTagsMatch}
              onRequireCityMatchChange={setRequireCityMatch}
              onRequireCategoryMatchChange={setRequireCategoryMatch}
              onRequireAllTagsMatchChange={setRequireAllTagsMatch}
            />
            <ConsumerPriceFilterFields
              maxPrice={maxPrice}
              currency={priceCurrency}
              onMaxPriceChange={setMaxPrice}
              onCurrencyChange={setPriceCurrency}
            />
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
              {result?.freshness_requirement === 'current_declaration' && <span style={badgeStyle()}>仅声明期内</span>}
              {result?.source_requirement === 'internal_sync_receipt' && <span style={badgeStyle()}>仅内部回执来源</span>}
              {result?.source_filter.provider_key && <span style={badgeStyle()}>厂商 {result.source_filter.provider_key}</span>}
              {result?.source_filter.data_domain && <span style={badgeStyle()}>数据域 {result.source_filter.data_domain}</span>}
              {result?.source_filter.max_age_seconds && <span style={badgeStyle()}>回执不超过 {Math.ceil(result.source_filter.max_age_seconds / 60)} 分钟</span>}
              {result?.price_filter.currency && result.price_filter.max_unit_price_micros !== null && (
                <span style={badgeStyle()}>价格不超过 {formatMicros(result.price_filter.max_unit_price_micros, result.price_filter.currency)}</span>
              )}
              {result?.capability_filter.kind && <span style={badgeStyle()}>{result.capability_filter.kind === 'action' ? '仅经营操作' : '仅信息查询'}</span>}
              {result?.capability_filter.access_level && <span style={badgeStyle()}>{result.capability_filter.access_level === 'authorized' ? '仅需授权调用' : '仅公开调用'}</span>}
              {result?.preference_constraints.require_city_match && <span style={badgeStyle()}>城市硬约束</span>}
              {result?.preference_constraints.require_category_match && <span style={badgeStyle()}>类别硬约束</span>}
              {result?.preference_constraints.require_all_tags_match && <span style={badgeStyle()}>全部标签硬约束</span>}
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
            {result?.candidate_scope && <ConsumerCandidateScopeSummary scope={result.candidate_scope} />}
            {result?.ranking_receipt && (
              <article className={base.formCard} style={listItemStyle()}>
                <header style={commerceStyles.itemHeader}>
                  <h3 style={commerceStyles.itemTitle}>排序凭证</h3>
                  <span style={badgeStyle()}>SHA-256 已复核</span>
                </header>
                <small style={commerceStyles.itemMeta}>摘要 {result.ranking_receipt.payload_sha256.slice(0, 24)}… · 未含运营方签名</small>
                <button style={actionStyle('secondary')} type="button" onClick={downloadRankingReceipt}><Download size={13} />下载凭证</button>
              </article>
            )}
            {result?.matches.map((match) => (
              <article className={base.formCard} style={listItemStyle()} key={`${match.merchant.id}:${match.capability.capability_key}`}>
                <header style={commerceStyles.itemHeader}>
                  <h3 style={commerceStyles.itemTitle}>{match.merchant.display_name} · {match.capability.display_name}</h3>
                  <span style={badgeStyle()}>{match.score} 分</span>
                </header>
                <p style={commerceStyles.itemText}>{match.reasons.join(' · ')}</p>
                <small style={commerceStyles.itemMeta}>{match.capability.capability_key} · {formatMicros(match.capability.unit_price_micros, match.capability.currency)}</small>
                <footer style={commerceStyles.itemHeader}>
                  <div style={commerceStyles.headerActions}>
                    <span style={badgeStyle(authorizationTone(match.authorization.status))} data-tone={authorizationTone(match.authorization.status)}>
                      {authorizationLabel(match.authorization.status)}
                    </span>
                    <span style={badgeStyle(freshnessTone(match.capability.freshness.status))} data-tone={freshnessTone(match.capability.freshness.status)}>
                      {freshnessLabel(match.capability.freshness.status)}
                    </span>
                    <span style={badgeStyle('neutral')} title={sourceDetail(match.capability.source)}>
                      {sourceLabel(match.capability.source.kind)} · 商户声明
                    </span>
                  </div>
                  <div style={commerceStyles.headerActions}>
                    {['request_required', 'grant_refresh_required'].includes(match.authorization.status) && (
                      <button style={actionStyle('secondary', busy)} type="button" onClick={() => requestAuthorization(match)} disabled={busy}>
                        <KeyRound size={13} />{match.authorization.status === 'grant_refresh_required' ? '申请新额度' : '申请授权'}
                      </button>
                    )}
                    {['not_required', 'granted'].includes(match.authorization.status) && (
                      <button
                        style={actionStyle(
                          'primary',
                          busy || (match.capability.kind === 'action' && Boolean(pendingActionConfirmation)),
                        )}
                        type="button"
                        onClick={() => setSelectedMatch(match)}
                        disabled={busy || (match.capability.kind === 'action' && Boolean(pendingActionConfirmation))}
                      >
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

      {pendingActionConfirmation && (
        <section className={base.integrationSection}>
          <header>
            <strong>待处理经营操作</strong>
            <span style={badgeStyle('danger')} data-tone="danger">等待处理</span>
          </header>
          <div style={commerceStyles.sectionBody}>
            <p style={commerceStyles.itemText}>
              {pendingActionConfirmation.merchantLabel} · {pendingActionConfirmation.capabilityLabel}
            </p>
            <small style={commerceStyles.itemMeta}>
              调用未完成。可以保留相同幂等键重试，或明确取消本次动作确认。
            </small>
            <button
              style={actionStyle('secondary', busy)}
              type="button"
              onClick={cancelPendingActionConfirmation}
              disabled={busy}
              title="取消本次经营操作"
            >
              <X size={14} />取消本次动作
            </button>
          </div>
        </section>
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

function freshnessLabel(status: 'current' | 'stale' | 'unknown') {
  if (status === 'current') return '声明期内'
  if (status === 'stale') return '声明已过期'
  return '未声明新鲜度'
}

function freshnessTone(status: 'current' | 'stale' | 'unknown') {
  if (status === 'current') return 'neutral' as const
  if (status === 'stale') return 'warn' as const
  return 'neutral' as const
}

function sourceLabel(kind: DirectoryCapability['source']['kind']) {
  if (kind === 'merchant_profile') return '商户公开资料'
  if (kind === 'merchant_static_data') return '商户静态数据'
  if (kind === 'merchant_runtime') return '商户运行时'
  if (kind === 'integration_sync_receipt') return '内部同步回执'
  return '商户声明数据'
}

function sourceDetail(source: DirectoryCapability['source']) {
  if (source.kind !== 'integration_sync_receipt') return '由商户项目声明，未经外部平台验证'
  return [
    source.provider_key,
    source.data_domain,
    source.receipt_status,
    source.receipt_completed_at
      ? new Date(source.receipt_completed_at).toLocaleString('zh-CN')
      : null,
    '内部回执，未经外部平台验证',
  ].filter(Boolean).join(' · ')
}

function authorizationLabel(status: string) {
  return {
    not_required: '无需授权',
    request_required: '需要申请',
    pending: '等待商户',
    granted: '已授权',
    grant_refresh_required: '授权额度不足',
    app_registration_required: '请先注册应用',
  }[status] ?? status
}

function authorizationTone(status: string): 'danger' | 'neutral' | 'warn' {
  if (status === 'app_registration_required') return 'danger'
  if (status === 'request_required' || status === 'pending' || status === 'grant_refresh_required') return 'warn'
  return 'neutral'
}
