import React, { useState } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import SocialConversation from '../../src/features/friends/SocialConversation'
import { useMessageTimeline } from '../../src/features/message-timeline/useMessageTimeline'
import { useAuthStore } from '../../src/store/auth'
import type { SocialMessage } from '../../src/features/friends/socialMessageTypes'
import '../../src/styles/globals.css'
const user = { id: 'reading-fixture', account: '阅读测试' }
useAuthStore.setState({ user, token: 'synthetic-fixture' })
function Fixture() {
  const [messages, setMessages] = useState<SocialMessage[]>([]), [input, setInput] = useState('')
  const scope = { kind: 'group' as const, id: 'reading-group' }
  const timeline = useMessageTimeline(scope, setMessages)
  return <main style={{ height: '100dvh', display: 'flex' }}><SocialConversation conversation={scope} title="合成测试群" me={user}
    timeline={timeline} messages={messages} setMessages={setMessages} input={input} setInput={setInput} targets={[]}
    loading={timeline.loading} error={timeline.error} retry={timeline.refresh} onSent={() => {}} /></main>
}
createRoot(document.getElementById('root')!).render(<BrowserRouter><Fixture /></BrowserRouter>)
