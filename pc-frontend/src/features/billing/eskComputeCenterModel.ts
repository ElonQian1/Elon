export interface Quote {
  quote_id: string; source: string; observed_at_ms: number; valid_until_ms: number
  usdt_per_esk_base_units: string; cny_per_usdt_base_units: string
}
export interface Purchase { entry_id: string; allocation_id: string; amount_base_units: string; created_at: string }
export interface UsageSource {
  billing_source: string; total_tokens: string; input_tokens: string
  cached_input_tokens: string; output_tokens: string; call_count: string
}
export interface ComputeBill {
  id: string; token_usage_event_id: string | null; task_reference: string | null
  feature: string | null; model: string | null; input_tokens: string
  cached_input_tokens: string; output_tokens: string; cost_fen: string
  price_rule_version: number | null; price_source: string; created_at: string
}
export interface ComputeHold {
  id: string; task_reference: string; feature: string; model: string | null
  reserved_fen: string; status: string; expires_at: string | null
}
export interface EskComputeSnapshot {
  schema: 'yilong.esk.compute_center.v1'; observed_at_ms: number; fresh_until_ms: number
  asset: {
    total_base_units: string; reserved_base_units: string; remaining_base_units: string
    entry_count: string; snapshot_digest: string; entries: Purchase[]; history_next_cursor: string | null
  }
  asset_source: 'platform_recorded'; chain_status: 'not_deployed'; simulated: false; funds_moved: false
  quote: Quote | null; quote_status: string; valuation: { usdt_base_units: string; cny_base_units: string } | null
  valuation_basis: 'reference_only'; month_basis: 'UTC_calendar_month'; usage_sources: UsageSource[]
  billing: {
    currency: 'CNY'; balance_fen: string | null; month_cost_fen: string
    bills: ComputeBill[]; page: number; has_more: boolean; holds: ComputeHold[]; holds_has_more: boolean
  }
  capabilities: { purchase: false; esk_service_spending: false }
  esk_service_reserved_base_units: null; esk_service_spent_base_units: null
  payment_status: string; service_status: string
}

const MAX = 9223372036854775807n
export function units(value: unknown, signed = false): bigint {
  if (typeof value !== 'string' || !(signed ? /^-?(0|[1-9]\d*)$/ : /^(0|[1-9]\d*)$/).test(value)
    || value.length > 20 || value === '-0') throw new Error('无效的金额或用量')
  const result = BigInt(value)
  if (result > MAX || result < -MAX) throw new Error('金额或用量超出范围')
  return result
}
export function formatUnits(value: string, decimals = 6): string {
  const amount = units(value, true)
  const sign = amount < 0n ? '-' : ''
  const absolute = amount < 0n ? -amount : amount
  const scale = 10n ** BigInt(decimals)
  const whole = String(absolute / scale).replace(/\B(?=(\d{3})+(?!\d))/g, ',')
  return `${sign}${whole}.${String(absolute % scale).padStart(decimals, '0')}`
}
export function formatCount(value: string): string {
  return units(value).toLocaleString('zh-CN')
}
function assert(condition: unknown): asserts condition {
  if (!condition) throw new Error('账户资料无效，请重新读取')
}
function label(value: unknown, nullable = false) {
  assert((nullable && value === null) || (typeof value === 'string' && value.length <= 256 && !/[\u0000-\u001f]/.test(value)))
}
export function parseSnapshot(raw: unknown, at = Date.now()): EskComputeSnapshot {
  assert(raw !== null && typeof raw === 'object')
  const v = raw as EskComputeSnapshot
  assert(v.schema === 'yilong.esk.compute_center.v1' && v.asset_source === 'platform_recorded'
    && v.chain_status === 'not_deployed' && v.simulated === false && v.funds_moved === false)
  assert(Number.isSafeInteger(v.observed_at_ms) && Number.isSafeInteger(v.fresh_until_ms)
    && v.observed_at_ms > 0 && v.observed_at_ms <= at + 5000 && v.fresh_until_ms > at
    && v.fresh_until_ms - v.observed_at_ms <= 60000)
  const a = v.asset
  assert(a && units(a.total_base_units) === units(a.reserved_base_units) + units(a.remaining_base_units))
  units(a.entry_count)
  assert(/^[0-9a-f]{64}$/.test(a.snapshot_digest))
  assert(Array.isArray(a.entries) && a.entries.length <= 20)
  a.entries.forEach(row => { label(row.entry_id); label(row.allocation_id); label(row.created_at); units(row.amount_base_units) })
  assert(a.history_next_cursor === null || (typeof a.history_next_cursor === 'string' && /^ephp1\.[0-9a-f]{64}\.eskp_entry_[0-9a-f]{32}$/.test(a.history_next_cursor)))
  assert(Array.isArray(v.usage_sources) && v.usage_sources.length <= 6)
  const sources = new Set<string>()
  v.usage_sources.forEach(row => {
    assert(['platform', 'own_codex', 'shared_codex', 'user_api_key', 'client_reported', 'other'].includes(row.billing_source)
      && !sources.has(row.billing_source))
    sources.add(row.billing_source)
    ;[row.total_tokens,row.input_tokens,row.cached_input_tokens,row.output_tokens,row.call_count].forEach(x => units(x))
  })
  assert(v.valuation_basis === 'reference_only' && v.month_basis === 'UTC_calendar_month')
  if (v.quote !== null) {
    const q = v.quote
    label(q.quote_id); label(q.source)
    assert(Number.isSafeInteger(q.observed_at_ms) && Number.isSafeInteger(q.valid_until_ms)
      && q.observed_at_ms > 0 && q.observed_at_ms <= v.observed_at_ms
      && q.valid_until_ms > v.observed_at_ms && q.valid_until_ms - q.observed_at_ms <= 300000)
    const rate = units(q.usdt_per_esk_base_units), cny = units(q.cny_per_usdt_base_units)
    assert(rate > 0n && cny > 0n)
    if (v.valuation !== null) {
      assert(v.quote_status === 'fresh')
      const numerator = units(a.total_base_units) * rate
      assert(units(v.valuation.usdt_base_units) === (numerator + 500000n) / 1000000n)
      assert(units(v.valuation.cny_base_units) === (numerator * cny + 500000000000n) / 1000000000000n)
    }
  } else assert(v.valuation === null && v.quote_status !== 'fresh')
  const b = v.billing
  assert(b && b.currency === 'CNY' && Number.isSafeInteger(b.page) && b.page >= 1 && b.page <= 1000
    && typeof b.has_more === 'boolean' && typeof b.holds_has_more === 'boolean')
  if (b.balance_fen !== null) units(b.balance_fen, true)
  units(b.month_cost_fen)
  assert(Array.isArray(b.bills) && b.bills.length <= 20 && Array.isArray(b.holds) && b.holds.length <= 20)
  b.bills.forEach(row => {
    label(row.id); label(row.created_at); label(row.model,true); label(row.feature,true)
    label(row.token_usage_event_id,true); label(row.task_reference,true); label(row.price_source)
    ;[row.input_tokens,row.cached_input_tokens,row.output_tokens,row.cost_fen].forEach(x => units(x))
    assert(row.price_rule_version === null || (Number.isSafeInteger(row.price_rule_version) && row.price_rule_version >= 1))
  })
  b.holds.forEach(row => {
    label(row.id); label(row.task_reference); label(row.feature); label(row.model,true); label(row.expires_at,true)
    units(row.reserved_fen); assert(['reserved','dispatch_hold','verification_hold'].includes(row.status))
  })
  assert(v.capabilities.purchase === false && v.capabilities.esk_service_spending === false
    && v.esk_service_reserved_base_units === null && v.esk_service_spent_base_units === null)
  return v
}

export const sourceLabels: Record<string,string> = {
  platform:'平台 AI', own_codex:'本人 AI 账号', shared_codex:'共享 AI 账号',
  user_api_key:'自带 API Key', client_reported:'客户端参考上报', other:'其他来源',
}
export const holdLabels: Record<string,string> = {
  reserved:'调用前预占',dispatch_hold:'任务执行中',verification_hold:'待核验',
}
