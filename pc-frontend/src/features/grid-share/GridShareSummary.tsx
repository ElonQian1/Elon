import { value, type GridShare } from './gridShareModel'
import styles from './GridShare.module.css'
import GridTokenIcon from './GridTokenIcon'
import { displayProfit, profitTone } from './gridSharePickerModel'

export default function GridShareSummary({ grid }: { grid: GridShare }) {
  const low = Number(grid.fields.lower), high = Number(grid.fields.upper), mark = Number(grid.fields.markPrice)
  const ratio = Number.isFinite(mark) && high > low ? Math.min(100, Math.max(0, (mark - low) / (high - low) * 100)) : null
  const metric = grid.fields.roi ? 'roi' : grid.fields.totalPnl ? 'totalPnl' : 'profit'
  const metricLabel = { roi: '策略总收益率', totalPnl: '策略总盈亏', profit: '网格利润' }[metric]
  const visible = metric === 'roi' || grid.show_amounts
  return <>
    <span className={styles.sourceLabel}>◆ 币安 · 网格快照</span>
    <span className={styles.coinHeading}><GridTokenIcon symbol={grid.fields.symbol} /><strong>{grid.fields.symbol}</strong></span>
    <div className={styles.tags}><span data-direction={grid.fields.direction}>{value(grid, 'direction')}</span><span>{value(grid, 'leverage')}×</span><span>{value(grid, 'count')} 格 · {value(grid, 'spacing')}</span></div>
    <div className={styles.profitPanel}><div className={styles.hero} data-tone={visible ? profitTone(grid.fields[metric]) : 'neutral'}>{visible ? displayProfit(grid.fields[metric]) + (grid.fields[metric] ? metric === 'roi' ? '%' : ' USDT' : '') : '历史分享未公开金额'}</div><span className={styles.eyebrow}>{visible ? metricLabel : '分享者仅公开策略参数'}{visible && metric === 'profit' ? ' · 不代表总盈亏' : ''}</span></div>
    {grid.show_amounts && grid.fields.unrealizedPnl && <span className={styles.pnlLine}>未实现盈亏 <strong data-tone={profitTone(grid.fields.unrealizedPnl)}>{displayProfit(grid.fields.unrealizedPnl)} USDT</strong></span>}
    {grid.show_amounts && <dl className={styles.amountSummary}>{(['investment', 'positionQty', 'positionNotional'] as const).map(key => <div key={key}><dt>{{ investment: '投入保证金 · USDT', positionQty: '持仓数量', positionNotional: '持仓货值 · USDT' }[key]}</dt><dd>{value(grid, key)}</dd></div>)}</dl>}
    <div><div className={styles.range}><span>{value(grid, 'lower')}</span><span>{value(grid, 'upper')}</span></div><div className={styles.track}>{ratio != null && <i className={styles.marker} style={{ left: `${ratio}%` }} />}</div><span className={styles.eyebrow}>价格区间 · 标记价 {value(grid, 'markPrice')}</span></div>
    <span className={styles.time}>{new Date(grid.observed_at_ms).toLocaleString()} 的快照 · {grid.show_amounts ? '金额与数量全部公开' : '旧快照 · 金额与数量未公开'}</span>
  </>
}
