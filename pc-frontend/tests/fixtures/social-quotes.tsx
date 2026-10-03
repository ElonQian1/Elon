import { useState } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import SocialConversation from '../../src/features/friends/SocialConversation'
import type { SocialMessage } from '../../src/features/friends/socialMessageTypes'
import { useMessageTimeline } from '../../src/features/message-timeline/useMessageTimeline'
import { useAuthStore } from '../../src/store/auth'
import '../../src/styles/globals.css'
useAuthStore.getState().acceptSession('synthetic-session', '2099-01-01', { id: 'me', account: '我' })

const source: SocialMessage = { id: 'source', sender_user_id: 'friend', sender_name: '示例群友',
  content: 'https://app.binance.com/uni-qr/cpos/fixture', revision: 1, created_at: '2026-09-29T04:00:00Z' }
if (new URLSearchParams(location.search).has('record')) source.content = '【一龙聊天记录】\n' + JSON.stringify({
  schema: 'chat_record_bundle_v1', record_id: 'fixture-record', group_id: 'fixture-group', title: '测试聊天记录',
  summary: '甲：会议资料\n乙：收到', message_count: 2, total_count: 2,
})
const messages: SocialMessage[] = [source, { id: 'reply', sender_user_id: 'me', sender_name: '我', outgoing: true,
  content: '很好，这个视频值得讨论。', created_at: '2026-09-29T04:01:00Z', quote: { message_id: 'source', sender_name: '示例群友',
    content: source.content, revision: 1, attachments: [], unavailable: false } },
  { id: 'old', sender_user_id: 'friend', sender_name: '示例群友', content: '> 引用 我\n> 这里是一段很长的旧版引用。'.repeat(1) + '\n\n这是旧版消息的正文。', created_at: '2026-09-29T04:02:00Z' }]
function Fixture() {
  const [rows, setRows] = useState(messages), [input, setInput] = useState('')
  const [id, setId] = useState('fixture-group')
  const timeline = useMessageTimeline<SocialMessage>(null, setRows)
  return <main style={{ height: '100dvh', display: 'flex', flexDirection: 'column', background: 'var(--bg-base)' }}>
    <header style={{ padding: 8 }}>引用消息组件验收 · 合成数据 <button onClick={() => { setId(id === 'fixture-group' ? 'other' : 'fixture-group'); setRows(messages); setInput('') }}>切换测试会话</button></header>
    <SocialConversation timeline={timeline} conversation={{ kind: 'group', id }} title="示例群聊" me={{ id: 'me', account: '我' }} messages={rows} setMessages={setRows}
      input={input} setInput={setInput} targets={[]} loading={false} error="" retry={() => {}} onSent={() => {}} />
  </main>
}
createRoot(document.getElementById('root')!).render(<BrowserRouter><Fixture /></BrowserRouter>)
