import { useEffect, useState } from 'react'
import { AlertTriangle, CircleCheck, LoaderCircle, MonitorUp, RotateCcw } from 'lucide-react'
import {
  getLocalAiNativeWindowState,
  localAiBrowserErrorMessage,
  type LocalAiNativeWindowState,
  type LocalAiWebProvider,
} from './localAiBrowserApi'
import styles from './NativeWindowStatusCard.module.css'

interface NativeWindowStatusCardProps {
  provider: LocalAiWebProvider
  ownerKey: string
  compact?: boolean
  busy?: boolean
  onRecover?: () => void
}

export default function NativeWindowStatusCard({
  provider,
  ownerKey,
  compact = false,
  busy = false,
  onRecover,
}: NativeWindowStatusCardProps) {
  const [state, setState] = useState<LocalAiNativeWindowState | null>(null)
  const [error, setError] = useState('')

  useEffect(() => {
    let active = true
    let timer = 0
    setState(null)
    setError('')
    const poll = async () => {
      if (!ownerKey) return
      try {
        const next = await getLocalAiNativeWindowState(provider.id, ownerKey)
        if (active) {
          setState(next)
          setError('')
        }
      } catch (reason) {
        if (active) {
          const message = localAiBrowserErrorMessage(reason)
          setState(null)
          setError(message.includes('尚未创建') ? '' : message)
        }
      }
      if (active) timer = window.setTimeout(() => void poll(), 1_500)
    }
    void poll()
    return () => {
      active = false
      window.clearTimeout(timer)
    }
  }, [ownerKey, provider.id])

  const view = statusView(state, error)
  const Icon = view.tone === 'ready'
    ? CircleCheck
    : view.tone === 'error'
      ? AlertTriangle
      : state ? LoaderCircle : MonitorUp

  return (
    <section className={styles.card} data-tone={view.tone} data-compact={compact} aria-label="一龙聊天窗状态">
      <Icon className={view.tone === 'loading' ? styles.spin : ''} size={compact ? 15 : 22} />
      <div>
        <strong>{view.title}</strong>
        <p>{view.detail}</p>
      </div>
      {onRecover && (state?.retryable || error) && (
        <button type="button" onClick={onRecover} disabled={busy}>
          <RotateCcw size={14} />恢复聊天窗
        </button>
      )}
    </section>
  )
}

function statusView(state: LocalAiNativeWindowState | null, error: string) {
  if (error) return { tone: 'error', title: '状态读取失败', detail: error }
  if (!state) return {
    tone: 'idle',
    title: '一龙聊天窗尚未打开',
    detail: '打开后会在这里显示真实的创建、页面加载和渲染就绪状态。',
  }
  if (state.phase === 'ready' && state.pageReady) return {
    tone: 'ready',
    title: '一龙聊天窗已就绪',
    detail: `${state.focused ? '窗口已聚焦' : '窗口在后台'} · 原生页面根节点已完成渲染`,
  }
  if (state.phase === 'error') return {
    tone: 'error',
    title: '一龙聊天窗需要恢复',
    detail: errorDetail(state.lastErrorCode),
  }
  if (state.phase === 'closed') return {
    tone: 'idle',
    title: '一龙聊天窗已关闭',
    detail: '官方网页会话不会因此被清除；可以重新打开原生聊天窗。',
  }
  return {
    tone: 'loading',
    title: state.phase === 'creating' ? '正在创建一龙聊天窗' : '正在加载一龙聊天页面',
    detail: '窗口会保留到页面就绪或显示稳定错误，不再静默闪退。',
  }
}

function errorDetail(code?: string | null): string {
  if (code === 'root_empty') return '页面已加载，但一龙 React 根节点为空；请恢复窗口或查看 Codex 控制台。'
  if (code === 'page_runtime_error') return '一龙聊天页面发生运行错误；窗口已保留，可直接重试。'
  if (code === 'webview_navigation_error') return 'WebView2 无法加载一龙聊天页面；窗口已保留诊断页。'
  if (code === 'webview_create_failed') return 'WebView2 创建失败；请更新或重启 Win 客户端后重试。'
  return '窗口没有完成健康检查；可以重新打开，原有厂商 Cookie 不会被清除。'
}
