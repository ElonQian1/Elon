import { useMemo, useState } from 'react'
import { Search, ChevronRight, ArrowDownUp } from 'lucide-react'
import type { GridSelection } from '../grid-chat/gridChatSnapshot'
import { directionName, displayProfit, profitTone, selectGridRows, tokenSymbol, type GridSort } from './gridSharePickerModel'
import GridTokenIcon from './GridTokenIcon'
import styles from './GridSharePicker.module.css'
import { historyTime } from '../grid-history/gridHistoryModel'

export default function GridSharePicker({ selection, busy, onSelect }: { selection: GridSelection; busy: boolean; onSelect: (id: string) => void }) {
  const [query, setQuery] = useState(''), [sort, setSort] = useState<GridSort>(selection.recordKind === 'HISTORY' ? 'ended-desc' : 'profit-desc')
  const rows = useMemo(() => selectGridRows(selection.rows, query, sort), [selection.rows, query, sort])
  return <section className={styles.picker} aria-label="选择要分享的币安网格">
    <div className={styles.heading}><div><h3>选择一个网格</h3><p>币种和收益一眼可见，选择后检查分享内容。</p></div><span>{selection.rows.length} 个{selection.recordKind === 'HISTORY' ? '已结束（本页）' : '运行中'}</span></div>
    <div className={styles.toolbar}>
      <label className={styles.search}><Search size={18} aria-hidden="true" /><input aria-label="搜索币种或策略编号" placeholder="搜索币种，例如 QNT" value={query} onChange={e => setQuery(e.target.value)} /></label>
      <label className={styles.sort}><ArrowDownUp size={16} aria-hidden="true" /><select aria-label="网格排序" value={sort} onChange={e => setSort(e.target.value as GridSort)}>{selection.recordKind === 'HISTORY' && <option value="ended-desc">最近结束优先</option>}<option value="profit-desc">网格利润从高到低</option><option value="profit-asc">网格利润从低到高</option><option value="symbol">按币种名称</option></select></label>
    </div>
    <p className={styles.explanation}>网格利润不等于策略总盈亏。{selection.recordKind === 'HISTORY' ? '历史结算与仓位处理未确认时保持未知；不会读取今天的持仓补入历史。' : '未实现盈亏在选中后读取。'}发送仍以预览中的公开设置为准。</p>
    <div className={styles.list} aria-busy={busy}>
      {rows.map(row => <button className={styles.item} type="button" key={row.id} disabled={busy} onClick={() => onSelect(row.id!)} aria-label={`选择 ${row.symbol} ${directionName(row.direction)} ${row.leverage ?? '?'}倍 网格利润 ${row.profit ?? '未读取'} USDT 策略 ${row.id}`}>
        <GridTokenIcon symbol={row.symbol!} />
        <span className={styles.identity}><span className={styles.symbol}>{tokenSymbol(row.symbol!)}<small>/ USDT</small></span><span className={styles.meta}><b data-direction={row.direction}>{directionName(row.direction)}</b><span>{row.leverage ?? '—'}×</span><span>{row.count ?? '—'} 格</span></span><span className={styles.range}>{row.lower ?? '—'} – {row.upper ?? '—'} <small>#{row.id}</small></span></span>
        <span className={styles.profit}><small>网格利润 · USDT</small><strong data-tone={profitTone(row.profit)} title={row.profit ?? undefined}>{displayProfit(row.profit)}</strong>{selection.recordKind === 'HISTORY' && <small>结束 {historyTime(row.end)}</small>}</span>
        <ChevronRight className={styles.chevron} size={18} aria-hidden="true" />
      </button>)}
      {!rows.length && <p className={styles.empty}>{selection.rows.length ? '没有匹配的网格，试试币种简称或策略编号。' : selection.recordKind === 'HISTORY' ? '本页没有匹配的已结束网格，可更换范围或继续翻页。' : '当前账户没有运行中的网格。可保留此账户，或在币安官网核对登录。'}</p>}
    </div>
    <p className={styles.asof}>本次读取 {new Date(selection.observedAtMs).toLocaleTimeString()} · {rows.length} / {selection.rows.length} 个网格</p>
  </section>
}
