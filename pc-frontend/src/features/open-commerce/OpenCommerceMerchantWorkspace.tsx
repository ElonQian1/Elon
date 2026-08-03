import { useCallback, useEffect, useMemo, useState } from 'react'
import { openCommerceApi } from './openCommerceApi'
import OpenCommerceMerchantEditor from './OpenCommerceMerchantEditor'
import OpenCommerceIntegrationManager from './OpenCommerceIntegrationManager'
import OpenCommerceRuntimeManager from './OpenCommerceRuntimeManager'
import OpenCommerceDirectoryPublisher from './OpenCommerceDirectoryPublisher'
import MerchantPortableIdentityPanel from './MerchantPortableIdentityPanel'
import OpenCommerceRateLimitManager from './OpenCommerceRateLimitManager'
import OpenCommerceAppBlockManager from './OpenCommerceAppBlockManager'
import MerchantRelationshipInbox from './MerchantRelationshipInbox'
import MerchantPreferenceInbox from './MerchantPreferenceInbox'
import MerchantDataRequestInbox from './MerchantDataRequestInbox'
import MerchantBusinessEvidencePanel from './MerchantBusinessEvidencePanel'
import type { OpenCommerceOverview } from './openCommerceTypes'
import { commerceStyles } from './openCommerceStyles'
import styles from './OpenCommercePanel.module.css'

const localStyles = {
  loading: { padding: 34, color: 'var(--text-muted)', textAlign: 'center' },
  eyebrow: { color: '#a9ded2', fontSize: 10, fontWeight: 900 },
  firstStep: { padding: 28, textAlign: 'center' },
  auditDot: { width: 6, height: 6, borderRadius: '50%', background: '#6dc99a' },
} as const

export default function OpenCommerceMerchantWorkspace({
  projectId,
  canEdit,
}: {
  projectId: string
  canEdit: boolean
}) {
  const [overview, setOverview] = useState<OpenCommerceOverview | null>(null)
  const [selectedMerchantId, setSelectedMerchantId] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [slug, setSlug] = useState('')
  const [description, setDescription] = useState('')
  const [publicProfile, setPublicProfile] = useState('{"category":"local_service"}')
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    if (!projectId) return
    setLoading(true)
    try {
      const next = await openCommerceApi.overview(projectId)
      setOverview(next)
      setSelectedMerchantId((current) => {
        if (next.merchants.some(({ merchant }) => merchant.id === current)) return current
        return next.merchants[0]?.merchant.id ?? ''
      })
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setLoading(false)
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  const selectedMerchant = useMemo(
    () => overview?.merchants.find(({ merchant }) => merchant.id === selectedMerchantId),
    [overview, selectedMerchantId],
  )

  async function createMerchant(event: React.FormEvent) {
    event.preventDefault()
    setBusy(true)
    setMessage('')
    try {
      const profile = parseObject(publicProfile)
      const merchant = await openCommerceApi.createMerchant(projectId, {
        display_name: displayName,
        slug: slug.trim() || undefined,
        description,
        node_mode: 'platform_hosted',
        public_profile: profile,
      })
      setDisplayName('')
      setSlug('')
      setDescription('')
      setMessage('商户节点已创建。下一步发布至少一项可调用能力。')
      await refresh()
      setSelectedMerchantId(merchant.id)
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy(false)
    }
  }

  if (loading && !overview) {
    return <div style={localStyles.loading}>正在加载开放商业网络…</div>
  }

  const totals = overview?.totals
  return (
    <div className={styles.panel}>
      <section className={styles.hero}>
        <div>
          <span style={localStyles.eyebrow}>AI-NATIVE OPEN COMMERCE · V1</span>
          <h2>让商户能力被任何获得授权的 AI 发现和调用</h2>
          <p>第一版先打通商户节点、能力契约、应用授权、幂等调用、计量与审计。数据不出售，V1 计量不真实扣款。</p>
        </div>
        <button type="button" onClick={refresh} disabled={loading}>{loading ? '刷新中…' : '刷新总览'}</button>
      </section>

      <section className={styles.stats}>
        <Stat label="商户节点" value={totals?.active_merchants ?? 0} detail={`共 ${totals?.merchants ?? 0} 个`} />
        <Stat label="目录发布" value={totals?.published_merchants ?? 0} detail="商户主动选择" />
        <Stat label="有效能力" value={totals?.active_capabilities ?? 0} detail={`共 ${totals?.capabilities ?? 0} 项`} />
        <Stat label="有效授权" value={totals?.active_grants ?? 0} detail="可撤销、可审计" />
        <Stat label="调用次数" value={totals?.invocations ?? 0} detail={`${formatMicros(totals?.metered_amount_micros ?? 0)} CNY 已计量`} />
        <Stat label="调用配额" value={totals?.active_rate_limit_policies ?? 0} detail={`近期 ${totals?.recent_rate_limited_invocations ?? 0} 次超限被拒绝`} />
        <Stat label="数据接入" value={totals?.connected_integrations ?? 0} detail={`共 ${totals?.integrations ?? 0} 个，${totals?.degraded_integrations ?? 0} 个异常`} />
        <Stat label="商户运行时" value={totals?.active_runtime_bindings ?? 0} detail="签名验证后方可调用" />
      </section>

      <div className={styles.workspace}>
        <aside className={styles.rail}>
          <header><strong>商户节点</strong><span>{overview?.merchants.length ?? 0}</span></header>
          <div className={styles.merchantList}>
            {overview?.merchants.map(({ merchant, capabilities }) => (
              <button
                type="button"
                key={merchant.id}
                data-active={merchant.id === selectedMerchantId}
                onClick={() => setSelectedMerchantId(merchant.id)}
              >
                <strong>{merchant.display_name}</strong>
                <span>{merchant.slug} · {capabilities.filter((item) => item.status === 'active').length} 项能力</span>
              </button>
            ))}
            {overview?.merchants.length === 0 && <p className={styles.empty}>还没有商户节点。</p>}
          </div>

          <form className={styles.createMerchant} onSubmit={createMerchant}>
            <header><strong>新建商户节点</strong><small>默认平台托管，可完整导出</small></header>
            <label>名称<input value={displayName} onChange={(event) => setDisplayName(event.target.value)} required disabled={!canEdit} /></label>
            <label>Slug（可选）<input value={slug} onChange={(event) => setSlug(event.target.value)} placeholder="my-store" disabled={!canEdit} /></label>
            <label>说明<textarea value={description} onChange={(event) => setDescription(event.target.value)} disabled={!canEdit} /></label>
            <label>公开资料 JSON<textarea value={publicProfile} onChange={(event) => setPublicProfile(event.target.value)} disabled={!canEdit} /></label>
            <button type="submit" disabled={!canEdit || busy}>{busy ? '创建中…' : '创建节点'}</button>
            {!canEdit && <small>当前角色只能查看和调用，不能管理节点。</small>}
          </form>
        </aside>

        <main className={styles.main}>
          {selectedMerchant
            ? <>
              <OpenCommerceDirectoryPublisher
                projectId={projectId}
                merchant={selectedMerchant}
                publication={overview?.directory_publications.find((item) => item.merchant_id === selectedMerchant.merchant.id)}
                canEdit={canEdit}
                onChanged={refresh}
              />
              <MerchantPortableIdentityPanel
                projectId={projectId}
                merchantId={selectedMerchant.merchant.id}
                canEdit={canEdit}
              />
              <OpenCommerceMerchantEditor projectId={projectId} merchant={selectedMerchant} grants={overview?.grants ?? []} canEdit={canEdit} onChanged={refresh} />
              <OpenCommerceRateLimitManager
                projectId={projectId}
                merchant={selectedMerchant}
                policies={overview?.rate_limit_policies ?? []}
                usage={overview?.rate_limit_usage ?? []}
                canEdit={canEdit}
                onChanged={refresh}
              />
              <OpenCommerceAppBlockManager
                projectId={projectId}
                merchantId={selectedMerchant.merchant.id}
                appActivityHealth={(overview?.app_activity_health ?? [])
                  .filter((item) => item.merchant_id === selectedMerchant.merchant.id)}
                suggestedAppIds={(overview?.recent_invocations ?? [])
                  .filter((invocation) => invocation.merchant_id === selectedMerchant.merchant.id)
                  .map((invocation) => invocation.requester_app_id)}
                canEdit={canEdit}
                onChanged={refresh}
              />
              <MerchantRelationshipInbox
                projectId={projectId}
                merchantId={selectedMerchant.merchant.id}
              />
              <MerchantPreferenceInbox
                projectId={projectId}
                merchantId={selectedMerchant.merchant.id}
              />
              <MerchantDataRequestInbox
                projectId={projectId}
                merchantId={selectedMerchant.merchant.id}
                canEdit={canEdit}
              />
              <MerchantBusinessEvidencePanel
                projectId={projectId}
                merchantId={selectedMerchant.merchant.id}
                integrations={overview?.integrations ?? []}
                canEdit={canEdit}
              />
              <OpenCommerceRuntimeManager
                projectId={projectId}
                merchantId={selectedMerchant.merchant.id}
                binding={overview?.runtime_bindings.find((item) => item.merchant_id === selectedMerchant.merchant.id)}
                canEdit={canEdit}
                onChanged={refresh}
              />
              <OpenCommerceIntegrationManager
                projectId={projectId}
                merchantId={selectedMerchant.merchant.id}
                integrations={overview?.integrations ?? []}
                receipts={overview?.recent_sync_receipts ?? []}
                canEdit={canEdit}
                onChanged={refresh}
              />
            </>
            : <section style={localStyles.firstStep}><strong>先创建一个商户节点</strong><p>它是商户自有数据与商业能力的边界，不是平台复制的一份商户页面。</p></section>}

          <section className={styles.audit}>
            <header><strong>最近审计</strong><span>原始调用值不会写入审计</span></header>
            {(overview?.recent_audit_events ?? []).slice(0, 12).map((event) => (
              <div key={event.id}>
                <span style={localStyles.auditDot} />
                <span><strong>{auditLabel(event.action)}</strong><small>{event.actor_app_id ?? 'system'} · {new Date(event.created_at).toLocaleString('zh-CN')}</small></span>
                <code>{event.subject_id}</code>
              </div>
            ))}
            {overview?.recent_audit_events.length === 0 && <p className={styles.empty}>暂无审计事件。</p>}
          </section>
        </main>
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </div>
  )
}

function Stat({ label, value, detail }: { label: string; value: number; detail: string }) {
  return <div><span>{label}</span><strong>{value}</strong><small>{detail}</small></div>
}

function formatMicros(value: number) {
  return (value / 1_000_000).toFixed(6)
}

function parseObject(value: string): Record<string, unknown> {
  const parsed = JSON.parse(value) as unknown
  if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') throw new Error('公开资料必须是 JSON object')
  return parsed as Record<string, unknown>
}

function auditLabel(action: string) {
  const labels: Record<string, string> = {
    'merchant.created': '创建商户',
    'merchant.updated': '更新商户',
    'directory.published': '发布开放目录',
    'directory.unpublished': '撤回开放目录',
    'capability.published': '发布能力',
    'capability.updated': '更新能力',
    'grant.created': '创建授权',
    'grant.revoked': '撤销授权',
    'invocation.succeeded': '调用成功',
    'invocation.failed': '调用失败',
    'invocation.rate_limited': '调用超出配额',
    'invocation.grant_budget_exceeded': '授权总预算已用尽',
    'invocation.grant_budget_rejected': '授权预算拒绝调用',
    'invocation.recovered_failed': '恢复孤儿调用',
    'rate_limit.upserted': '设置调用配额',
    'rate_limit.status_changed': '切换配额状态',
    'app_block.activated': '封禁开发者 App',
    'app_block.released': '解除 App 封禁',
    'runtime.configured': '配置商户运行时',
    'runtime.verified': '验证商户运行时',
  }
  return labels[action] ?? action
}

function errorMessage(error: unknown) {
  if (error instanceof Error) return error.message
  if (error && typeof error === 'object' && 'message' in error) return String(error.message)
  return '操作失败，请稍后重试'
}
