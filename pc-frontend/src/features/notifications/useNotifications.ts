import { useEffect, useRef } from 'react'
import { useAuthStore } from '../../store/auth'

const MAX_RECONNECT_MS = 30_000
const DONE_EVENT_TYPE = 'project_task_done'
const GROUP_AI_EVENT_TYPE = 'project_ai_matter_event'
const PROJECT_MESSAGE_EVENT_TYPE = 'project_message_updated'

/** 对应旧 pc_app_notifications.js：WebSocket 任务完成通知 + 声音 + 标题角标 */
export function useNotifications() {
  const token = useAuthStore((s) => s.token)
  // 用 ref 保存所有可变状态，避免 effect 闭包陈旧引用
  const stateRef = useRef({
    socket: null as WebSocket | null,
    reconnectTimer: 0 as ReturnType<typeof setTimeout> | 0,
    reconnectDelay: 1200,
    connectedToken: '',
    audioCtx: null as AudioContext | null,
    lastEventKey: '',
    lastEventAt: 0,
    doneCount: 0,
    titleTimer: 0 as ReturnType<typeof setTimeout> | 0,
    baseTitle: document.title,
  })

  useEffect(() => {
    const s = stateRef.current

    function makeWsUrl(tok: string) {
      const url = new URL('/ws/app', location.href)
      url.protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
      url.searchParams.set('token', tok)
      return url.toString()
    }

    function clearReconnectTimer() {
      if (s.reconnectTimer) {
        clearTimeout(s.reconnectTimer)
        s.reconnectTimer = 0
      }
    }

    function closeSocket() {
      if (s.socket) {
        s.socket.onopen = null
        s.socket.onmessage = null
        s.socket.onclose = null
        s.socket.onerror = null
        s.socket.close()
        s.socket = null
      }
    }

    function scheduleReconnect() {
      clearReconnectTimer()
      s.reconnectTimer = setTimeout(connect, s.reconnectDelay)
      s.reconnectDelay = Math.min(MAX_RECONNECT_MS, Math.round(s.reconnectDelay * 1.8))
    }

    function connect() {
      if (!token) { s.connectedToken = ''; closeSocket(); return }
      clearReconnectTimer()
      if (s.socket && s.connectedToken === token && s.socket.readyState <= WebSocket.OPEN) return

      closeSocket()
      s.connectedToken = token
      const ws = new WebSocket(makeWsUrl(token))
      s.socket = ws
      ws.onopen = () => { s.reconnectDelay = 1200 }
      ws.onmessage = (ev) => handleMessage(ev.data)
      ws.onclose = () => { s.socket = null; if (token) scheduleReconnect() }
      ws.onerror = () => { ws.close() }
    }

    function handleMessage(raw: string) {
      let data: Record<string, unknown>
      try { data = JSON.parse(raw) } catch { return }
      if (!data) return
      if (data.type === PROJECT_MESSAGE_EVENT_TYPE) {
        window.dispatchEvent(new CustomEvent('elon:project-message-updated', { detail: data }))
        return
      }
      if (data.type === GROUP_AI_EVENT_TYPE) {
        handleGroupAiEvent(data)
        return
      }
      if (data.type !== DONE_EVENT_TYPE) return

      const key = [data.projectId ?? '', data.conversationId ?? '', data.message ?? ''].join('|')
      const now = Date.now()
      if (key === s.lastEventKey && now - s.lastEventAt < 60_000) return
      s.lastEventKey = key
      s.lastEventAt = now

      window.dispatchEvent(new CustomEvent('elon:project-task-done', { detail: data }))
      showBrowserNotification(data)
      playDoneSound()
      markTitle()
    }

    function handleGroupAiEvent(data: Record<string, unknown>) {
      const key = [data.projectId ?? '', data.matterId ?? '', data.matterEventType ?? '', data.message ?? ''].join('|')
      const now = Date.now()
      if (key === s.lastEventKey && now - s.lastEventAt < 30_000) return
      s.lastEventKey = key
      s.lastEventAt = now

      window.dispatchEvent(new CustomEvent('elon:project-ai-matter-event', { detail: data }))
      showGroupAiNotification(data)
      markTitle()
    }

    function showGroupAiNotification(data: Record<string, unknown>) {
      if (!('Notification' in window) || Notification.permission !== 'granted') return
      const message = String(data.message ?? '群体 AI Matter 已更新').trim()
      const eventType = String(data.matterEventType ?? '').trim()
      try {
        const n = new Notification('群体 AI 开发已更新', {
          body: `${eventType || 'matter'} · ${message}`.slice(0, 220),
          tag: `elon-group-ai-${data.matterId ?? data.projectId ?? Date.now()}`,
          ...({ renotify: true } as object),
        } as NotificationOptions)
        n.onclick = () => { window.focus(); n.close() }
      } catch { /* ignore */ }
    }

    function showBrowserNotification(data: Record<string, unknown>) {
      if (!('Notification' in window) || Notification.permission !== 'granted') return
      const message = String(data.message ?? '项目会话已完成').trim()
      const body = data.apkUrl ? message + '\nAPK 可以下载测试。' : message
      try {
        const n = new Notification('一龙会话已完成', {
          body: body.slice(0, 220),
          tag: `elon-done-${data.conversationId ?? data.projectId ?? Date.now()}`,
          // renotify 是标准属性但 TS lib 尚未收录，用类型断言绕过
          ...({ renotify: true } as object),
        } as NotificationOptions)
        n.onclick = () => { window.focus(); n.close() }
      } catch { /* ignore */ }
    }

    function markTitle() {
      s.doneCount += 1
      document.title = `(${s.doneCount}) 会话已完成 - ${s.baseTitle}`
      if (s.titleTimer) clearTimeout(s.titleTimer)
      s.titleTimer = setTimeout(() => {
        s.doneCount = 0
        document.title = s.baseTitle
      }, 20_000)
    }

    function ensureAudioCtx(): AudioContext | null {
      const Cls = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
      if (!Cls) return null
      if (!s.audioCtx) s.audioCtx = new Cls()
      return s.audioCtx
    }

    function playDoneSound() {
      const ctx = ensureAudioCtx()
      if (!ctx) return
      const play = () => {
        const gain = ctx.createGain()
        gain.gain.setValueAtTime(0.0001, ctx.currentTime)
        gain.gain.exponentialRampToValueAtTime(0.18, ctx.currentTime + 0.02)
        gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.42)
        gain.connect(ctx.destination)
        ;[740, 988].forEach((freq, i) => {
          const osc = ctx.createOscillator()
          const start = ctx.currentTime + i * 0.16
          osc.type = 'sine'
          osc.frequency.setValueAtTime(freq, start)
          osc.connect(gain)
          osc.start(start)
          osc.stop(start + 0.2)
        })
      }
      if (ctx.state === 'suspended') ctx.resume().then(play).catch(() => {})
      else play()
    }

    function prepareOnGesture() {
      const ctx = ensureAudioCtx()
      if (ctx?.state === 'suspended') ctx.resume().catch(() => {})
      if ('Notification' in window && Notification.permission === 'default') {
        Notification.requestPermission().catch(() => {})
      }
    }

    window.addEventListener('pointerdown', prepareOnGesture, { once: true, passive: true })
    window.addEventListener('keydown', prepareOnGesture, { once: true, passive: true })

    // 定期检测 token 变化后重连
    const interval = setInterval(connect, 2500)
    connect()

    return () => {
      clearInterval(interval)
      clearReconnectTimer()
      closeSocket()
      if (s.titleTimer) clearTimeout(s.titleTimer)
      window.removeEventListener('pointerdown', prepareOnGesture)
      window.removeEventListener('keydown', prepareOnGesture)
    }
  // token 变化时重启整个 effect
  }, [token])
}
