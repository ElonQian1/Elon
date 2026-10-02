import { runExchangeWebAdapterCommand } from '../exchange-webview/exchangeWebviewApi'
import { gridReadPort, type GridReadPort } from '../grid-chat/readGridChatSnapshot'
import { gridSource, sameGridSource, type GridSelection, type GridSource } from '../grid-chat/gridChatSnapshot'
import { gridReadRequestId } from '../grid-share/gridShareIdentity'
import { historyFacts } from './gridHistoryModel'

/** Explicit fixed-endpoint history page; no screen automation or present-position enrichment. */
export async function readGridHistory(owner: string, days: number, page: number, active: () => boolean, expected?: GridSource): Promise<GridSelection> {
  return readHistoryPage(gridReadPort(owner), value => runExchangeWebAdapterCommand('binance', owner, 'report', value), days, page, active, expected)
}
export async function readHistoryPage(port: GridReadPort, reportCommand: (value: string) => Promise<void>, days: number, page: number, active: () => boolean, expected?: GridSource): Promise<GridSelection> {
  if (![7, 30, 90].includes(days) || !Number.isInteger(page) || page < 1 || page > 1000) throw Error('历史查询范围无效')
  const source = gridSource(await port.get()), request = gridReadRequestId(), start = port.now()
  if (!active() || expected && !sameGridSource(source, expected)) throw Error('币安来源已变化，请从第一页重新读取')
  await reportCommand(JSON.stringify({ request, kind: 'history', days, page, symbol: '', id: '' }))
  for (let attempt = 0; attempt < 100 && port.now() - start < 35000; attempt++) {
    if (!active()) throw Error('历史读取已取消')
    await port.sleep()
    const state = await port.get()
    if (!active() || !sameGridSource(source, gridSource(state))) throw Error('币安来源已变化，请重新读取')
    const report = state.reports[request]
    if (!report) continue
    if (report.schema !== 'yilong.binance_report_observation.v1' || report.request !== request || report.kind !== 'history'
      || report.account !== source.account || report.account_kind !== source.accountKind || report.page !== page
      || report.status !== 'ready' || report.coverage !== 'page' || !Array.isArray(report.rows) || report.rows.length > 20
      || !Number.isSafeInteger(report.total) || Number(report.total) < report.rows.length || Number(report.total) > 10000000) throw Error('历史报告不可用，请检查币安登录与网络')
    const rows = report.rows.map(historyFacts)
    if (new Set(rows.map(row => row.id)).size !== rows.length) throw Error('历史页包含重复策略，请重新读取')
    return { source, rows, observedAtMs: port.now(), recordKind: 'HISTORY', page, days, total: Number(report.total) }
  }
  throw Error('历史读取超时，请稍后重试')
}
