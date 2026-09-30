import { useEffect, useState } from 'react'
import { useAuthStore } from '../../store/auth'
import { readCenter,secureCenterUrl } from './eskComputeCenterApi'
import { formatCount,formatUnits,holdLabels,sourceLabels,type EskComputeSnapshot } from './eskComputeCenterModel'
import styles from './EskComputeCenter.module.css'

type Section = 'overview' | 'purchases' | 'usage' | 'bills'
const sections: [Section,string][] = [['overview','账户'],['purchases','购入登记'],['usage','AI 用量'],['bills','账单']]

export default function EskComputeCenter({ session }: { session?: { token:string; userId:string } }) {
  const storedToken = useAuthStore(s => s.token)
  const storedUserId = useAuthStore(s => s.user?.id)
  const token = session?.token ?? storedToken
  const userId = session?.userId ?? storedUserId
  const [section,setSection] = useState<Section>('overview')
  const [page,setPage] = useState(1)
  const [refresh,setRefresh] = useState(0)
  const [data,setData] = useState<EskComputeSnapshot | null>(null)
  const [forIdentity,setForIdentity] = useState<string | null>(null)
  const [error,setError] = useState('')
  const [loading,setLoading] = useState(false)
  const [now,setNow] = useState(Date.now())
  const [visible,setVisible] = useState(!document.hidden)
  const identity = `${userId || ''}:${token || ''}`

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()),1000)
    const changed = () => { setVisible(!document.hidden); setData(null); setError('已清除后台账户资料，请重新读取'); setRefresh(v => v + 1) }
    document.addEventListener('visibilitychange',changed)
    return () => { window.clearInterval(timer); document.removeEventListener('visibilitychange',changed) }
  },[])
  useEffect(() => { setPage(1); setSection('overview') },[identity])
  useEffect(() => {
    const controller = new AbortController()
    let active = true
    setData(null); setError(''); setForIdentity(null)
    if (!visible || !token || !userId) { setLoading(false); return () => controller.abort() }
    setLoading(true)
    const timeout = window.setTimeout(() => controller.abort(),15000)
    void readCenter(page,() => token,controller.signal).then(value => {
      if (active && !controller.signal.aborted) { setData(value); setForIdentity(identity); setNow(Date.now()) }
    }).catch(reason => {
      if (!active) return
      if (controller.signal.aborted) setError('读取超时或已取消，请重试')
      else setError(reason instanceof Error ? reason.message : '账户读取失败，请重试')
    }).finally(() => { window.clearTimeout(timeout); if (active) setLoading(false) })
    return () => { active = false; controller.abort(); window.clearTimeout(timeout) }
  },[identity,page,refresh,visible,token,userId])
  useEffect(() => { if (data && data.fresh_until_ms <= now) { setData(null); setError('本次账户资料已到期，请刷新确认最新余额。') } },[data,now])

  const current = visible && forIdentity === identity && data && data.fresh_until_ms > now ? data : null
  const expired = data && data.fresh_until_ms <= now
  const quoteFresh = current?.quote && current.quote.valid_until_ms > now && current.valuation
  const reread = () => { setLoading(false); setRefresh(v => v + 1) }
  return <section className={styles.center} aria-labelledby="esk-compute-title">
    <header className={styles.header}>
      <div><h2 id="esk-compute-title">ESK 与算力</h2><p>余额、购入登记、AI 用量与实际账单</p></div>
      <button type="button" onClick={reread} disabled={loading}>刷新账户</button>
    </header>
    <div className={styles.navigation} role="group" aria-label="账户中心内容">
      {sections.map(([id,label]) => <button key={id} type="button" aria-pressed={section === id}
        onClick={() => setSection(id)}>{label}</button>)}
    </div>
    {!current ? <div className={styles.status} role={error ? 'alert' : 'status'}>
      {expired ? '本次账户资料已到期，请刷新确认最新余额。' : loading ? '正在读取当前账户…' : error || '请登录后读取自己的账户。'}
      {location.protocol !== 'https:' && <p><a href={secureEntry()} target="_blank" rel="noopener noreferrer">在安全窗口查看 ESK 与算力</a></p>}
    </div> : <>
      {section === 'overview' && <>
        <div className={styles.balance}>
          <span>正式 ESK 总登记</span><strong>{formatUnits(current.asset.total_base_units)} <small>ESK</small></strong>
          <p>{quoteFresh && current.valuation ? `参考估值 ${formatUnits(current.valuation.usdt_base_units)} USDT · ¥${formatUnits(current.valuation.cny_base_units)}` : 'USDT / 人民币参考估值暂不可用'}</p>
          {current.quote && <p>来源：{current.quote.source} · {new Date(current.quote.observed_at_ms).toLocaleString('zh-CN')}{!quoteFresh && ' · 报价已过期或估值不可用'}</p>}
        </div>
        <div className={styles.metrics}>
          <div><span>卖回申请占用</span><strong>{formatUnits(current.asset.reserved_base_units)} ESK</strong></div>
          <div><span>剩余正式登记</span><strong>{formatUnits(current.asset.remaining_base_units)} ESK</strong></div>
          <div><span>本月 AI 实际消费</span><strong>¥{formatUnits(current.billing.month_cost_fen,2)} CNY</strong></div>
          <div><span>原人民币可用余额</span><strong>{current.billing.balance_fen === null ? '未开通' : `¥${formatUnits(current.billing.balance_fen,2)} CNY`}</strong></div>
        </div>
        <div className={styles.notice}>
          <strong>ESK 服务支付尚未接入</strong>
          <p>平台后续主要使用 ESK 支付 AI 服务，人民币和 USDT 用于参考折算。正式登记经管理员审核，尚未上链；当前 AI 消费仍以人民币结算，剩余登记量尚不能用于 AI 扣费。</p>
          <button type="button" disabled>购买 ESK · 收款渠道待配置</button>
        </div>
        <h3>当前 AI 预占 · CNY</h3>
        {current.billing.holds.length === 0 ? <p>暂无 AI 预占。</p> : current.billing.holds.map(row => <div key={row.id} className={styles.row}>
          <div><strong>{row.feature} · {row.model || '模型未记录'}</strong><span>{holdLabels[row.status]} · {row.task_reference}</span></div>
          <strong>¥{formatUnits(row.reserved_fen,2)} CNY</strong>
        </div>)}
        {current.billing.holds_has_more && <p>当前显示最近 20 项预占，另有预占未展示。</p>}
      </>}
      {section === 'purchases' && <>
        <h3>正式购入登记 · {formatCount(current.asset.entry_count)} 笔</h3>
        <p>到账依据为管理员审核。这里展示正式登记，不包含 Paper 模拟数量。</p>
        {current.asset.entries.length === 0 ? <p>暂无正式购入登记。</p> : current.asset.entries.map(row => <details key={row.entry_id} className={styles.receipt}>
          <summary><span>{date(row.created_at)}</span><strong>+{formatUnits(row.amount_base_units)} ESK</strong></summary>
          <dl><dt>登记编号</dt><dd>{row.entry_id}</dd><dt>购入审核记录</dt><dd>{row.allocation_id}</dd><dt>到账依据</dt><dd>管理员审核登记</dd></dl>
        </details>)}
        {current.asset.history_next_cursor && <p>当前显示最近 20 笔。完整审核流水可在主 APK「正式 ESK 平台登记」查看。</p>}
      </>}
      {section === 'usage' && <>
        <h3>本月 AI Token 用量</h3><p>按 UTC 自然月统计。Token 是用量单位，价格由模型与服务决定。</p>
        {current.usage_sources.length === 0 ? <p>本月暂无 AI 用量。</p> : current.usage_sources.map(row => <details key={row.billing_source} className={styles.receipt}>
          <summary><span>{sourceLabels[row.billing_source]}</span><strong>{formatCount(row.total_tokens)} Token</strong></summary>
          <dl><dt>输入用量</dt><dd>{formatCount(row.input_tokens)}</dd><dt>缓存输入</dt><dd>{formatCount(row.cached_input_tokens)}</dd>
            <dt>输出用量</dt><dd>{formatCount(row.output_tokens)}</dd><dt>调用数量</dt><dd>{formatCount(row.call_count)}</dd></dl>
          <p>{row.billing_source === 'client_reported' ? '客户端参考上报不作为平台扣费证据。' : row.billing_source === 'own_codex' || row.billing_source === 'user_api_key' ? '本人账号 / 自带 Key 用量不等于平台扣费。' : '具体收费以实际账单为准。'}</p>
        </details>)}
      </>}
      {section === 'bills' && <>
        <h3>AI 实际账单 · CNY</h3><p>历史账单保留结算币种和当时的价格版本，不按当前行情改写为 ESK。</p>
        {current.billing.bills.length === 0 ? <p>本页暂无账单。</p> : current.billing.bills.map(row => <details key={row.id} className={styles.receipt}>
          <summary><span>{row.feature || 'AI 服务'} · {row.model || '模型未记录'}<small>{date(row.created_at)}</small></span><strong>−¥{formatUnits(row.cost_fen,2)}</strong></summary>
          <dl><dt>输入计费用量</dt><dd>{formatCount(row.input_tokens)} Token</dd><dt>缓存计费用量</dt><dd>{formatCount(row.cached_input_tokens)} Token</dd>
            <dt>输出计费用量</dt><dd>{formatCount(row.output_tokens)} Token</dd><dt>价格版本</dt><dd>{row.price_rule_version === null ? row.price_source === 'legacy' ? '历史记录' : '内置价格快照' : `v${row.price_rule_version}`}</dd>
            <dt>账单编号</dt><dd>{row.id}</dd><dt>任务引用</dt><dd>{row.task_reference || '未记录'}</dd><dt>计量回执</dt><dd>{row.token_usage_event_id || '历史记录未关联'}</dd></dl>
        </details>)}
        <div className={styles.navigation}><button type="button" disabled={page <= 1 || loading} onClick={() => setPage(v => v - 1)}>上一页</button>
          <span>第 {current.billing.page} 页</span><button type="button" disabled={!current.billing.has_more || loading || page >= 1000} onClick={() => setPage(v => v + 1)}>下一页</button></div>
      </>}
      <footer className={styles.footer}>账户读取于 {new Date(current.observed_at_ms).toLocaleTimeString('zh-CN')} · 60 秒有效 · 仅当前账户</footer>
    </>}
  </section>
}
function date(value: string) { const parsed = new Date(value); return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString('zh-CN') }
function secureEntry() { try { return `${new URL(secureCenterUrl(1)).origin}/pc/esk-compute` } catch { return undefined } }
