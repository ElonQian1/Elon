import { useEffect, useRef, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import type { AiWebChatBackend } from '../../user-browser/useAiWebChatBackend'
import { clearGroupContinuation, groupContinuation } from './groupAiContext'
import type { AiHomeMode } from '../../ai/AiHomeModeSwitch'
import styles from './GroupAiReplyContext.module.css'

export default function GroupAiContinuation({ owner, web, onMode }: { owner: string; web: AiWebChatBackend; onMode: (mode: AiHomeMode) => void }) {
  const [query] = useSearchParams()
  const id = query.get('groupDiscussion') || ''
  const handoff = groupContinuation(id, owner)
  const [phase, setPhase] = useState<'idle' | 'opening' | 'ready' | 'failed'>('idle')
  const [error, setError] = useState('')
  const started = useRef(0)
  const commandFinished = useRef(false)
  const origin = useRef({ owner, id, group: handoff?.group || '' })
  if (origin.current.owner !== owner || origin.current.id !== id) origin.current = { owner, id, group: handoff?.group || '' }
  const [readyFor, setReadyFor] = useState('')
  const live = useRef({ owner, id, web }); live.current = { owner, id, web }
  const navigate = useNavigate()
  useEffect(() => { if (handoff) { onMode('chat'); web.selectProvider('chatgpt') } }, [id, owner])
  useEffect(() => {
    if (phase !== 'opening' || !handoff) return
    const timer = window.setInterval(() => {
      const current = live.current
      if (current.id !== id || current.owner !== owner) return
      const c = current.web.controller, s = c.snapshot
      const fresh = s?.url === 'https://chatgpt.com/' || s?.url === 'https://chatgpt.com'
      if (commandFinished.current && fresh && current.web.provider?.id === 'chatgpt' && c.canSubmitDraft && !c.newConversationRecoveryActive &&
          !c.busyAction && s && !s.streaming && !s.messages.length && !s.draft?.trim() && !c.draft.trim()) {
        c.setDraft(handoff.draft); setReadyFor(`${owner}:${id}`); setPhase('ready'); clearGroupContinuation(id)
      } else if (Date.now() - started.current > 30000) {
        setPhase('failed'); setError('尚未确认空白私人会话，未填入或发送内容。请重试。')
      }
    }, 250)
    return () => window.clearInterval(timer)
  }, [phase, id, owner])
  if (!id || (!handoff && readyFor !== `${owner}:${id}`)) return null
  async function begin() {
    const c = web.controller
    if (c.draft.trim() || c.snapshot?.draft?.trim() || c.snapshot?.streaming || c.busyAction || c.queuedSendActive) {
      setError('当前私人会话还有草稿或正在回复，请先处理；不会覆盖或发送。'); return
    }
    setError(''); commandFinished.current = false; setPhase('opening'); started.current = Date.now()
    try {
      await c.run('new_conversation')
      if (live.current.owner === owner && live.current.id === id) commandFinished.current = true
    } catch { setPhase('failed'); setError('新会话未打开，未填入或发送内容。请重试。') }
  }
  return <div className={styles.handoff} role="status" data-group-discussion>
    <span>{phase === 'ready' ? '精选上下文已放入输入框，发送后开始私人讨论。' : phase === 'opening' ? '正在打开私人会话…' : error || '准备使用自己的 ChatGPT 继续讨论。'}</span>
    {(phase === 'idle' || phase === 'failed') && <button type="button" disabled={!web.ready || web.provider?.id !== 'chatgpt'} onClick={() => void begin()}>开始私人讨论</button>}
    <button type="button" onClick={() => { clearGroupContinuation(id); navigate(`/friends?group=${encodeURIComponent(origin.current.group)}`) }}>返回群聊</button>
  </div>
}
