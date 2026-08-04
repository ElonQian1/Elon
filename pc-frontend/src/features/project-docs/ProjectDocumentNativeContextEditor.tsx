import { useEffect, useMemo, useState } from 'react'
import { Save, X } from 'lucide-react'

import type { NativeContextCandidate } from './projectDocumentNativeContextModel'
import styles from './ProjectDocumentNativeContextInbox.module.css'

interface Props {
  candidate: NativeContextCandidate
  disabled: boolean
  onCancel: () => void
  onSave: (summary: string, topics: string[]) => void | Promise<void>
}

export default function ProjectDocumentNativeContextEditor({ candidate, disabled, onCancel, onSave }: Props) {
  const [summary, setSummary] = useState(candidate.summary)
  const [topicsText, setTopicsText] = useState(candidate.topics.join(', '))

  useEffect(() => {
    setSummary(candidate.summary)
    setTopicsText(candidate.topics.join(', '))
  }, [candidate.candidate_id, candidate.summary, candidate.topics, candidate.updated_at_ms])

  const topics = useMemo(() => normalizeTopics(topicsText), [topicsText])
  const summaryLength = summary.trim().replace(/\s+/g, ' ').length
  const valid = summaryLength >= 12 && summaryLength <= 800 && topics.length > 0

  return (
    <form className={styles.editor} onSubmit={(event) => {
      event.preventDefault()
      if (valid && !disabled) void onSave(summary, topics)
    }}>
      <label>
        <span>候选摘要</span>
        <textarea value={summary} maxLength={800} rows={3} disabled={disabled}
          onChange={(event) => setSummary(event.target.value)} />
        <small>{summaryLength}/800，至少 12 字符</small>
      </label>
      <label>
        <span>Topics</span>
        <input value={topicsText} maxLength={400} disabled={disabled}
          onChange={(event) => setTopicsText(event.target.value)}
          placeholder="用逗号或换行分隔，最多 8 个" />
        <small>{topics.length}/8；只修改导航标签，证据路径和 hash 保持不变</small>
      </label>
      <div>
        <button type="button" onClick={onCancel} disabled={disabled}><X size={14} />取消</button>
        <button type="submit" className={styles.saveEdit} disabled={!valid || disabled}>
          <Save size={14} />{disabled ? '保存中…' : '保存修订'}
        </button>
      </div>
    </form>
  )
}

function normalizeTopics(value: string): string[] {
  const seen = new Set<string>()
  return value
    .split(/[,\n，]/)
    .map((topic) => topic.trim().replace(/\s+/g, ' ').slice(0, 48))
    .filter((topic) => {
      const key = topic.toLocaleLowerCase()
      return !!topic && !seen.has(key) && !!seen.add(key)
    })
    .slice(0, 8)
}
