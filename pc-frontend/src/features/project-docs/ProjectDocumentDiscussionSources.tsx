import { CheckCircle2, FileClock, Play } from 'lucide-react'

import type { DiscussionSource } from './projectDocumentDiscussionModel'
import styles from './ProjectDocumentDiscussionMap.module.css'

interface Props {
  sources: DiscussionSource[]
  canStartAi: boolean
  organizing: boolean
  readOnly: boolean
  onResume: (source: DiscussionSource) => void
}

export default function ProjectDocumentDiscussionSources({
  sources,
  canStartAi,
  organizing,
  readOnly,
  onResume,
}: Props) {
  const processed = sources.reduce((sum, source) => sum + source.processed_chunk_ids.length, 0)
  const chunks = sources.reduce((sum, source) => sum + source.chunk_count, 0)
  const incomplete = sources.filter((source) => source.compilation_status !== 'complete')

  return (
    <div className={styles.sourceStrip}>
      <header>
        <FileClock size={14} />
        <div><strong>来源编译</strong><small>{sources.length
          ? `${sources.length} 个来源 · ${processed}/${chunks || '?'} chunks · ${incomplete.length} 个待续编`
          : '导入聊天后，AI 会按稳定分块增量编译'}</small></div>
      </header>
      <div>
        {sources.slice(0, 16).map((source) => {
          const complete = source.compilation_status === 'complete'
          const resumable = /^docs\/inbox\/conversations\/.+\.md$/i.test(source.reference)
          return <article key={source.id} data-complete={complete}>
            {complete ? <CheckCircle2 size={12} /> : <FileClock size={12} />}
            <span title={source.title}>{source.title}</span>
            <small>{source.processed_chunk_ids.length}/{source.chunk_count || '?'}</small>
            {!complete && <button type="button" title={resumable ? '从未处理的 chunk 继续编译' : '来源路径缺失，需先由 AI 修复引用'}
              disabled={!canStartAi || organizing || readOnly || !resumable} onClick={() => onResume(source)}>
              <Play size={10} />续编
            </button>}
          </article>
        })}
        {!sources.length && <span className={styles.sourceEmpty}>尚无来源</span>}
      </div>
    </div>
  )
}
