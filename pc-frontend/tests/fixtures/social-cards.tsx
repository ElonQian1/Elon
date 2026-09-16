// Local-only Vite fixture. Uses production React/PWA renderers and synthetic messages.
import React, { useEffect, useRef, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import SocialConversation from '../../src/features/friends/SocialConversation'
import type { SocialMessage } from '../../src/features/friends/socialMessageTypes'
import '../../src/styles/globals.css'

const samples = [
  'https://mp.weixin.qq.com/s/fixture-article',
  '45 【城市散步：把日常画成一本旅行手账 ✨ - 测试作者 | 小红书 - 你的生活兴趣社区】 😆 synthetic123 😆 https://www.xiaohongshu.com/discovery/item/fixture?xsec_token=synthetic&xsec_source=pc_share',
  '3.53 复制打开抖音，看看【测试工作室的作品】《我的世界》第一季合集 https://v.douyin.com/fixture/ aNW:/ i@p.QX :9pm 02/07',
  '【用镜头记录一座城市，看看我们身边平常而有趣的故事】 【精准空降到 01:20】 https://www.bilibili.com/video/BV1enYL6SEtU/?share_source=copy_web&t=80&p=3',
  'https://app.binance.com/uni-qr/cpos/123456?r=synthetic&l=zh-CN',
  'https://x.com/example/status/123456',
]
const me = { id: 'fixture-me', account: '示例用户' }
const initial: SocialMessage[] = [...samples, `附加评论必须保留 ${samples[4]}`].map((content, i) => ({
  id: `fixture-${i}`, content, sender_user_id: i % 2 ? 'fixture-friend' : me.id,
  sender_name: '示例群友', outgoing: i % 2 === 0, created_at: '2026-09-16T08:00:00Z',
}))
function PwaCards() {
  const host = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const cleanups = samples.map((text, i) => {
      const bubble = document.createElement('div'); bubble.className = 'bubble'; bubble.textContent = text; host.current!.append(bubble)
      ElonSocialLinks.prepareBubble(bubble, text)
      return ElonSocialLinks.mount(bubble, text, { compact: true, owner: 'fixture-pwa', api: async (_path, options) => {
        const response = await fetch('/api/me/link-preview', options); return response.json()
      } })
    })
    return () => { cleanups.forEach(cleanup => cleanup()); host.current?.replaceChildren() }
  }, [])
  return <div ref={host} style={{ display: 'grid', gap: 16, justifyContent: 'end', padding: 24, background: '#080a0d' }} />
}
function Fixture() {
  const [messages, setMessages] = useState(initial)
  const [input, setInput] = useState('')
  return <main style={{ height: '100dvh', display: 'flex', flexDirection: 'column', background: '#232527' }}>
    <header style={{ padding: '8px 16px', fontSize: 12, color: '#b7bdc8' }}>组件验收 · 合成消息 · 无真实聊天写入</header>
    <button type="button" onClick={() => setMessages(old => old.map(m => m.id === 'fixture-0' ? { ...m, content: samples[3], revision: 2 } : m))}>模拟消息更新</button>
    <SocialConversation conversation={{ kind: 'group', id: 'fixture-group' }} title="示例群聊" me={me} messages={messages} setMessages={setMessages}
      input={input} setInput={setInput} targets={[]} loading={false} error="" retry={() => {}} onSent={() => {}} />
  </main>
}
createRoot(document.getElementById('root')!).render(<BrowserRouter>{new URLSearchParams(location.search).has('pwa') ? <PwaCards /> : <Fixture />}</BrowserRouter>)
