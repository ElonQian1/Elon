import { getExchangeWebObservation, runExchangeWebAdapterCommand } from '../exchange-webview/exchangeWebviewApi'
import type { ExchangeWebObservation } from '../exchange-webview/exchangeWebviewApi'
import { gridRows, gridSource, object, projectGrid, sameGridSource, validateObservation } from './gridChatSnapshot'
import type { GridAttachment, GridSelection } from './gridChatSnapshot'

export interface GridReadPort {
  get(): Promise<ExchangeWebObservation>
  command(action: 'refresh' | 'detail', value?: string): Promise<void>
  sleep(): Promise<void>
  now(): number
}
export function gridReadPort(ownerKey: string): GridReadPort {
  return {
    get: () => getExchangeWebObservation('binance', ownerKey),
    command: (action, value) => runExchangeWebAdapterCommand('binance', ownerKey, action, value),
    sleep: () => new Promise(resolve => setTimeout(resolve, 350)),
    now: Date.now,
  }
}
function sequence(event: unknown) { return Number(object(event).observationSequence) || 0 }
function checkActive(active: () => boolean) {
  if (!active()) throw new Error('已取消读取。')
}
function fresh(event: unknown, after: number, start: number, now: number) {
  const row = object(event), time = Number(row.observedAtMs)
  return Number.isSafeInteger(time) && time >= start && time <= now + 5000
    && Number.isSafeInteger(row.observationSequence) && sequence(row) > after
}
async function awaitObservation(
  port: GridReadPort, initial: ExchangeWebObservation, kind: 'list' | 'detail',
  id: string, active: () => boolean,
) {
  const start = port.now(), source = gridSource(initial)
  const previous = kind === 'list' ? initial.list : initial.details[id]
  if (initial.list && !Number.isSafeInteger(initial.list.observationSequence)) {
    throw new Error('当前 Win 宿主缺少快照时间证明，请更新并重启一龙后读取。')
  }
  checkActive(active)
  await port.command(kind === 'list' ? 'refresh' : 'detail', id || undefined)
  // Polling exists only inside the explicit click operation, never during send or ordinary chat.
  for (let attempt = 0; attempt < 60 && port.now() - start < 22_000; attempt++) {
    checkActive(active)
    await port.sleep()
    checkActive(active)
    const state = await port.get()
    checkActive(active)
    if (!sameGridSource(source, gridSource(state))) throw new Error('币安账号或页面已变化，请重新读取。')
    if (state.unavailableAtMs >= start) throw new Error('币安读取失败，请检查官网登录与网络，再重新附带。')
    const event = kind === 'list' ? state.list : state.details[id]
    if (fresh(event, sequence(previous), start, port.now())) {
      validateObservation(object(event), source)
      return { state, event: object(event) }
    }
  }
  throw new Error('未收到新的网格数据。请在币安官网打开运行中的 U 本位网格列表，再点击“附带网格”。')
}
export async function readGridSelection(port: GridReadPort, active: () => boolean): Promise<GridSelection> {
  const initial = await port.get()
  const result = await awaitObservation(port, initial, 'list', '', active)
  return gridRows(result.state)
}
export async function readGridAttachment(
  port: GridReadPort, selection: GridSelection, id: string, chatScope: string, active: () => boolean,
): Promise<GridAttachment> {
  const initial = await port.get()
  checkActive(active)
  if (!selection.rows.some(row => row.id === id) || !sameGridSource(selection.source, gridSource(initial))) {
    throw new Error('所选网格的来源已变化，请重新读取列表。')
  }
  const { state, event } = await awaitObservation(port, initial, 'detail', id, active)
  if (!gridRows(state).rows.some(row => row.id === id)) throw new Error('该网格已不在当前列表，请重新选择。')
  const row = object(event.row)
  if (row.id !== id || (row.account != null && row.account !== selection.source.account)) {
    throw new Error('网格详情与所选策略不一致，已取消附带。')
  }
  return { source: selection.source, chatScope, observedAtMs: Number(event.observedAtMs), facts: projectGrid(row) }
}
