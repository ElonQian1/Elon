import { value, type GridShare } from './gridShareModel'
import styles from './GridShare.module.css'

export default function GridShareSummary({ grid }: { grid: GridShare }) {
  const low = Number(grid.fields.lower), high = Number(grid.fields.upper), mark = Number(grid.fields.markPrice)
  const ratio = Number.isFinite(mark) && high > low ? Math.min(100, Math.max(0, (mark - low) / (high - low) * 100)) : null
  return <>
    <span className={styles.eyebrow}>币安 · 网格持仓分享</span>
    <strong>{grid.fields.symbol}</strong>
    <div className={styles.tags}><span>{value(grid, 'direction')}</span><span>{value(grid, 'leverage')}×</span><span>{value(grid, 'count')} 格 · {value(grid, 'spacing')}</span></div>
    <div><div className={styles.hero}>{grid.fields.roi ? `${grid.fields.roi}%` : '—'}</div><span className={styles.eyebrow}>策略总收益率{grid.fields.roi ? '' : ' · 未读取'}</span></div>
    <div><div className={styles.range}><span>{value(grid, 'lower')}</span><span>{value(grid, 'upper')}</span></div><div className={styles.track}>{ratio != null && <i className={styles.marker} style={{ left: `${ratio}%` }} />}</div><span className={styles.eyebrow}>价格区间 · 标记价 {value(grid, 'markPrice')}</span></div>
    <span className={styles.time}>{new Date(grid.observed_at_ms).toLocaleString()} 的快照 · {grid.show_amounts ? '已公开金额' : '金额与数量未公开'}</span>
  </>
}
