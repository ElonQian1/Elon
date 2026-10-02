import type { GridSelection } from '../grid-chat/gridChatSnapshot'

export default function GridHistoryControls({ selection, busy, read }: { selection: GridSelection | null; busy: boolean; read: (days: number, page: number) => void }) {
  const history = selection?.recordKind === 'HISTORY', page = history ? selection.page ?? 1 : 1, days = history ? selection.days ?? 30 : 30
  return <div aria-label="历史网格读取范围">
    <label>已结束网格范围 <select aria-label="历史时间范围" value={days} disabled={busy} onChange={e => read(Number(e.target.value), 1)}><option value={7}>最近 7 天</option><option value={30}>最近 30 天</option><option value={90}>最近 90 天</option></select></label>
    <button type="button" disabled={busy} onClick={() => read(days, 1)}>读取已结束网格</button>
    {history && <p>第 {page} 页 · 每页最多 20 条 · 范围内共 {selection.total} 条 <button type="button" disabled={busy || page <= 1} onClick={() => read(days, page - 1)}>上一页</button><button type="button" disabled={busy || page * 20 >= (selection.total ?? 0)} onClick={() => read(days, page + 1)}>下一页</button></p>}
  </div>
}
