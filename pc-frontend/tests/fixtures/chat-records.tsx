// Local fixture renders the real reader with synthetic, HTTP-served data only.
import React, { useState } from 'react'
import { createRoot } from 'react-dom/client'
import ChatRecordReader from '../../src/features/friends/chat-records/ChatRecordReader'
import { useAuthStore } from '../../src/store/auth'
import type { RecordCard } from '../../src/features/friends/chat-records/recordApi'
import '../../src/styles/globals.css'

useAuthStore.getState().acceptSession('synthetic-session', '2099-01-01', { id: 'reader', account: 'Fixture' })
const card: RecordCard = { schema: 'chat_record_bundle_v1', record_id: 'record_test', group_id: 'group_test', title: '微信聊天记录', summary: '示例', message_count: 5, total_count: 6 }
function Fixture() {
  const [open, setOpen] = useState(false)
  return <><button onClick={() => setOpen(true)}>打开测试记录</button>{open && <ChatRecordReader card={card} onClose={() => setOpen(false)} />}</>
}
createRoot(document.getElementById('root')!).render(<Fixture />)
