import { useEffect, useState, type RefObject } from 'react'
import { createPortal } from 'react-dom'
import { ExternalLink, EyeOff, FolderOpen, MonitorUp, RefreshCw } from 'lucide-react'
import type { AiWebChatBackend } from './useAiWebChatBackend'
import {
  getLocalAiWebResearchCaptureStatus,
  type LocalAiResearchCaptureStatus,
} from './localAiBrowserApi'
import AiProviderSessionStatus from './AiProviderSessionStatus'
import styles from './AiWebProviderPopover.module.css'

export default function AiWebProviderPopover({
  anchorRef,
  web,
  onClose,
}: {
  anchorRef: RefObject<HTMLElement | null>
  web: AiWebChatBackend
  onClose: () => void
}) {
  const [position, setPosition] = useState({ left: 12, bottom: 12 })
  const [researchStatus, setResearchStatus] = useState<LocalAiResearchCaptureStatus>()
  const busy = Boolean(web.controller.busyAction)

  useEffect(() => {
    const reposition = () => {
      const rect = anchorRef.current?.getBoundingClientRect()
      if (!rect) return
      setPosition({
        left: Math.max(12, Math.min(rect.left, window.innerWidth - 372)),
        bottom: Math.max(12, window.innerHeight - rect.top + 8),
      })
    }
    reposition()
    window.addEventListener('resize', reposition)
    return () => window.removeEventListener('resize', reposition)
  }, [anchorRef])

  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => { if (event.key === 'Escape') onClose() }
    document.addEventListener('keydown', closeOnEscape)
    return () => document.removeEventListener('keydown', closeOnEscape)
  }, [onClose])

  useEffect(() => {
    const request = web.officialRequest
    let active = true
    setResearchStatus(undefined)
    if (request) {
      void getLocalAiWebResearchCaptureStatus(request.providerId, request.ownerKey)
        .then((status) => { if (active) setResearchStatus(status) })
        .catch(() => {})
    }
    return () => { active = false }
  }, [web.officialRequest])

  return createPortal(<>
    <button className={styles.backdrop} type="button" aria-label="关闭网页 AI 选择器" onClick={onClose} />
    <section className={styles.popover} style={position} role="dialog" aria-label="选择网页 AI 来源">
      <header><div><strong>选择 Chat 来源</strong><span>共用当前一龙聊天界面</span></div><button type="button" onClick={onClose}>×</button></header>
      <div className={styles.providers}>
        {web.providers.map((provider) => (
          <button key={provider.id} type="button" data-active={provider.id === web.provider?.id} onClick={() => web.selectProvider(provider.id)}>
            <b>{provider.id === 'chatgpt' ? '◎' : 'G'}</b>
            <span><strong>{provider.displayName}</strong><small>{provider.id === 'chatgpt' ? 'ChatGPT 官方网页会话' : 'Google 搜索 AI 模式'}</small></span>
          </button>
        ))}
      </div>
      <div className={styles.session}>
        <AiProviderSessionStatus state={web.userState} compact />
        {web.message && <p>{web.message}</p>}
        {web.controller.sessionState?.diagnostics && (
          <p title="仅包含计数、状态和命令回执，不记录提示词、回复正文、Cookie 或 Token">
            诊断：消息 {web.controller.sessionState.diagnostics.messageCount} · 助手回复 {web.controller.sessionState.diagnostics.assistantMessageCount}
            {web.controller.sessionState.diagnostics.lastCommandAction ? ` · ${web.controller.sessionState.diagnostics.lastCommandAction} ${web.controller.sessionState.diagnostics.lastCommandOk === false ? '失败' : '完成'}` : ''}
          </p>
        )}
        {researchStatus && (
          <p data-warning={researchStatusWarning(researchStatus)}>
            {researchStatusCopy(researchStatus)}
          </p>
        )}
        {web.capability.state !== 'ready' && (
          <button type="button" onClick={() => void web.capability.refresh()} disabled={web.capability.state === 'checking'}>重新检查本地能力</button>
        )}
      </div>
      <div className={styles.actions}>
        <button type="button" onClick={() => void web.controller.openOfficial()} disabled={!web.ready || busy}><MonitorUp size={14} />显示官方页（登录可选）</button>
        <button type="button" onClick={() => void web.controller.control('background')} disabled={!web.ready || !web.controller.sessionState?.windowVisible || busy}><EyeOff size={14} />收起后台</button>
        <button type="button" onClick={() => void web.controller.control('reload')} disabled={!web.ready || !web.controller.sessionOpen || busy}><RefreshCw size={14} />刷新官方页</button>
        <button type="button" onClick={() => void web.controller.control('external')} disabled={busy}><ExternalLink size={14} />系统浏览器</button>
        <button type="button" onClick={() => void web.controller.openResearchDirectory()} disabled={!web.ready || busy}><FolderOpen size={14} />打开研究采样</button>
        {web.provider?.id === 'chatgpt' && web.userState.canStartGoogleLogin && (
          <button type="button" onClick={() => void web.controller.run('start_google_login')} disabled={busy}>使用 Google 登录 ChatGPT</button>
        )}
      </div>
      <footer>上线前开发采样默认开启：受控接口原始响应保存到本机 Profile，最多保留 {web.provider?.researchCaptureRetentionDays || 30} 天；Cookie、Token、请求头和采样内容均不上传一龙云端。</footer>
    </section>
  </>, document.body)
}

function researchStatusWarning(status: LocalAiResearchCaptureStatus) {
  return ['upstream_changed', 'parse_error', 'incomplete'].includes(status.compatibility)
}

function researchStatusCopy(status: LocalAiResearchCaptureStatus) {
  if (researchStatusWarning(status)) {
    return `官网响应结构可能已升级：解码 ${status.decodedFrameCount} 帧，仅识别 ${status.acceptedFrameCount} 帧；原始响应已保存在本机，请更新解析与渲染代码。`
  }
  if (status.compatibility === 'rich_compatible') {
    const kinds = status.richKinds.length ? ` · 富内容 ${status.richKinds.join(' / ')}` : ''
    return `最近私有响应解析正常：识别 ${status.acceptedFrameCount}/${status.decodedFrameCount} 帧${kinds}。`
  }
  if (status.compatibility === 'text_compatible') {
    return `最近私有响应正文解析正常：识别 ${status.acceptedFrameCount}/${status.decodedFrameCount} 帧。`
  }
  if (status.captureCount > 0) {
    return `本机已保存 ${status.captureCount} 份原始响应；最近样本尚无完整结构分析。`
  }
  return '尚未生成本机私有响应采样；发送一次问题后会自动分析。'
}
