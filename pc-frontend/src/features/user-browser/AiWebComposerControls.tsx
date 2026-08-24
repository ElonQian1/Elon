import { useEffect, useState } from 'react'
import { ChevronDown, Clock3, Grid3X3, Mic, Paperclip, StopCircle, Wrench } from 'lucide-react'
import type { LocalAiComposerOption, LocalAiFeatureNavigationItem } from './localAiBrowserProtocol'
import type { AiWebChatBackend } from './useAiWebChatBackend'
import AiWebAccessRecoveryCard from './AiWebAccessRecoveryCard'
import styles from './AiWebComposerControls.module.css'

export { default as AiBrowserExperience } from './AiBrowserExperience'

type Panel = 'model' | 'tools' | 'features' | null
type ComposerSection = Exclude<Panel, 'features' | null>
type CachedMenu<T> = { options: T[]; updatedAt: number }
const MENU_CACHE_TTL_MS = 60_000

export default function AiWebComposerControls({ web }: { web: AiWebChatBackend }) {
  const [panel, setPanel] = useState<Panel>(null)
  const busy = Boolean(web.controller.busyAction)
  const actions = new Set(web.provider?.adapterActions ?? [])
  const snapshot = web.controller.snapshot
  const menuUpdatedAt = web.controller.sessionState?.updatedAtMs || Date.now()
  const [composerCache, setComposerCache] = useState<Record<ComposerSection, CachedMenu<LocalAiComposerOption>>>({
    model: { options: [], updatedAt: 0 },
    tools: { options: [], updatedAt: 0 },
  })
  const [featureCache, setFeatureCache] = useState<CachedMenu<LocalAiFeatureNavigationItem>>({ options: [], updatedAt: 0 })
  const composerOptions = panel === 'model' || panel === 'tools' ? composerCache[panel].options : []
  const featureOptions = panel === 'features' ? featureCache.options : []
  const temporaryChat = web.controller.uiManifest?.controls.find((control) => control.semantic === 'temporary_chat')

  useEffect(() => {
    setPanel(null)
    setComposerCache({ model: { options: [], updatedAt: 0 }, tools: { options: [], updatedAt: 0 } })
    setFeatureCache({ options: [], updatedAt: 0 })
  }, [web.provider?.id])

  useEffect(() => {
    const current = web.controller.composerSnapshot
    if (!current) return
    setComposerCache((cached) => ({ ...cached, [current.section]: { options: current.options, updatedAt: menuUpdatedAt } }))
  }, [menuUpdatedAt, web.controller.composerSnapshot])

  useEffect(() => {
    const current = web.controller.featureSnapshot
    if (current) setFeatureCache({ options: current.features, updatedAt: menuUpdatedAt })
  }, [menuUpdatedAt, web.controller.featureSnapshot])

  async function openComposerPanel(next: ComposerSection) {
    const opening = panel !== next
    setPanel(opening ? next : null)
    if (opening && menuNeedsRefresh(composerCache[next])) await web.controller.refreshComposerControls(next)
  }

  async function openFeatures() {
    const opening = panel !== 'features'
    setPanel(opening ? 'features' : null)
    if (opening && menuNeedsRefresh(featureCache)) await web.controller.refreshFeatureNavigation()
  }

  async function requestAttachment() {
    await web.controller.control('restore')
    await web.controller.run('request_attachment_upload')
  }

  if (!web.ready || !web.provider) return null

  return (
    <section className={styles.host} aria-label={`${web.provider.displayName} 原生聊天能力`}>
      <AiWebAccessRecoveryCard web={web} />
      <div className={styles.toolbar}>
        {actions.has('list_model_options') && (
          <button type="button" data-active={panel === 'model'} onClick={() => void openComposerPanel('model')} disabled={busy}>
            <span>{snapshot?.currentModel || '选择模型'}</span><ChevronDown size={13} />
          </button>
        )}
        {actions.has('list_composer_tools') && (
          <button type="button" data-active={panel === 'tools'} onClick={() => void openComposerPanel('tools')} disabled={busy}>
            <Wrench size={13} /><span>工具</span>
          </button>
        )}
        {actions.has('request_attachment_upload') && (
          <button type="button" onClick={() => void requestAttachment()} disabled={busy || !web.canCompose} title="由 ChatGPT 官方文件选择器读取附件">
            <Paperclip size={13} /><span>附件</span>
          </button>
        )}
        {actions.has('start_dictation') && !snapshot?.dictationActive && (
          <button type="button" onClick={() => void web.controller.run('start_dictation')} disabled={busy || !web.canCompose} title="首次使用可能需要在官方窗口允许麦克风">
            <Mic size={13} /><span>听写</span>
          </button>
        )}
        {snapshot?.dictationActive && (
          <>
            <button type="button" data-active onClick={() => void web.controller.run('submit_dictation')} disabled={busy}>
              <Mic size={13} /><span>完成听写</span>
            </button>
            <button type="button" onClick={() => void web.controller.run('cancel_dictation')} disabled={busy}>
              <StopCircle size={13} /><span>取消</span>
            </button>
          </>
        )}
        {actions.has('list_navigation') && (
          <button type="button" data-active={panel === 'features'} onClick={() => void openFeatures()} disabled={busy}>
            <Grid3X3 size={13} /><span>ChatGPT 功能</span>
          </button>
        )}
        {temporaryChat && actions.has('invoke_ui_control') && (
          <button
            type="button"
            data-active={temporaryChat.selected}
            onClick={() => void web.controller.run('invoke_ui_control', temporaryChat.id)}
            disabled={busy || !temporaryChat.enabled}
            title="使用 ChatGPT 官网当前的临时聊天开关"
          >
            <Clock3 size={13} /><span>{temporaryChat.selected ? '临时聊天已开' : '临时聊天'}</span>
          </button>
        )}
        <span className={styles.source}>{web.provider.displayName} 官方网页会话</span>
      </div>

      {snapshot?.attachments?.length ? (
        <div className={styles.attachments} aria-label="待发送附件">
          {snapshot.attachments.map((attachment) => (
            <span key={attachment.id} data-state={attachment.state}>
              <Paperclip size={12} />
              <b>{attachment.name}</b>
              <small>{attachment.state === 'uploading' ? '上传中' : attachment.state === 'error' ? '失败' : '已就绪'}</small>
              {attachment.removable && actions.has('remove_attachment') && (
                <button type="button" aria-label={`移除 ${attachment.name}`} onClick={() => void web.controller.run('remove_attachment', attachment.id)} disabled={busy}>×</button>
              )}
            </span>
          ))}
        </div>
      ) : null}

      {panel && (
        <div className={styles.panel} role="menu" aria-label={panelLabel(panel)}>
          {panel === 'features'
            ? featureOptions.map((option) => (
                <button type="button" role="menuitem" key={option.id} data-selected={option.selected} onClick={() => void web.controller.run('select_navigation', option.id)} disabled={busy}>
                  <span>{option.label}</span><small>{option.kind}</small>
                </button>
              ))
            : composerOptions.map((option) => (
                <button
                  type="button"
                  role="menuitemradio"
                  aria-checked={option.selected}
                  key={option.id}
                  data-selected={option.selected}
                  onClick={() => void web.controller.run(panel === 'model' ? 'select_model_option' : 'select_composer_tool', option.id)}
                  disabled={busy}
                >
                  <span>{option.label}</span>
                  <small>{option.selected ? '当前' : option.opensSubmenu ? '展开' : option.semantic || option.kind}</small>
                </button>
              ))}
          {(panel === 'features' ? featureOptions : composerOptions).length === 0 && (
            <p>{busy ? '正在读取官网可见选项…' : '当前官网没有返回可用选项，可显示官方页检查。'}</p>
          )}
        </div>
      )}
    </section>
  )
}

function panelLabel(panel: Exclude<Panel, null>) {
  if (panel === 'model') return '选择 ChatGPT 模型'
  if (panel === 'tools') return '选择 ChatGPT 工具'
  return '打开 ChatGPT 功能'
}

function menuNeedsRefresh(value: { options: unknown[]; updatedAt: number }) {
  return value.options.length === 0 || Date.now() - value.updatedAt >= MENU_CACHE_TTL_MS
}
