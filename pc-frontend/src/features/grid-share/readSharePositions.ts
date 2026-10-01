import { runExchangeWebAdapterCommand } from '../exchange-webview/exchangeWebviewApi'
import { gridSource, object, sameGridSource, type GridAttachment } from '../grid-chat/gridChatSnapshot'
import { gridReadPort } from '../grid-chat/readGridChatSnapshot'

export type ShareReadSnapshot = GridAttachment & { positionNotice?: string }
export const positionMap: Record<string, string> = { quantity: 'positionQty', notional: 'positionNotional', entry: 'entryPrice', mark: 'markPrice', liquidation: 'liquidationPrice', pnl: 'unrealizedPnl' }
export function projectPosition(report: Record<string, unknown>, symbol: string): Record<string, string> {
  if (report.status !== 'ready' || report.coverage !== 'strategy_position' || !Array.isArray(report.rows) || report.rows.length !== 1) throw Error('持仓数据缺失或包含多个方向，本次仅分享已读到的网格参数')
  const row = object(report.rows[0])
  if (row.symbol !== symbol) throw Error('持仓合约不一致')
  const result: Record<string, string> = {}
  for (const [key, target] of Object.entries(positionMap)) {
    const value = row[key]
    if (typeof value !== 'string' || !/^-?(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/.test(value)) continue
    if (['entry', 'mark', 'liquidation'].includes(key) && Number(value) <= 0) continue
    result[target] = value
  }
  return result
}
/** A positions report is requested only as part of the sender's explicit snapshot read. */
export async function readSharePositions(owner: string, attachment: GridAttachment, active: () => boolean): Promise<ShareReadSnapshot> {
  const port = gridReadPort(owner), request = crypto.randomUUID().replace(/-/g, '')
  const start = Date.now()
  try {
    if (!active() || !sameGridSource(attachment.source, gridSource(await port.get()))) throw Error('币安来源已变化')
    await runExchangeWebAdapterCommand('binance', owner, 'report', JSON.stringify({ request, kind: 'positions', page: 1, days: 7, symbol: attachment.facts.symbol, id: attachment.facts.id }))
    for (let attempt = 0; attempt < 100 && Date.now() - start < 35_000; attempt++) {
      if (!active()) throw Error('已取消读取')
      await port.sleep()
      const state = await port.get()
      if (!sameGridSource(attachment.source, gridSource(state))) throw Error('币安来源已变化')
      const report = state.reports[request]
      if (!report) continue
      if (report.schema !== 'yilong.binance_report_observation.v1' || report.request !== request || report.kind !== 'positions'
        || report.account !== attachment.source.account || report.account_kind !== attachment.source.accountKind) throw Error('持仓报告来源不一致')
      return { ...attachment, facts: { ...attachment.facts, ...projectPosition(report, attachment.facts.symbol!) } }
    }
    throw Error('持仓读取超时，本次仅分享网格参数')
  } catch (error) {
    if (!active() || !sameGridSource(attachment.source, gridSource(await port.get()))) throw Error('来源变化或读取已取消，请重新读取')
    return { ...attachment, positionNotice: (error as Error).message }
  }
}
