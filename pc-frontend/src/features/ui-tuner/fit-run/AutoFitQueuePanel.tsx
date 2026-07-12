import { ListRestart, PlayCircle, Save } from 'lucide-react'
import type { DesignDiffRegionAnalysis } from '../comparison/autoPairApi'
import type { useAutoFitQueue } from './useAutoFitQueue'
import styles from './UiFitRunPanel.module.css'

interface AutoFitQueuePanelProps {
  analysis: DesignDiffRegionAnalysis | null
  queue: ReturnType<typeof useAutoFitQueue>
}

export function AutoFitQueuePanel({ analysis, queue }: AutoFitQueuePanelProps) {
  const runnable = analysis?.regions.filter((region) => region.recommendedRuntimeNodeId) ?? []
  if (!analysis) return null
  return (
    <section className={styles.queuePanel} aria-label="全页面自动拟合队列">
      <header>
        <div>
          <strong>全页面拟合</strong>
          <span>{queueLabel(queue.phase, queue.currentIndex, queue.regions.length)}</span>
        </div>
        {queue.phase === 'IDLE' ? (
          <button type="button" disabled={runnable.length === 0} onClick={() => queue.start(runnable)}>
            <PlayCircle size={13} aria-hidden="true" />拟合全部 {runnable.length} 个节点
          </button>
        ) : queue.phase === 'READY_TO_COMMIT' ? (
          <button type="button" onClick={() => { void queue.commit() }}>
            <Save size={13} aria-hidden="true" />统一写回并验收
          </button>
        ) : (
          <button type="button" disabled={queue.active} onClick={queue.reset}>
            <ListRestart size={13} aria-hidden="true" />重置队列
          </button>
        )}
      </header>
      {queue.regions.length > 0 && (
        <div className={styles.queueProgress}>
          {queue.regions.map((region, index) => (
            <span
              key={region.id}
              className={index < queue.currentIndex || queue.phase === 'COMPLETED'
                ? styles.queueDone
                : index === queue.currentIndex ? styles.queueCurrent : ''}
              title={region.candidates[0]?.definitionId}
            >
              {index + 1}
            </span>
          ))}
        </div>
      )}
      {queue.error && <p className={styles.queueError}>{queue.error}</p>}
      {queue.phase === 'CODEX_RUNNING' && <p className={styles.queueNotice}>Codex 正在处理必要的结构源码，完成后自动验收。</p>}
      {queue.phase === 'READY_TO_COMMIT' && <p className={styles.queueNotice}>Live 效果已就绪，源码尚未修改；确认后只构建和安装一次。</p>}
    </section>
  )
}

function queueLabel(phase: string, index: number, total: number) {
  if (phase === 'IDLE') return '等待启动'
  if (phase === 'COMPLETED') return `已完成 ${total}/${total}`
  if (phase === 'READY_TO_COMMIT') return `已拟合 ${total}/${total}，待统一写回`
  if (phase === 'COMMITTING') return '正在统一写回和构建验收'
  if (phase === 'CODEX_RUNNING') return 'Codex 正在一次性处理复杂源码'
  if (phase === 'FAILED') return `停在 ${Math.max(index + 1, 1)}/${total}`
  return `正在处理 ${index + 1}/${total}`
}
