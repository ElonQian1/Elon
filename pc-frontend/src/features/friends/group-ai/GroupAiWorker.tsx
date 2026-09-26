import { useEffect } from 'react'
import { useAuthStore } from '../../../store/auth'
import { getDesktopInvoke } from '../../shell/desktopShell'
import { postWinEvent } from '../../codex-control/codexControlApi'
import { pollGroupAiCommands } from './groupAiControlBridge'

/** No Shell, route navigation, visual interaction or credentials passed over MCP. */
export default function GroupAiWorker() {
  useEffect(() => {
    if (!getDesktopInvoke()) return
    let disposed = false, busy = false
    let previous = localStorage.getItem('elon_auth')
    const report = () => { void postWinEvent({ source: 'frontend', level: 'info', kind: 'group.worker.ready',
      summary: '后台群聊执行页已启动', fields: { route: '/pc/group-ai-worker', logged_in: !!useAuthStore.getState().token },
    }).catch(() => {}) }
    report()
    const syncLogin = async () => {
      const current = localStorage.getItem('elon_auth')
      if (current !== previous) {
        previous = current
        if (current === null) useAuthStore.setState({ token: null, expiresAt: null, user: null })
        else await useAuthStore.persist.rehydrate()
        report()
      }
    }
    const poll = async () => {
      if (busy || disposed) return
      busy = true
      try {
        await syncLogin()
        if (!disposed) await pollGroupAiCommands(true)
      } finally { busy = false }
    }
    const storage = (event: StorageEvent) => {
      if (event.key === null || event.key === 'elon_auth') void syncLogin().catch(() => {})
    }
    window.addEventListener('storage', storage)
    const tick = () => { void poll().catch(() => {}) }
    tick()
    const timer = window.setInterval(tick, 2500)
    return () => { disposed = true; window.clearInterval(timer); window.removeEventListener('storage', storage) }
  }, [])
  return null
}
