import { object, projectGrid, type GridAttachment, type GridSelection } from '../grid-chat/gridChatSnapshot'

export { HISTORY_NOTICE } from './gridHistoryNotice'
const integer = /^[1-9][0-9]{0,15}$/
export function historyFacts(raw: unknown) {
  const row = object(raw)
  if (['NEW', 'WORKING', 'RUNNING'].includes(String(row.status))) throw Error('历史列表混入运行中记录，请重新读取')
  const facts = projectGrid({ ...row, metrics: row })
  facts.recordKind = 'HISTORY'
  facts.end = typeof row.end === 'string' && integer.test(row.end) ? row.end : null
  if (facts.created && Number(facts.created) > Date.now() + 5000) throw Error('历史记录开始时间无效')
  if (facts.end && facts.created && Number(facts.end) < Number(facts.created)) throw Error('历史记录结束时间早于开始时间')
  if (facts.end && Number(facts.end) > Date.now() + 5000) throw Error('历史记录结束时间无效')
  // Source history is gridProfit, not a verified final total. Never enrich it with today's positions.
  Object.assign(facts, { settlement: 'UNKNOWN', positionState: 'UNKNOWN', endReason: 'UNKNOWN', feeBasis: 'UNKNOWN', roiBasis: 'UNKNOWN' })
  return facts
}
export function historicalAttachment(selection: GridSelection, id: string, scope: string): GridAttachment {
  const facts = selection.rows.find(row => row.id === id)
  if (selection.recordKind !== 'HISTORY' || !facts || facts.recordKind !== 'HISTORY') throw Error('历史记录不在本次选择范围')
  return { source: selection.source, observedAtMs: selection.observedAtMs, chatScope: scope, facts }
}
export const isHistory = (fields: Record<string, string | null | undefined>) => fields.recordKind === 'HISTORY'
export function historyTime(raw: string | null | undefined) {
  return raw && integer.test(raw) ? new Date(Number(raw)).toLocaleString() : '未读取'
}
