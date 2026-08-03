import { Activity, CornerDownLeft, MousePointerClick, RefreshCw, Square, TextCursorInput } from 'lucide-react'
import type { DesignPlatform, SemanticUiNode } from './types'
import type { DesignRuntimeControlsModel } from './useDesignRuntimeControls'
import styles from './DesignRuntimeControls.module.css'

interface Props {
  platform: DesignPlatform
  selectedNode: SemanticUiNode | null
  disabled: boolean
  model: DesignRuntimeControlsModel
}

export function DesignRuntimeControls({ platform, selectedNode, disabled, model }: Props) {
  const runtimeAvailable = Boolean(model.capabilities)
  const browserReady = model.browserResult?.runtime?.status === 'READY'
    && !['STOPPED', 'NOT_RUNNING', 'STOP_INCOMPLETE'].includes(model.browserResult.status)
  const busy = disabled || Boolean(model.busyAction)
  const candidate = model.bindingCandidates[0]
  const candidateAdopted = Boolean(candidate
    && model.draft?.sourceBinding?.status === 'CANDIDATE'
    && model.draft.sourceBinding.sourceFile === candidate.file)
  return (
    <section className={styles.controls} aria-label="后台设计 Runtime 控制">
      <div className={styles.capability}>
        <Activity size={13} aria-hidden="true" />
        <strong>{model.capabilities?.runtimeSchema ?? '节点待升级'}</strong>
        <span>{model.capabilities ? '后台多端 schema 已确认' : '能力工具尚不可用'}</span>
        <button type="button" title="刷新节点能力" disabled={busy} onClick={() => void model.refreshCapabilities()}>
          <RefreshCw size={12} aria-hidden="true" />
        </button>
      </div>

      {platform !== 'android' && (
        <div className={styles.browserRow}>
          <span>持久浏览器</span>
          <button type="button" disabled={!runtimeAvailable || busy} onClick={() => void model.prepareBrowser(browserReady)}>
            {browserReady ? '重新准备' : '启动并捕获'}
          </button>
          <button type="button" disabled={!browserReady || !selectedNode?.interactive || busy} onClick={() => selectedNode && void model.interact({ action: 'click', selector: selectedNode.selector })}>
            <MousePointerClick size={12} aria-hidden="true" />点击
          </button>
          <button type="button" disabled={!browserReady || !selectedNode || busy} onClick={() => selectedNode && void model.interact({ action: 'scrollIntoView', selector: selectedNode.selector })}>滚动到节点</button>
          <button type="button" disabled={!browserReady || !selectedNode || busy} onClick={() => selectedNode && void model.interact({ action: 'pressKey', selector: selectedNode.selector, key: 'Enter' })}>
            <CornerDownLeft size={12} aria-hidden="true" />Enter
          </button>
          <button type="button" disabled={!browserReady || busy} onClick={() => void model.stopBrowser()}>
            <Square size={11} aria-hidden="true" />停止
          </button>
          <code>{model.browserResult?.runtime ? `${model.browserResult.runtime.operationCount}/${model.browserResult.runtime.limits.maxOperations}` : '0/128'}</code>
        </div>
      )}

      {platform !== 'android' && (
        <div className={styles.fixtureRow}>
          <TextCursorInput size={13} aria-hidden="true" />
          <input value={model.fixtureProfile} onChange={(event) => model.setFixtureProfile(event.currentTarget.value)} placeholder="fixture profile" aria-label="非秘密 fixture profile" />
          <input value={model.fixtureKey} onChange={(event) => model.setFixtureKey(event.currentTarget.value)} placeholder="form key" aria-label="fixture form key" />
          <button type="button" disabled={!browserReady || !model.activeFixtureProfile || !selectedNode || !model.fixtureKey.trim() || busy} onClick={() => selectedNode && void model.interact({ action: 'fill', selector: selectedNode.selector, fixtureKey: model.fixtureKey.trim() })}>填入 fixture</button>
          <small>只传 profile/key，不传真实值；秘密字段和 password/file 永久拒绝。</small>
        </div>
      )}

      <div className={styles.draftRuntimeRow}>
        <span>草稿画面</span>
        <button type="button" disabled={!runtimeAvailable || !model.draft || busy} onClick={() => void model.previewDraft()}>预览草稿</button>
        <button type="button" disabled={!runtimeAvailable || !model.draft || busy} onClick={() => void model.restoreDraft()}>恢复画面</button>
        <button type="button" disabled={!runtimeAvailable || !model.draft || busy} onClick={() => void model.suggestBinding()}>查找源码</button>
        {candidate && (
          <>
            <code title={candidate.excerpt}>{candidate.file}:{candidate.line} · {candidate.score}</code>
            <button type="button" disabled={candidateAdopted || busy} onClick={() => void model.applyBinding(candidate, false)}>采用候选</button>
            <button type="button" disabled={!candidateAdopted || busy} onClick={() => void model.applyBinding(candidate, true)}>确认绑定</button>
          </>
        )}
        <small>{model.draftPreview
          ? `${model.draftPreview.action === 'PREVIEW' ? '正在显示临时预览' : '已恢复'} · 未修改源码`
          : '候选必须先采用，再显式确认 BOUND'}</small>
      </div>

      <div className={styles.matrixRow}>
        {platform === 'tauri' && <button type="button" disabled={!runtimeAvailable || busy} onClick={() => void model.captureBehavior()}>采集菜单 / 对话框 / command trace</button>}
        <button type="button" disabled={!model.canRefreshMatrix || busy} onClick={() => void model.refreshMatrix()}>刷新验证矩阵</button>
        {model.verificationMatrix?.platforms.map((item) => (
          <span key={item.platform} data-status={item.status}>{item.platform.toUpperCase()} {item.status}</span>
        ))}
        <small className={model.error ? styles.error : ''}>{model.error || model.message || '代码能力与真实运行证据分开显示'}</small>
      </div>
    </section>
  )
}
