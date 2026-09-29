import { useState } from 'react'
import { Bot, ChevronRight, FileText } from 'lucide-react'
import { recordCard } from './recordApi'
import ChatRecordReader from './ChatRecordReader'
import styles from './ChatRecords.module.css'

export default function ChatRecordMessage({ content, group, onAnalyze }: { content: string; group: string; onAnalyze?: () => void }) {
  const card = recordCard(content)
  const [open, setOpen] = useState(false)
  if (!card || card.group_id !== group) return <span>聊天记录不可用</span>
  return <><button className={styles.card} type="button" onClick={() => setOpen(true)} aria-label={`查看聊天记录：${card.title}`}>
    <strong>{card.title}</strong><span>{card.summary}</span><small><FileText size={16} />聊天记录 · {card.message_count} 条<ChevronRight size={16} /></small>
  </button>{onAnalyze && <button type="button" className={styles.analyze} onClick={onAnalyze}><Bot size={16} aria-hidden="true" />AI 分析记录</button>}
    {open && <ChatRecordReader card={card} onClose={() => setOpen(false)} />}</>
}
