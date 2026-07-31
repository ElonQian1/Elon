import { useCallback, useEffect, useState } from 'react'
import { KeyRound, Play, RefreshCw, Search } from 'lucide-react'
import { openCommerceApi } from './openCommerceApi'
import { openCommerceClientApi } from './openCommerceClientApi'
import type {
  ConsumerDiscoveryMatch,
  ConsumerDiscoveryResponse,
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
  const [tags, setTags] = useState('')
  const [maxPrice, setMaxPrice] = useState('')
  const [result, setResult] = useState<ConsumerDiscoveryResponse | null>(null)
  const [invocation, setInvocation] = useState<Record<string, unknown> | null>(null)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

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

  const discover = useCallback(async () => {
    setBusy(true)
    setMessage('')
    setInvocation(null)
    try {
      const response = await openCommerceClientApi.discover({
        query: query.trim() || undefined,
        capability_key: capabilityKey.trim() || undefined,
        requester_app_id: appId,
        preferences: {
          categories: [],
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
  }, [appId, capabilityKey, city, maxPrice, query, tags])

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

  async function invoke(match: ConsumerDiscoveryMatch) {
    const request = {
      merchant_id: match.merchant.id,
      capability_key: match.capability.capability_key,
      requester_app_id: appId,
      grant_id: match.authorization.grant_id,
      idempotency_key: `consumer-sandbox-${crypto.randomUUID()}`,
      input: {},
    }
    setBusy(true)
    setMessage('')
    try {
      const response = appId === 'pc-web'
        ? await openCommerceApi.invoke(request)
        : await openCommerceClientApi.invokeAsApp(appId, request)
      setInvocation(response)
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
              <select value={appId} onChange={(event) => setAppId(event.target.value)}>
                <option value="pc-web">公共网页身份（仅公开能力）</option>
                {apps.map((app) => <option key={app.id} value={app.app_id}>{app.display_name}</option>)}
              </select>
            </label>
            <label>搜索词<input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="咖啡、维修、零售" /></label>
            <label>能力 Key<input value={capabilityKey} onChange={(event) => setCapabilityKey(event.target.value)} placeholder="menu.preview" /></label>
            <label>城市<input value={city} onChange={(event) => setCity(event.target.value)} placeholder="Ji'an" /></label>
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
            <span
              style={badgeStyle(result?.ranking_is_paid ? 'danger' : 'neutral')}
              data-tone={result?.ranking_is_paid ? 'danger' : 'neutral'}
            >
              {result?.ranking_is_paid ? '存在付费排序' : '非付费排序'}
            </span>
          </header>
          <div className={base.formCard} style={{ ...commerceStyles.sectionBody, ...commerceStyles.scrollArea }}>
            {result?.matches.map((match) => (
              <article className={base.formCard} style={listItemStyle()} key={`${match.merchant.id}:${match.capability.id}`}>
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
                      <button style={actionStyle('primary', busy)} type="button" onClick={() => invoke(match)} disabled={busy}>
                        <Play size={13} />调用
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
    owner_only: '仅商户所有者',
  }[status] ?? status
}

function authorizationTone(status: string): 'danger' | 'neutral' | 'warn' {
  if (status === 'owner_only') return 'danger'
  if (status === 'request_required' || status === 'pending') return 'warn'
  return 'neutral'
}
