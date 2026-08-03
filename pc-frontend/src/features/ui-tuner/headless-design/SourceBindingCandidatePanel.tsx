import { useEffect, useMemo, useState } from 'react'
import type { DesignDraft, DesignSourceBindingCandidate } from './types'
import styles from './SourceBindingCandidatePanel.module.css'

interface Props {
  candidates: DesignSourceBindingCandidate[]
  draft: DesignDraft | null
  busy: boolean
  onApply: (candidate: DesignSourceBindingCandidate, confirmed: boolean) => Promise<unknown>
}

export function SourceBindingCandidatePanel({ candidates, draft, busy, onApply }: Props) {
  const [selectedKey, setSelectedKey] = useState('')
  const selected = useMemo(() => (
    candidates.find((candidate) => candidateKey(candidate) === selectedKey) ?? candidates[0]
  ), [candidates, selectedKey])
  useEffect(() => {
    if (selected && !candidates.some((candidate) => candidateKey(candidate) === selectedKey)) {
      setSelectedKey(candidateKey(selected))
    }
  }, [candidates, selected, selectedKey])
  if (!selected) return null

  const adopted = draft?.sourceBinding?.status === 'CANDIDATE'
    && draft.sourceBinding.sourceFile === selected.suggestedBinding.sourceFile
    && draft.sourceBinding.sourceRevision === selected.suggestedBinding.sourceRevision
    && draft.sourceBinding.range?.start === selected.suggestedBinding.range?.start
    && draft.sourceBinding.range?.end === selected.suggestedBinding.range?.end

  return (
    <section className={styles.panel} aria-label="源码绑定候选审查">
      <div className={styles.list} role="listbox" aria-label="源码候选列表">
        {candidates.map((candidate, index) => {
          const key = candidateKey(candidate)
          return (
            <button
              type="button"
              role="option"
              aria-selected={key === candidateKey(selected)}
              key={key}
              onClick={() => setSelectedKey(key)}
            >
              <strong>{index + 1}. {candidate.file}:{candidate.line}</strong>
              <span>{candidate.suggestedBinding.confidence} · {candidate.score}</span>
            </button>
          )
        })}
      </div>
      <div className={styles.review}>
        <div className={styles.metadata}>
          <code title={selected.sourceSha256}>SHA {selected.sourceSha256.slice(0, 12)}</code>
          <code>bytes {selected.byteRange.start}–{selected.byteRange.end}</code>
          <span>{selected.matchedSignals.join(' · ') || '无额外匹配信号'}</span>
        </div>
        <pre>{selected.excerpt}</pre>
        <div className={styles.actions}>
          <button type="button" disabled={adopted || busy} onClick={() => void onApply(selected, false)}>
            第一步：采用候选
          </button>
          <button type="button" disabled={!adopted || busy} onClick={() => void onApply(selected, true)}>
            第二步：确认 BOUND
          </button>
          <small>{adopted ? '哈希与 byte range 已匹配，可显式确认' : '先采用当前文件、哈希和范围，再允许确认'}</small>
        </div>
      </div>
    </section>
  )
}

function candidateKey(candidate: DesignSourceBindingCandidate) {
  return `${candidate.file}:${candidate.byteRange.start}:${candidate.byteRange.end}:${candidate.sourceSha256}`
}
