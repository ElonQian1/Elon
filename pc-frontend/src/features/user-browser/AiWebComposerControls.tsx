import { useEffect, useRef, useState } from 'react'
import { AudioLines, ChevronDown, Clock3, Grid3X3, Mic, MicOff, Paperclip, PhoneOff, StopCircle, Wrench } from 'lucide-react'
import type { LocalAiComposerOption, LocalAiFeatureNavigationItem } from './localAiBrowserProtocol'
import {
  findLocalAiRealtimeVoiceControls,
  localAiManagedRealtimeVoiceControllable,
} from './localAiRealtimeVoice'
import {
  isLocalAiInteractionPreset,
  localAiComposerOptionsOrPreset,
  localAiComposerSnapshotFromState,
  localAiFeatureSnapshotFromState,
  localAiFeaturesOrPreset,
  localAiStableInteractionNeedsRefresh,
  resolveLocalAiComposerPreset,
  resolveLocalAiFeaturePreset,
} from './localAiInteractionPresets'
import useLocalAiRealtimeVoiceControl from './useLocalAiRealtimeVoiceControl'
import type { AiWebChatBackend } from './useAiWebChatBackend'
import { isLocalAiAttachmentTransportEvent } from './localAiBrowserApi'
import AiWebAccessRecoveryCard from './AiWebAccessRecoveryCard'
import AiWebRealtimeVoiceDock from './AiWebRealtimeVoiceDock'
import styles from './AiWebComposerControls.module.css'

export { default as AiBrowserExperience } from './AiBrowserExperience'

type Panel = 'model' | 'tools' | 'features' | null
type ComposerSection = Exclude<Panel, 'features' | null>
type CachedMenu<T> = { options: T[]; updatedAt: number }

function realtimeVoiceStatusText(
  control: ReturnType<typeof useLocalAiRealtimeVoiceControl>,
  providerName: string,
) {
  if (control.hangupStatus === 'confirming') return '正在确认官网语音已结束…'
  if (control.hangupStatus === 'unconfirmed') {
    return '官网语音可能仍在通话，请再次挂断或打开官方页确认'
  }
  if (control.activationStatus === 'confirming') {
    if (control.managedVoicePhase === 'requesting_microphone') return '正在请求麦克风权限…'
    if (control.managedVoicePhase === 'creating_offer' || control.managedVoicePhase === 'armed'
      || control.managedVoicePhase === 'applying_answer' || control.managedVoicePhase === 'connecting') {
      return '正在建立 Win 托管实时语音…'
    }
    return '正在确认官网实时语音已连接…'
  }
  if (control.activationStatus === 'unconfirmed') {
    return control.managedFallbackCode
      ? 'Win 托管语音已自动回退；可继续使用官网语音或再次尝试'
      : '官网语音连接尚未确认，可再次尝试或显示官方页'
  }
  if (control.activationStatus === 'active') {
    if (control.managedFallbackCode && !control.managedVoiceActive) {
      return '官网语音已连接 · Win 托管增强已自动回退'
    }
    if (control.managedVoiceActive) {
      if (control.managedMuted) return 'Win 托管实时语音已连接 · 麦克风已静音'
      if (control.privateDataChannelActive && control.managedRemoteAudio) {
        return 'Win 托管实时语音已连接 · 音频与私有转写正常'
      }
      if (control.privateDataChannelActive) return 'Win 托管实时语音已连接 · 私有转写正常'
      return 'Win 托管实时语音已连接'
    }
    return control.privateDataChannelActive
      ? '官网实时语音已连接 · 私有转写通道正常'
      : '官网实时语音已连接'
  }
  if (control.transcriptSyncing) return '正在同步语音转写与回复…'
  return `${providerName} 官方网页会话`
}

export default function AiWebComposerControls({ web }: { web: AiWebChatBackend }) {
  const [panel, setPanel] = useState<Panel>(null)
  const busy = Boolean(web.controller.busyAction)
  const actions = new Set(web.provider?.adapterActions ?? [])
  const snapshot = web.controller.snapshot
  const menuUpdatedAt = web.controller.sessionState?.interactionLive
    ? web.controller.sessionState.interactionUpdatedAtMs || Date.now()
    : 0
  const providerId = web.provider?.id
  const [composerCache, setComposerCache] = useState<Record<ComposerSection, CachedMenu<LocalAiComposerOption>>>({
    model: { options: [], updatedAt: 0 },
    tools: { options: [], updatedAt: 0 },
  })
  const [featureCache, setFeatureCache] = useState<CachedMenu<LocalAiFeatureNavigationItem>>({ options: [], updatedAt: 0 })
  const presetFlight = useRef(false)
  const attachmentRefreshSequence = useRef(0)
  const composerOptions = panel === 'model' || panel === 'tools'
    ? localAiComposerOptionsOrPreset(providerId, panel, composerCache[panel].options)
    : []
  const featureOptions = panel === 'features'
    ? localAiFeaturesOrPreset(providerId, featureCache.options)
    : []
  const temporaryChat = web.controller.uiManifest?.controls.find((control) => control.semantic === 'temporary_chat')
  const realtimeVoice = findLocalAiRealtimeVoiceControls(web.controller.uiManifest?.controls ?? [])
  const realtimeVoiceControl = useLocalAiRealtimeVoiceControl(web)
  const candidateAttachmentTransport = web.controller.sessionState?.attachmentTransportEvent
  const attachmentTransport = isLocalAiAttachmentTransportEvent(candidateAttachmentTransport)
    ? candidateAttachmentTransport
    : null
  const attachmentState = attachmentTransport?.state ?? null
  const attachmentSequence = attachmentTransport?.sequence ?? 0
  const voiceStatusText = realtimeVoiceStatusText(realtimeVoiceControl, web.provider.displayName)
  const managedVoiceControllable = localAiManagedRealtimeVoiceControllable(
    realtimeVoiceControl.managedVoicePhase,
  )
  const voiceDockVisible = realtimeVoiceControl.activationStatus !== 'idle'
    || realtimeVoiceControl.hangupStatus !== 'idle'
    || realtimeVoice.active
    || managedVoiceControllable
  const toggleMuteAction = realtimeVoiceControl.managedVoiceActive
    ? (realtimeVoiceControl.managedMuted ? 'unmute' : 'mute')
    : realtimeVoice.unmute ? 'unmute' : 'mute'
  const toggleMuteControlId = toggleMuteAction === 'unmute'
    ? realtimeVoice.unmute?.id ?? ''
    : realtimeVoice.mute?.id ?? ''
  const canToggleMute = realtimeVoiceControl.managedMicrophoneActive || Boolean(
    toggleMuteAction === 'unmute' ? realtimeVoice.unmute : realtimeVoice.mute,
  )

  useEffect(() => {
    if (attachmentState === 'armed') {
      attachmentRefreshSequence.current = 0
      return
    }
    if (attachmentState !== 'completed' || attachmentSequence <= 0) return
    if (attachmentRefreshSequence.current >= attachmentSequence) return
    attachmentRefreshSequence.current = attachmentSequence
    void web.controller.run('snapshot')
  }, [attachmentSequence, attachmentState, web.controller])

  useEffect(() => {
    setPanel(null)
    const state = web.controller.sessionState
    const model = localAiComposerSnapshotFromState(state, 'model')
    const tools = localAiComposerSnapshotFromState(state, 'tools')
    const features = localAiFeatureSnapshotFromState(state)
    setComposerCache({
      model: { options: model?.options ?? [], updatedAt: model ? menuUpdatedAt : 0 },
      tools: { options: tools?.options ?? [], updatedAt: tools ? menuUpdatedAt : 0 },
    })
    setFeatureCache({ options: features?.features ?? [], updatedAt: features ? menuUpdatedAt : 0 })
  }, [web.controller.sessionIdentity, providerId])

  useEffect(() => {
    const current = web.controller.composerSnapshot
    if (!current?.options.length) return
    setComposerCache((cached) => ({ ...cached, [current.section]: { options: current.options, updatedAt: menuUpdatedAt } }))
  }, [menuUpdatedAt, web.controller.composerSnapshot])

  useEffect(() => {
    const current = web.controller.featureSnapshot
    if (current?.features.length) setFeatureCache({ options: current.features, updatedAt: menuUpdatedAt })
  }, [menuUpdatedAt, web.controller.featureSnapshot])

  async function openComposerPanel(next: ComposerSection) {
    const opening = panel !== next
    setPanel(opening ? next : null)
    const visible = localAiComposerOptionsOrPreset(providerId, next, composerCache[next].options)
    if (opening && localAiStableInteractionNeedsRefresh(visible, composerCache[next].updatedAt)) {
      await web.controller.refreshComposerControls(next)
    }
  }

  async function openFeatures() {
    const opening = panel !== 'features'
    setPanel(opening ? 'features' : null)
    const visible = localAiFeaturesOrPreset(providerId, featureCache.options)
    if (opening && localAiStableInteractionNeedsRefresh(visible, featureCache.updatedAt)) {
      await web.controller.refreshFeatureNavigation()
    }
  }

  async function selectComposerOption(section: ComposerSection, option: LocalAiComposerOption) {
    const action = section === 'model' ? 'select_model_option' : 'select_composer_tool'
    if (!isLocalAiInteractionPreset(option.id)) {
      await web.controller.run(action, option.id)
      return
    }
    if (presetFlight.current) return
    presetFlight.current = true
    try {
      const next = await web.controller.refreshComposerControls(section)
      const live = localAiComposerSnapshotFromState(next, section)?.options ?? []
      const resolved = resolveLocalAiComposerPreset(option, live)
      if (resolved) await web.controller.run(action, resolved.id)
    } finally {
      presetFlight.current = false
    }
  }

  async function selectFeature(option: LocalAiFeatureNavigationItem) {
    if (!isLocalAiInteractionPreset(option.id)) {
      await web.controller.run('select_navigation', option.id)
      return
    }
    if (presetFlight.current) return
    presetFlight.current = true
    try {
      const next = await web.controller.refreshFeatureNavigation()
      const live = localAiFeatureSnapshotFromState(next)?.features ?? []
      const resolved = resolveLocalAiFeaturePreset(option, live)
      if (resolved) await web.controller.run('select_navigation', resolved.id)
    } finally {
      presetFlight.current = false
    }
  }

  async function requestAttachment() {
    await web.controller.control('restore')
    await web.controller.run('request_attachment_upload')
  }

  if (!web.ready || !web.provider) return null

  return (
    <section className={styles.host} aria-label={`${web.provider.displayName} 原生聊天能力`}>
      <AiWebAccessRecoveryCard web={web} />
      <AiWebRealtimeVoiceDock
        visible={voiceDockVisible}
        statusText={voiceStatusText}
        managed={realtimeVoiceControl.managedVoiceActive || managedVoiceControllable}
        connected={realtimeVoiceControl.activationStatus === 'active'}
        microphoneActive={realtimeVoiceControl.managedMicrophoneActive}
        remoteAudio={realtimeVoiceControl.managedRemoteAudio}
        privateTranscript={realtimeVoiceControl.privateDataChannelActive}
        muted={realtimeVoiceControl.managedMuted}
        canToggleMute={canToggleMute}
        canEnd={managedVoiceControllable || Boolean(realtimeVoice.end)}
        busy={busy}
        hangupConfirming={realtimeVoiceControl.hangupStatus === 'confirming'}
        onToggleMute={() => void realtimeVoiceControl.run(toggleMuteAction, toggleMuteControlId)}
        onEnd={() => void realtimeVoiceControl.run('end', realtimeVoice.end?.id ?? '')}
      />
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
        {actions.has('invoke_ui_control') && realtimeVoice.start && !realtimeVoice.active && (
          <button
            type="button"
            onClick={() => void realtimeVoiceControl.run('start', realtimeVoice.start?.id ?? '')}
            disabled={busy || Boolean(web.controller.draft.trim()) || realtimeVoiceControl.activationStatus === 'confirming' || realtimeVoiceControl.hangupStatus === 'confirming' || !realtimeVoice.start.enabled}
            title={web.controller.draft.trim()
              ? '请先发送或清空当前输入，再启动实时语音'
              : '使用 ChatGPT 官网实时语音；首次使用时 WebView2 可能请求麦克风权限'}
          >
            <AudioLines size={13} /><span>{realtimeVoiceControl.activationStatus === 'confirming' ? '正在连接' : '实时语音'}</span>
          </button>
        )}
        {!voiceDockVisible && actions.has('invoke_ui_control') && realtimeVoice.mute && (
          <button
            type="button"
            onClick={() => void realtimeVoiceControl.run('mute', realtimeVoice.mute?.id ?? '')}
            disabled={busy || realtimeVoiceControl.hangupStatus === 'confirming' || !realtimeVoice.mute.enabled}
            title="使用 ChatGPT 官网控件将实时语音静音"
          >
            <MicOff size={13} /><span>静音</span>
          </button>
        )}
        {!voiceDockVisible && actions.has('invoke_ui_control') && realtimeVoice.unmute && (
          <button
            type="button"
            onClick={() => void realtimeVoiceControl.run('unmute', realtimeVoice.unmute?.id ?? '')}
            disabled={busy || realtimeVoiceControl.hangupStatus === 'confirming' || !realtimeVoice.unmute.enabled}
            title="使用 ChatGPT 官网控件恢复麦克风"
          >
            <Mic size={13} /><span>取消静音</span>
          </button>
        )}
        {!voiceDockVisible && actions.has('invoke_ui_control') && realtimeVoice.end && (
          <button
            type="button"
            data-active
            onClick={() => void realtimeVoiceControl.run('end', realtimeVoice.end?.id ?? '')}
            disabled={busy || realtimeVoiceControl.hangupStatus === 'confirming' || !realtimeVoice.end.enabled}
            title="使用 ChatGPT 官网控件结束实时语音"
          >
            <PhoneOff size={13} /><span>{realtimeVoiceControl.hangupStatus === 'unconfirmed'
              ? '再次挂断'
              : realtimeVoiceControl.hangupStatus === 'confirming' ? '正在确认挂断' : '结束语音'}</span>
          </button>
        )}
        <span className={styles.source}>{attachmentState === 'armed'
          ? '请选择要上传的附件…'
          : attachmentState === 'started'
            ? '官网正在上传附件…'
            : attachmentState === 'completed'
              ? `附件已上传${attachmentTransport && attachmentTransport.completedCount > 1 ? ` ${attachmentTransport.completedCount} 个` : ''}`
              : attachmentState === 'failed'
                ? '附件上传失败，可重试或显示官方页检查'
                : voiceStatusText}</span>
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
                <button type="button" role="menuitem" key={option.id} data-selected={option.selected} onClick={() => void selectFeature(option)} disabled={busy}>
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
                  onClick={() => void selectComposerOption(panel, option)}
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
