import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  AppWindow,
  Camera,
  Globe2,
  MonitorCog,
  MousePointerClick,
  Play,
  RefreshCw,
  RotateCcw,
  Save,
  Search,
  Smartphone,
  Square,
} from 'lucide-react'
import { UiTunerProjectSessionPanel } from '../UiTunerProjectSessionPanel'
import { UiWorkspaceModeBar } from '../workspace/UiWorkspaceModeBar'
import type { SourcePreviewMode } from '../source-preview/types'
import { buildHeadlessDesignContext } from './headlessDesignContext'
import { DesignRuntimeControls } from './DesignRuntimeControls'
import { DesignPlanningReview } from './DesignPlanningReview'
import type { DesignPlatform, SemanticUiNode } from './types'
import type { DesignIntentPlan } from './designPlanningTypes'
import { useHeadlessDesignSession } from './useHeadlessDesignSession'
import { useDesignTaskEventSync } from './useDesignTaskEventSync'
import { useDesignRuntimeControls } from './useDesignRuntimeControls'
import { useDesignPlanningControls } from './useDesignPlanningControls'
import styles from './HeadlessDesignWorkspace.module.css'

interface Props {
  active: boolean
  initialProjectRoot: string
  onModeChange: (mode: SourcePreviewMode) => void
}

const PLATFORM_OPTIONS: Array<{
  id: DesignPlatform
  label: string
  icon: typeof Globe2
}> = [
  { id: 'web', label: 'Web', icon: Globe2 },
  { id: 'pwa', label: 'PWA', icon: Smartphone },
  { id: 'tauri', label: 'Tauri', icon: AppWindow },
  { id: 'android', label: 'Android', icon: MonitorCog },
]

export function HeadlessDesignWorkspace({ active, initialProjectRoot, onModeChange }: Props) {
  const model = useHeadlessDesignSession(active, initialProjectRoot)
  const followIntentPlan = useCallback(async (plan: DesignIntentPlan) => {
    const reusable = model.sessions.find((item) => item.designSessionId === plan.designSessionId)
    if (reusable) {
      await model.selectSession(reusable)
      return
    }
    await model.prepareIntentTarget(plan.primaryPlatform ?? model.platform, plan.route)
  }, [model])
  const taskFollow = useDesignTaskEventSync({
    active,
    projectRoot: initialProjectRoot,
    designSessionId: model.session?.designSessionId,
    draftId: model.designDraft?.draftId,
    reload: model.reload,
  })
  const runtimeControls = useDesignRuntimeControls({
    active,
    projectRoot: initialProjectRoot,
    session: model.session,
    draft: model.designDraft,
    initialTauriBehavior: model.surface?.nativeBehavior ?? null,
    onEvidenceChanged: model.reload,
  })
  const planningControls = useDesignPlanningControls({
    projectRoot: initialProjectRoot,
    platform: model.platform,
    route: model.route,
    session: model.session,
    draft: model.designDraft,
    onPlan: followIntentPlan,
  })
  const pack = useMemo(() => buildHeadlessDesignContext({
    projectRoot: initialProjectRoot,
    target: model.target,
    session: model.session,
    surface: model.surface,
    selectedNode: model.selectedNode,
    designDraft: model.designDraft,
    writebackReceipt: model.writebackReceipt,
    capabilities: runtimeControls.capabilities,
    browserRuntime: runtimeControls.browserResult?.runtime ?? null,
    tauriBehavior: runtimeControls.tauriBehavior ?? model.surface?.nativeBehavior ?? null,
    verificationMatrix: runtimeControls.verificationMatrix,
    draftPreview: runtimeControls.draftPreview,
    sourceBindingCandidates: runtimeControls.bindingCandidates,
    intentPlan: planningControls.intentPlan,
    bindingHealth: planningControls.bindingHealth,
    writebackPlan: planningControls.writebackPlan,
    liveFollow: taskFollow,
  }), [initialProjectRoot, model.designDraft, model.selectedNode, model.session, model.surface, model.target, model.writebackReceipt, planningControls.bindingHealth, planningControls.intentPlan, planningControls.writebackPlan, runtimeControls.bindingCandidates, runtimeControls.browserResult?.runtime, runtimeControls.capabilities, runtimeControls.draftPreview, runtimeControls.tauriBehavior, runtimeControls.verificationMatrix, taskFollow])
  const intent = useMemo(() => [
    `修改 ${model.platform.toUpperCase()} 端 ${model.route || '/'} 页面。`,
    model.selectedNode
      ? `当前选中 ${model.selectedNode.label || model.selectedNode.selector}（${model.selectedNode.selector}）。`
      : '先读取当前后台页面和语义 UI 树，再选择需要修改的节点。',
    '修改后重新捕获同一 designSession，并报告新的 UI tree 与 PNG 哈希。',
  ].join(''), [model.platform, model.route, model.selectedNode])
  const viewport = model.surface?.surface?.viewport ?? model.session?.viewport ?? model.viewport
  const nodes = model.surface?.nodes ?? []
  const [surfaceView, setSurfaceView] = useState<'frontend' | 'native'>('frontend')
  const [draftProperty, setDraftProperty] = useState('backgroundColor')
  const [draftAfter, setDraftAfter] = useState('')
  useEffect(() => {
    if (model.platform !== 'tauri') setSurfaceView('frontend')
    else if (!model.pixelUrl && model.nativePixelUrl) setSurfaceView('native')
  }, [model.nativePixelUrl, model.pixelUrl, model.platform])
  const nativeViewport = model.surface?.nativeHost?.artifact
  const activeViewport = surfaceView === 'native' && nativeViewport?.width && nativeViewport.height
    ? { width: nativeViewport.width, height: nativeViewport.height }
    : viewport
  const activePixelUrl = surfaceView === 'native' ? model.nativePixelUrl : model.pixelUrl

  return (
    <div className={styles.workspace} style={{ display: active ? 'grid' : 'none' }}>
      <aside className={styles.navigator}>
        <header>
          <strong>多端设计目标</strong>
          <button type="button" title="重新发现项目目标" disabled={model.busy} onClick={() => void model.reload()}>
            <RefreshCw size={14} aria-hidden="true" />
          </button>
        </header>
        <div className={styles.platforms}>
          {PLATFORM_OPTIONS.map((option) => {
            const target = model.targets.find((item) => item.platform === option.id)
            const Icon = option.icon
            return (
              <button
                type="button"
                key={option.id}
                disabled={!target}
                className={model.platform === option.id ? styles.activePlatform : ''}
                title={target ? `${target.adapter} · ${target.evidenceLevel}` : '当前项目未发现此端入口'}
                onClick={() => model.selectPlatform(option.id)}
              >
                <Icon size={15} aria-hidden="true" />
                <span>{option.label}</span>
                <small>{target ? '已发现' : '未发现'}</small>
              </button>
            )
          })}
        </div>

        <section className={styles.listSection}>
          <div className={styles.sectionTitle}>
            <span>最近会话</span>
            <small>{model.sessions.length}</small>
          </div>
          <div className={styles.sessionList}>
            {model.sessions.length === 0 && <p>捕获页面后，会话会按项目保存在本机。</p>}
            {model.sessions.map((item) => (
              <button
                type="button"
                key={item.designSessionId}
                className={model.session?.designSessionId === item.designSessionId ? styles.activeItem : ''}
                onClick={() => void model.selectSession(item)}
              >
                <strong>{item.label}</strong>
                <span>{item.route}</span>
                <small>{item.hasEvidence ? '有证据' : item.state}</small>
              </button>
            ))}
          </div>
        </section>

        <section className={`${styles.listSection} ${styles.nodeSection}`}>
          <div className={styles.sectionTitle}>
            <span>语义 UI 树</span>
            <small>{model.surface?.surface?.nodeCount ?? nodes.length}</small>
          </div>
          <div className={styles.searchRow}>
            <input value={model.query} onChange={(event) => model.setQuery(event.currentTarget.value)} placeholder="selector / role / 文案" />
            <button type="button" disabled={!model.session || model.busy} title="查询 UI 树" onClick={() => void model.search()}>
              <Search size={14} aria-hidden="true" />
            </button>
          </div>
          <div className={styles.nodeList}>
            {nodes.map((node) => (
              <button
                type="button"
                key={node.id}
                className={model.selectedNode?.id === node.id ? styles.activeItem : ''}
                onClick={() => model.setSelectedNode(node)}
              >
                <strong>{node.label || node.role || node.tag}</strong>
                <span>{node.selector}</span>
              </button>
            ))}
          </div>
        </section>
      </aside>

      <main className={styles.stage}>
        <UiWorkspaceModeBar
          mode="headless"
          onModeChange={onModeChange}
          status={model.target
            ? `${model.target.label} · ${model.target.evidenceLevel}${model.target.nativeHostVerified ? ' · 原生宿主已验证' : ''}${taskFollow.active ? ` · 跟随任务 ${taskFollow.taskId}` : ''}`
            : '选择项目中已发现的设计目标'}
        />
        <div className={styles.routeBar}>
          <label>
            <span>路由</span>
            <input value={model.route} onChange={(event) => model.setRoute(event.currentTarget.value)} placeholder="/settings" />
          </label>
          {model.platform !== 'android' && (
            <label className={styles.urlField}>
              <span>本地 URL</span>
              <input value={model.url} onChange={(event) => model.setUrl(event.currentTarget.value)} placeholder="http://127.0.0.1:5173" />
            </label>
          )}
          <button type="button" disabled={!model.target || model.busy} onClick={() => void model.capture()}>
            <Camera size={15} aria-hidden="true" />
            {model.busy ? '正在读取…' : '后台捕获'}
          </button>
          <button
            type="button"
            disabled={!model.selectedNode?.interactive || model.busy}
            title="在一次性隔离后台页面中按 selector 重放点击；需要保留状态时使用下方持久浏览器"
            onClick={() => void model.interactSelected()}
          >
            <MousePointerClick size={15} aria-hidden="true" />
            后台点击
          </button>
        </div>

        <DesignRuntimeControls
          platform={model.platform}
          selectedNode={model.selectedNode}
          disabled={!model.session || model.busy}
          model={runtimeControls}
        />

        <DesignPlanningReview model={planningControls} hasDraft={Boolean(model.designDraft)} />

        {model.platform === 'tauri' && (
          <div className={styles.tauriBar}>
            <span>原生宿主</span>
            <button type="button" disabled={!model.session || model.busy} onClick={() => void model.prepareTauri()}>
              <Play size={14} aria-hidden="true" />
              {model.tauriRuntimeStatus === 'STARTING' ? '轮询窗口' : '启动 / 状态'}
            </button>
            <button
              type="button"
              disabled={!['READY', 'CAPTURED'].includes(model.tauriRuntimeStatus) || model.busy}
              onClick={() => void model.captureTauri()}
            >
              <Camera size={14} aria-hidden="true" />捕获原生
            </button>
            <button type="button" disabled={!model.tauriRuntimeStatus || model.busy} onClick={() => void model.stopTauri()}>
              <Square size={13} aria-hidden="true" />停止
            </button>
            <div className={styles.surfaceTabs}>
              <button type="button" className={surfaceView === 'frontend' ? styles.activeSurface : ''} disabled={!model.pixelUrl} onClick={() => setSurfaceView('frontend')}>WebView</button>
              <button type="button" className={surfaceView === 'native' ? styles.activeSurface : ''} disabled={!model.nativePixelUrl} onClick={() => setSurfaceView('native')}>原生窗口</button>
            </div>
          </div>
        )}

        <div className={styles.draftBar}>
          <span>可撤销草稿{model.designDraft ? ` · r${model.designDraft.revision}` : ''}</span>
          <input
            value={draftProperty}
            onChange={(event) => setDraftProperty(event.currentTarget.value)}
            placeholder="样式属性"
            aria-label="草稿样式属性"
          />
          <input
            value={draftAfter}
            onChange={(event) => setDraftAfter(event.currentTarget.value)}
            placeholder={model.selectedNode
              ? `新值（当前 ${(model.selectedNode.style as Record<string, string | undefined>)[draftProperty] || '未声明'}）`
              : '先选择语义节点'}
            aria-label="草稿样式新值"
          />
          <button type="button" disabled={!model.selectedNode || !draftAfter.trim() || model.busy} onClick={() => void model.saveDraftPatch(draftProperty, draftAfter)}>
            <Save size={13} aria-hidden="true" />保存草稿
          </button>
          <button type="button" disabled={!model.designDraft?.historyDepth || model.busy} onClick={() => void model.undoDraft()}>
            <RotateCcw size={13} aria-hidden="true" />撤销
          </button>
          <button
            type="button"
            disabled={planningControls.writebackPlan?.decision !== 'APPROVED'
              || planningControls.writebackPlan.draftRevision !== model.designDraft?.revision
              || !planningControls.bindingHealth?.readyForWriteback
              || model.busy}
            onClick={() => planningControls.writebackPlan
              && void model.beginDraftWriteback(planningControls.writebackPlan.planId)}
          >
            固定写回基线
          </button>
          <small>
            {model.writebackReceipt
              ? `回执 ${model.writebackReceipt.status}`
              : model.designDraft?.sourceBinding
                ? `绑定 ${model.designDraft.sourceBinding.status} · ${model.designDraft.sourceBinding.sourceFile}`
                : 'AI 建立 source binding 后才能写回'}
          </small>
        </div>

        <div className={styles.evidenceShell}>
          {activePixelUrl ? (
            <div
              className={styles.captureCanvas}
              style={{ aspectRatio: `${activeViewport.width} / ${activeViewport.height}` }}
              data-design-session-id={model.session?.designSessionId}
              data-design-surface={surfaceView}
            >
              <img src={activePixelUrl} alt={surfaceView === 'native' ? 'Tauri 原生窗口证据' : `${model.platform} ${model.route} 后台页面证据`} />
              {surfaceView === 'frontend' && nodes.map((node) => (
                <SemanticOverlay
                  key={node.id}
                  node={node}
                  viewport={viewport}
                  selected={model.selectedNode?.id === node.id}
                  onSelect={() => model.setSelectedNode(node)}
                />
              ))}
            </div>
          ) : (
            <div className={styles.emptyEvidence}>
              <MonitorCog size={30} aria-hidden="true" />
              <strong>{model.session ? '当前会话还没有像素证据' : '选择平台并输入本地 URL'}</strong>
              <span>后台捕获会生成 PNG、紧凑语义 UI 树和 SHA-256；不会把 Base64 塞进 AI 上下文。</span>
              {model.platform === 'android' && (
                <button type="button" onClick={() => onModeChange('evidence')}>切换 Android 真帧工作台</button>
              )}
            </div>
          )}
        </div>

        <footer className={styles.statusBar}>
          <span className={model.error || taskFollow.error ? styles.error : ''}>{model.error || taskFollow.error || (taskFollow.active ? `AI 工作中 · 跟随任务 ${taskFollow.taskId}` : model.status) || '等待后台设计会话'}</span>
          <span>{model.session?.designSessionId ?? '尚未打开 designSession'}</span>
          {model.surface?.pixels?.sha256 && <code>PNG {model.surface.pixels.sha256.slice(0, 12)}</code>}
          {model.surface?.uiTree?.sha256 && <code>UI {model.surface.uiTree.sha256.slice(0, 12)}</code>}
          {model.surface?.nativeHost?.artifact.sha256 && <code>Native {model.surface.nativeHost.artifact.sha256.slice(0, 12)}</code>}
        </footer>
      </main>

      <aside className={styles.conversation}>
        <UiTunerProjectSessionPanel
          pack={pack}
          intent={intent}
          conversationLayout="panel"
          defaultConversationOpen
          onMutationTaskStarted={() => undefined}
          onTaskSettled={() => { void model.reload() }}
          onTaskActivityChange={taskFollow.onTaskActivityChange}
          onDesignIntentPlan={planningControls.planIntent}
        />
      </aside>
    </div>
  )
}

function SemanticOverlay({
  node,
  viewport,
  selected,
  onSelect,
}: {
  node: SemanticUiNode
  viewport: { width: number; height: number }
  selected: boolean
  onSelect: () => void
}) {
  const left = percent(node.bounds.left, viewport.width)
  const top = percent(node.bounds.top, viewport.height)
  const width = percent(node.bounds.width, viewport.width)
  const height = percent(node.bounds.height, viewport.height)
  return (
    <button
      type="button"
      className={`${styles.semanticOverlay} ${selected ? styles.selectedOverlay : ''}`}
      style={{ left, top, width, height }}
      title={`${node.label || node.role} · ${node.selector}`}
      aria-label={`选择 ${node.label || node.selector}`}
      onClick={onSelect}
    />
  )
}

function percent(value: number, total: number) {
  return `${Math.max(0, Math.min(100, (value / Math.max(1, total)) * 100))}%`
}
