import { value, type GridShare } from './gridShareModel'
import styles from './GridShare.module.css'
import GridTokenIcon from './GridTokenIcon'
import { displayProfit, profitTone } from './gridSharePickerModel'
import { isHistory } from '../grid-history/gridHistoryModel'

export default function GridShareSummary({ grid }: { grid: GridShare }) {
  const history = isHistory(grid.fields)
  const low = Number(grid.fields.lower), high = Number(grid.fields.upper), mark = Number(grid.fields.markPrice)
  const ratio = Number.isFinite(mark) && high > low ? Math.min(100, Math.max(0, (mark - low) / (high - low) * 100)) : null
  const metric = history ? 'totalPnl' : grid.fields.roi ? 'roi' : grid.fields.totalPnl ? 'totalPnl' : 'profit'
  const metricLabel = history ? grid.fields.settlement === 'CONFIRMED' ? '最终总盈亏' : '结束记录总盈亏 · 结算未确认' : { roi: '策略总收益率', totalPnl: '策略总盈亏', profit: '网格利润' }[metric]
  const visible = metric === 'roi' || grid.show_amounts
  return <>
    <span className={styles.sourceLabel}>◆ 币安 · {history ? '历史网格' : '网格快照'}</span>
    <span className={styles.coinHeading}><GridTokenIcon symbol={grid.fields.symbol} /><strong>{grid.fields.symbol}</strong></span>
    <div className={styles.tags}><span data-direction={grid.fields.direction}>{value(grid, 'direction')}</span><span>{value(grid, 'leverage')}×</span><span>{value(grid, 'count')} 格 · {value(grid, 'spacing')}</span></div>
    <div className={styles.profitPanel}><div className={styles.hero} data-tone={visible ? profitTone(grid.fields[metric]) : 'neutral'}>{visible ? displayProfit(grid.fields[metric]) + (grid.fields[metric] ? metric === 'roi' ? '%' : ' USDT' : '') : '历史分享未公开金额'}</div><span className={styles.eyebrow}>{visible ? metricLabel : '分享者仅公开策略参数'}{visible && metric === 'profit' ? ' · 不代表总盈亏' : ''}</span></div>
    {grid.show_amounts && grid.fields.unrealizedPnl && <span className={styles.pnlLine}>未实现盈亏 <strong data-tone={profitTone(grid.fields.unrealizedPnl)}>{displayProfit(grid.fields.unrealizedPnl)} USDT</strong></span>}
    {history && <><span>已结束 · 仓位{value(grid, 'positionState')} · 结算{value(grid, 'settlement')}</span><span className={styles.pnlLine}>网格利润（非总盈亏） <strong data-tone={grid.show_amounts ? profitTone(grid.fields.profit) : 'neutral'}>{grid.show_amounts ? displayProfit(grid.fields.profit) : '未公开'} USDT</strong></span><span>结束 {value(grid, 'end')}</span></>}
    {grid.show_amounts && <dl className={styles.amountSummary}>{(history ? ['investment'] as const : ['investment', 'positionQty', 'positionNotional'] as const).map(key => <div key={key}><dt>{{ investment: '投入保证金 · USDT', positionQty: '持仓数量', positionNotional: '持仓货值 · USDT' }[key]}</dt><dd>{value(grid, key)}</dd></div>)}</dl>}
    <div><div className={styles.range}><span>{value(grid, 'lower')}</span><span>{value(grid, 'upper')}</span></div>{!history && <div className={styles.track}>{ratio != null && <i className={styles.marker} style={{ left: `${ratio}%` }} />}</div>}<span className={styles.eyebrow}>{history ? '结束记录价格区间' : `价格区间 · 标记价 ${value(grid, 'markPrice')}`}</span></div>
    <span className={styles.time}>{new Date(grid.observed_at_ms).toLocaleString()} 的快照 · {grid.show_amounts ? '金额与数量全部公开' : '旧快照 · 金额与数量未公开'}</span>
  </>
}
