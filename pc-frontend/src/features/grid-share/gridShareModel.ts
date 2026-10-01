import type { GridAttachment } from '../grid-chat/gridChatSnapshot'

export const SHARE_PREFIX = '【一龙AI对话】\n'
export const SHARE_SCHEMA = 'elon.ai_conversation_share.v1'
export const amountKeys = ['investment', 'initialNotional', 'perGridQty', 'perGridQuoteQty', 'profit', 'matchedPnl', 'fundingFee', 'fee', 'positionQty', 'positionNotional', 'totalPnl', 'unrealizedPnl'] as const
export const labels: Record<string, string> = {
  symbol: '合约', direction: '方向', status: '采集时状态', leverage: '杠杆', lower: '区间下限', upper: '区间上限', count: '网格数量', spacing: '间距',
  markPrice: '标记价格', roi: '策略总收益率 (%)', entryPrice: '持仓均价', liquidationPrice: '预估强平价',
  investment: '投入保证金 (USDT)', initialNotional: '初始货值 (USDT)', perGridQty: '每格基础币数量', perGridQuoteQty: '每格报价币数量',
  profit: '网格利润 (USDT)', matchedPnl: '已配对收益 (USDT)', fundingFee: '资金费 (USDT)', fee: '手续费 (USDT)',
  positionQty: '实际持仓数量', positionNotional: '持仓货值 (USDT)', totalPnl: '策略总盈亏 (USDT)', unrealizedPnl: '未实现盈亏 (USDT)',
  matchedCount: '配对次数', marginType: '保证金模式', orderCurrency: '下单计量', stopUpper: '止损上限', stopLower: '止损下限',
}
export interface GridShare { schema: 'yilong.grid_share.v1'; observed_at_ms: number; show_amounts: boolean; fields: Record<string, string>; note?: string; previous_snapshot_id?: string }
export interface GridShareCard { schema: string; provider: 'binance'; snapshot_id: string; group_id: string; title: string; summary: string; sender_name?: string; grid: GridShare }
export interface GridShareView { snapshot_id: string; group_id: string; owner_id: string; owner_name: string; latest_snapshot_id?: string | null; document: ReturnType<typeof shareDocument> }
export function value(grid: GridShare, key: string) {
  if (!grid.show_amounts && amountKeys.includes(key as typeof amountKeys[number])) return '未公开'
  const v = grid.fields[key]
  return ({ LONG: '做多', SHORT: '做空', NEUTRAL: '中性', ARITH: '等差', GEO: '等比', CROSSED: '全仓', ISOLATED: '逐仓', BASE: '基础币', QUOTE: '报价币', WORKING: '运行中', NEW: '运行中' } as Record<string, string>)[v] ?? v ?? '未读取'
}
export function shareDocument(grid: GridShare) {
  const get = (key: string) => grid.fields[key] ?? '未读取'
  return { schema: SHARE_SCHEMA, provider: 'binance', title: `${get('symbol')} 网格快照`,
    summary: `${value(grid, 'direction') === '未读取' ? '方向未读取' : value(grid, 'direction')} · ${get('leverage')}× · ${get('lower')}–${get('upper')} · ${get('count')} 格 · 历史快照`, messages: [], grid }
}
export function publicGrid(attachment: GridAttachment, showAmounts = false, note = '', previous?: string): GridShare {
  const facts = attachment.facts as unknown as Record<string, string | null>
  const fields = Object.fromEntries(Object.keys(labels).filter(k => (showAmounts || !amountKeys.includes(k as typeof amountKeys[number])) && typeof facts[k] === 'string').map(k => [k, facts[k]!]))
  return { schema: 'yilong.grid_share.v1', observed_at_ms: attachment.observedAtMs, show_amounts: showAmounts, fields,
    ...(note.trim() ? { note: note.trim().slice(0, 200) } : {}), ...(previous ? { previous_snapshot_id: previous } : {}) }
}
export function gridCard(content: string): GridShareCard | null {
  try {
    if (!content.startsWith(SHARE_PREFIX) || content.length > 8000) return null
    const card = JSON.parse(content.slice(SHARE_PREFIX.length)) as GridShareCard
    return card.schema === SHARE_SCHEMA && card.provider === 'binance' && /^ai_snapshot_[\w-]+$/.test(card.snapshot_id)
      && /^[\w-]+$/.test(card.group_id) && card.grid?.schema === 'yilong.grid_share.v1' && typeof card.grid.fields?.symbol === 'string' ? card : null
  } catch { return null }
}
export const detailSections: Record<string, string[]> = {
  '持仓': ['positionQty', 'positionNotional', 'entryPrice', 'markPrice', 'liquidationPrice', 'marginType'],
  '收益': ['roi', 'totalPnl', 'profit', 'matchedPnl', 'unrealizedPnl', 'fundingFee', 'fee', 'matchedCount'],
  '参数': ['direction', 'leverage', 'lower', 'upper', 'count', 'spacing', 'investment', 'initialNotional', 'perGridQty', 'perGridQuoteQty', 'orderCurrency', 'stopUpper', 'stopLower'],
}
