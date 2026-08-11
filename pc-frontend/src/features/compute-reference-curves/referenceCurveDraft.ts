import {
  type ComputePriceComponent,
  type MyComputeOfferView,
} from '../compute-supply/computeOfferApi'
import { type ReferenceCurveEntryIntent } from './computeReferenceCurveApi'

export function componentsFromOffer(view: MyComputeOfferView): ComputePriceComponent[] {
  return view.offer.price_terms.components.map((component) => ({ ...component }))
}

export function buildReferenceEntry(
  view: MyComputeOfferView,
  deliveryWindowId: string,
  components: ComputePriceComponent[],
  ordinal: number,
): ReferenceCurveEntryIntent {
  const offer = view.offer
  if (offer.status !== 'active') throw new Error('只有 active Offer 可以加入参考价格批次')
  if (offer.price_terms.currency !== 'CNY') throw new Error('平台参考价格 V1 只支持 CNY')
  if (!['spot', 'capacity_future'].includes(offer.price_terms.pricing_mode)) {
    throw new Error('平台参考价格 V1 只支持 spot 或 capacity_future')
  }
  const window = offer.delivery_windows.find((item) => item.binding.window_id === deliveryWindowId)
  if (!window) throw new Error('请选择当前 Offer 的交付窗口')
  if (!components.length) throw new Error('报价至少需要一个价格组件')
  components.forEach(validateComponent)
  const consumerMax = components.reduce((total, item) => total + componentTotal(item, 'consumer'), 0)
  const providerMax = components.reduce((total, item) => total + componentTotal(item, 'provider'), 0)
  if (!Number.isSafeInteger(consumerMax) || !Number.isSafeInteger(providerMax)) {
    throw new Error('价格组件合计超出安全整数范围')
  }
  return {
    entry_key: `entry-${String(ordinal).padStart(4, '0')}`,
    provider_id: offer.provider_id,
    offer_id: offer.offer_id,
    offer_version: offer.offer_version,
    offer_digest: offer.offer_digest,
    sku_id: offer.sku.sku_id,
    sku_digest: offer.sku.sku_digest,
    delivery_window_id: window.binding.window_id,
    delivery_window_digest: window.binding.window_digest,
    pricing_mode: offer.price_terms.pricing_mode as 'spot' | 'capacity_future',
    currency: 'CNY',
    offer_curve_id: offer.price_terms.curve_id,
    offer_curve_version: offer.price_terms.curve_version,
    instrument_id: offer.price_terms.instrument_id,
    components: components.map((component) => ({ ...component })),
    fee_rules: [],
    consumer_max_amount_micros: consumerMax,
    provider_max_amount_micros: providerMax,
  }
}

export function reindexEntries(entries: ReferenceCurveEntryIntent[]): ReferenceCurveEntryIntent[] {
  return entries.map((entry, index) => ({ ...entry, entry_key: `entry-${String(index + 1).padStart(4, '0')}` }))
}

export function defaultLocalTime(offsetMinutes: number): string {
  const date = new Date(Date.now() + offsetMinutes * 60_000)
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000)
  return local.toISOString().slice(0, 16)
}

export function localTimeToIso(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) throw new Error('价格曲线时间无效')
  return date.toISOString()
}

export function formatMicros(value: number): string {
  return `¥${(value / 1_000_000).toFixed(4)}`
}

function validateComponent(component: ComputePriceComponent) {
  for (const [label, value] of [
    ['单位粒度', component.unit_size],
    ['最大数量', component.max_units],
  ] as const) {
    if (!Number.isSafeInteger(value) || value <= 0) throw new Error(`${component.meter} 的${label}必须为正整数`)
  }
  for (const [label, value] of [
    ['消费者单价', component.consumer_unit_price_micros],
    ['供给者单价', component.provider_unit_price_micros],
  ] as const) {
    if (!Number.isSafeInteger(value) || value < 0) throw new Error(`${component.meter} 的${label}必须为非负整数`)
  }
  if (component.max_units % component.unit_size !== 0) throw new Error(`${component.meter} 的最大数量必须是单位粒度的整数倍`)
  if (component.provider_unit_price_micros > component.consumer_unit_price_micros) {
    throw new Error(`${component.meter} 的供给者单价不能高于消费者单价`)
  }
}

function componentTotal(component: ComputePriceComponent, side: 'consumer' | 'provider') {
  const price = side === 'consumer'
    ? component.consumer_unit_price_micros
    : component.provider_unit_price_micros
  return (component.max_units / component.unit_size) * price
}
