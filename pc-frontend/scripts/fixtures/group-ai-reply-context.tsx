import React, { useState } from 'react'
import { createRoot } from 'react-dom/client'
import { MemoryRouter, useLocation } from 'react-router-dom'
import { useAuthStore } from '../../src/store/auth'
import GroupAiReplyContext from '../../src/features/friends/group-ai/GroupAiReplyContext'
import GroupAiContinuation from '../../src/features/friends/group-ai/GroupAiContinuation'
import type { AiWebChatBackend } from '../../src/features/user-browser/useAiWebChatBackend'
import type { GroupAiReplyMetadata } from '../../src/features/friends/group-ai/groupAiContext'
import '../../src/styles/globals.css'
useAuthStore.setState({ user: { id: 'fixture-owner', account: 'fixture' } as never, token: 'fixture-only' })
const metadata: GroupAiReplyMetadata = { schema: 1, requester_id: 'fixture-owner', provider: 'chatgpt_web', allow_continue: false, version: 1, source_count: 2,
  previews: [{ sender_name: '甲', text: '讨论下一次版本发布的顺序。' }, { sender_name: '乙', text: '先确认测试结果，再发布。' }] }
function Fixture() {
  const route = useLocation()
  const [draft, setDraft] = useState(new URLSearchParams(location.search).get('draft') ? 'Existing private draft' : '')
  const [url, setUrl] = useState('https://chatgpt.com/c/existing')
  const web = { ready: true, provider: { id: 'chatgpt' }, selectProvider: () => {}, controller: {
    draft, setDraft, snapshot: { url, draft: '', messages: [], streaming: false }, canSubmitDraft: true,
    run: async (action: string) => { if (action !== 'new_conversation') throw new Error('Unexpected action'); setUrl('https://chatgpt.com/') },
  } } as unknown as AiWebChatBackend
  if (route.pathname === '/ai') return <main><GroupAiContinuation owner="fixture-owner" web={web} onMode={() => {}} /><textarea aria-label="私人聊天输入框" value={draft} onChange={e => setDraft(e.target.value)} /></main>
  return <main style={{ maxWidth: 600, padding: 24, margin: 'auto' }}>
  <span data-route>{route.pathname + route.search}</span>
  <div style={{ padding: 16, border: '1px solid #555', borderRadius: 6 }}>建议先完成针对性验收，再安排发布。
    <GroupAiReplyContext owner="fixture-owner" group="fixture-group" message="gai_fixture" metadata={metadata} part="footer" />
  </div><GroupAiReplyContext owner="fixture-owner" group="fixture-group" message="gai_fixture" metadata={metadata} part="sources" />
</main>
}
createRoot(document.getElementById('root')!).render(<MemoryRouter><Fixture /></MemoryRouter>)
