import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { Link } from 'react-router-dom'
import { ChevronDown, Search, Settings } from 'lucide-react'
import { useModelStore } from './useModelStore'
import { useAuthStore } from '../../store/auth'
import { providerGroupTitle, shortButtonLabel } from './modelUtils'
import { groupModelOptions } from './modelGroups'
import { ModelHoverPreview } from './ModelHoverPreview'
import {
  filterOptionsForRuntimeRoute,
  optionMatchesRuntimeRoute,
  routeModelEmptyState,
} from './routeModelPolicy'
import type { AgentOption } from './types'
import type { ModelOptionGroup } from './modelGroups'
import {
  ACTIVE_RUNTIME_ROUTE_GROUPS,
  DEFAULT_RUNTIME_ROUTE,
  runtimeRouteOption,
} from '../conversation/runtimeRoutes'
import type { RuntimeRoute } from '../conversation/runtimeRoutes'
import styles from './ModelPicker.module.css'

interface Props {
  /** 触发按钮的 ref，用于定位 popover */
  anchorRef: React.RefObject<HTMLElement | null>
  onClose: () => void
  runtimeRoute?: RuntimeRoute
  onRuntimeRouteChange?: (value: RuntimeRoute) => void
}

export function ModelPickerPopover({
  anchorRef,
  onClose,
  runtimeRoute,
  onRuntimeRouteChange,
}: Props) {
  const user = useAuthStore((s) => s.user)
  const {
    options,
    selectedAgent,
    label,
    codexCliOnly,
    localAiSummary,
    loading,
    error,
    load,
    saveSelection,
  } = useModelStore()
  const [saving, setSaving] = useState(false)
  const [saveError, setSaveError] = useState('')
  const [pos, setPos] = useState({ left: 12, bottom: 12, width: 360 })
  const [previewKey, setPreviewKey] = useState<string | null>(null)
  const [previewAnchorRect, setPreviewAnchorRect] = useState<DOMRect | null>(null)
  const [modelQuery, setModelQuery] = useState('')
  const [showDetectedModels, setShowDetectedModels] = useState(false)
  const previewCloseTimer = useRef<number | null>(null)
  const hasRuntimeRoutePicker = !!runtimeRoute && !!onRuntimeRouteChange
  const route = runtimeRoute ?? DEFAULT_RUNTIME_ROUTE
  const selectedRoute = runtimeRouteOption(route)

  // 定位 popover（锚点按钮上方）
  useEffect(() => {
    function reposition() {
      const el = anchorRef.current
      if (!el) return
      const rect = el.getBoundingClientRect()
      const targetWidth = hasRuntimeRoutePicker ? 680 : 430
      const width = Math.min(targetWidth, window.innerWidth - 24)
      const left = Math.max(12, Math.min(rect.left, window.innerWidth - width - 12))
      const estimatedHeight = hasRuntimeRoutePicker
        ? Math.min(window.innerWidth <= 720 ? 680 : 620, window.innerHeight - 24)
        : Math.min(520, window.innerHeight - 24)
      const maxBottom = Math.max(12, window.innerHeight - estimatedHeight - 12)
      const bottom = Math.max(12, Math.min(window.innerHeight - rect.top + 8, maxBottom))
      setPos({ left: Math.round(left), bottom: Math.round(bottom), width })
    }
    reposition()
    window.addEventListener('resize', reposition)
    return () => window.removeEventListener('resize', reposition)
  }, [anchorRef, hasRuntimeRoutePicker])

  useEffect(() => {
    return () => {
      if (previewCloseTimer.current) window.clearTimeout(previewCloseTimer.current)
    }
  }, [])

  useEffect(() => {
    setModelQuery('')
    setShowDetectedModels(false)
  }, [route])

  // 初次打开时加载
  useEffect(() => {
    if (user?.id && !loading) load(user.id)

  }, [user?.id])

  // Escape 关闭
  useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose() }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [onClose])

  async function handleSelect(option: AgentOption) {
    if (!user?.id) return
    if (option.selectable === false) return
    if (hasRuntimeRoutePicker && !optionMatchesRuntimeRoute(option, route)) return
    setSaving(true)
    setSaveError('')
    try {
      await saveSelection(option, user.id)
      onClose()
    } catch (err) {
      setSaveError((err as { message?: string }).message ?? '切换失败')
    } finally {
      setSaving(false)
    }
  }

  function cancelPreviewClose() {
    if (previewCloseTimer.current) {
      window.clearTimeout(previewCloseTimer.current)
      previewCloseTimer.current = null
    }
  }

  function schedulePreviewClose() {
    cancelPreviewClose()
    previewCloseTimer.current = window.setTimeout(() => {
      setPreviewKey(null)
      setPreviewAnchorRect(null)
      previewCloseTimer.current = null
    }, 180)
  }

  function showPreview(group: ModelOptionGroup, element: HTMLElement) {
    cancelPreviewClose()
    setPreviewKey(group.key)
    setPreviewAnchorRect(element.getBoundingClientRect())
  }

  const visibleOptions = hasRuntimeRoutePicker
    ? filterOptionsForRuntimeRoute(options, route)
    : options
  const emptyState = hasRuntimeRoutePicker ? routeModelEmptyState(route) : null
  const modelGroups = groupModelOptions(visibleOptions, selectedAgent)
  const previewGroup =
    previewKey ? modelGroups.find((group) => group.key === previewKey) ?? null : null

  const query = modelQuery.trim().toLowerCase()
  const matchesQuery = (group: ModelOptionGroup) => {
    if (!query) return true
    return `${group.label} ${group.subtitle} ${providerGroupTitle(group.provider)}`
      .toLowerCase()
      .includes(query)
  }
  const availableModelGroups = modelGroups.filter((group) =>
    group.options.some((option) => option.selectable !== false),
  )
  const detectedModelGroups = modelGroups.filter((group) =>
    group.options.every((option) => option.selectable === false),
  )
  const displayedModelGroups = (showDetectedModels
    ? modelGroups
    : availableModelGroups
  ).filter(matchesQuery)
  const selectedModelGroup = modelGroups.find((group) => group.selectedOption)

  // 按 provider 分组
  const providerGroups = new Map<string, ModelOptionGroup[]>()
  for (const group of displayedModelGroups) {
    const title = providerGroupTitle(group.provider)
    if (!providerGroups.has(title)) providerGroups.set(title, [])
    providerGroups.get(title)!.push(group)
  }

  const popover = (
    <>
      <button
        className={styles.backdrop}
        type="button"
        aria-label="关闭模型选择"
        onClick={onClose}
      />
      <section
        className={[
          styles.popover,
          hasRuntimeRoutePicker ? styles.routePopover : '',
        ].join(' ')}
        role="dialog"
        aria-label="选择 AI 模型"
        style={{ left: pos.left, bottom: pos.bottom, width: pos.width }}
      >
        <header className={styles.header}>
          <div>
            <strong>{hasRuntimeRoutePicker ? '选择 AI 来源和模型' : '选择 AI 模型'}</strong>
            <span>{hasRuntimeRoutePicker ? selectedRoute.title : label || '服务器默认'}</span>
          </div>
          <button className={styles.closeBtn} type="button" onClick={onClose} aria-label="关闭">
            ×
          </button>
        </header>

        <div className={hasRuntimeRoutePicker ? styles.pickerBody : styles.singleBody}>
          {hasRuntimeRoutePicker && (
            <aside className={styles.routePane} aria-label="用哪个 AI">
              <div className={styles.paneIntro}>
                <strong>AI 来源</strong>
                <span>先决定由哪台电脑或平台提供 AI</span>
              </div>
              {ACTIVE_RUNTIME_ROUTE_GROUPS.map((group) => (
                <section className={styles.routeGroup} key={group.title}>
                  <div className={styles.routeGroupTitle}>
                    <strong>{group.title}</strong>
                    <span>{group.description}</span>
                  </div>
                  {group.options.map((item) => (
                    <div
                      className={[
                        styles.routeOption,
                        item.value === route ? styles.routeOptionActive : '',
                      ].join(' ')}
                      key={item.value}
                    >
                      <button
                        className={styles.routeOptionSelect}
                        type="button"
                        onClick={() => {
                          setPreviewKey(null)
                          setPreviewAnchorRect(null)
                          onRuntimeRouteChange?.(item.value)
                        }}
                        aria-pressed={item.value === route}
                      >
                        <span className={styles.routeCode}>{item.code}</span>
                        <span className={styles.routeCopy}>
                          <strong>{item.title}</strong>
                          <span>{item.subtitle}</span>
                        </span>
                      </button>
                      {item.configHref && (
                        <Link
                          className={styles.routeConfigLink}
                          to={item.configHref}
                          title={item.configLabel ?? `${item.title}配置`}
                          aria-label={item.configLabel ?? `${item.title}配置`}
                          onClick={onClose}
                        >
                          <Settings size={15} strokeWidth={2.2} aria-hidden="true" />
                        </Link>
                      )}
                    </div>
                  ))}
                </section>
              ))}
            </aside>
          )}

          <div className={styles.modelPane}>
            <div className={styles.modelPaneHeader}>
              <div>
                <span className={styles.paneEyebrow}>当前来源 · {selectedRoute.shortLabel}</span>
                <strong>{selectedModelGroup?.label ?? (route === 'auto' ? '自动选择模型' : '选择模型')}</strong>
              </div>
              <span className={styles.modelCount}>{availableModelGroups.length} 个可用</span>
            </div>
            <p className={styles.routeDescription}>{selectedRoute.description}</p>
            <label className={styles.searchBox}>
              <Search size={14} aria-hidden="true" />
              <input
                value={modelQuery}
                onChange={(event) => setModelQuery(event.target.value)}
                placeholder="搜索模型或服务商"
                aria-label="搜索模型或服务商"
              />
              {modelQuery && (
                <button
                  type="button"
                  className={styles.clearSearch}
                  onClick={() => setModelQuery('')}
                  aria-label="清除模型搜索"
                >
                  ×
                </button>
              )}
            </label>
            <div className={styles.list} onScroll={schedulePreviewClose}>
              {loading && <p className={styles.empty}>正在读取模型列表…</p>}
              {!loading && (error || saveError) && (
                <p className={styles.errorMsg}>{saveError || error}</p>
              )}
              {!loading && !error && visibleOptions.length === 0 && (
                <div className={styles.emptyCard}>
                  <strong>{emptyState?.title ?? '当前没有可选模型'}</strong>
                  <span>
                    {emptyState?.body ?? '请检查服务器 agent 配置或 PC 节点 CLI 配置。'}
                  </span>
                  {emptyState?.actionHref && (
                    <Link className={styles.emptyAction} to={emptyState.actionHref} onClick={onClose}>
                      <Settings size={14} strokeWidth={2.2} aria-hidden="true" />
                      {emptyState.actionLabel ?? '去配置'}
                    </Link>
                  )}
                </div>
              )}
              {!loading && !error && query && visibleOptions.length > 0 && displayedModelGroups.length === 0 && (
                <div className={styles.emptyCard}>
                  <strong>没有匹配的模型</strong>
                  <span>换一个关键词，或清除搜索条件。</span>
                </div>
              )}
              {!loading &&
                Array.from(providerGroups.entries()).map(([title, groupedOptions]) => (
                  <div key={title}>
                    <div className={styles.section}>
                      <span>{title}</span>
                      <span>{groupedOptions.length} 个</span>
                    </div>
                    {groupedOptions.map((group) => (
                      <button
                        key={group.key}
                        className={[
                          styles.option,
                          group.primaryOption.selectable === false ? styles.optionDisabled : '',
                          group.selectedOption ? styles.active : '',
                        ].join(' ')}
                        type="button"
                        disabled={saving}
                        aria-disabled={group.primaryOption.selectable === false}
                        onMouseEnter={(event) => showPreview(group, event.currentTarget)}
                        onMouseLeave={schedulePreviewClose}
                        onFocus={(event) => showPreview(group, event.currentTarget)}
                        onClick={() => handleSelect(group.primaryOption)}
                      >
                        <span>
                          <strong>{group.label}</strong>
                          {group.subtitle && <span>{group.subtitle}</span>}
                        </span>
                        <span className={styles.check}>
                          {group.primaryOption.selectable === false
                            ? '探测'
                            : group.selectedOption
                              ? '✓'
                              : group.options.length > 1
                                ? '›'
                              : ''}
                        </span>
                      </button>
                    ))}
                  </div>
                ))}
              {!loading && !query && detectedModelGroups.length > 0 && (
                <button
                  type="button"
                  className={styles.detectedToggle}
                  onClick={() => setShowDetectedModels((current) => !current)}
                  aria-expanded={showDetectedModels}
                >
                  <span>
                    已检测到 {detectedModelGroups.length} 个本机模型，但暂不可直接使用
                  </span>
                  <ChevronDown
                    size={15}
                    className={showDetectedModels ? styles.chevronOpen : ''}
                    aria-hidden="true"
                  />
                </button>
              )}
            </div>
          </div>
        </div>

        <footer className={styles.footer}>
          {hasRuntimeRoutePicker && localAiSummary && (
            <span className={styles.footerStatus}>{localAiSummary}</span>
          )}
          <button type="button" disabled={saving} onClick={() => user?.id && load(user.id)}>
            刷新
          </button>
          {!codexCliOnly && (
            <button type="button" onClick={() => window.open('/web', '_blank')}>
              完整模型设置
            </button>
          )}
        </footer>
      </section>
      <ModelHoverPreview
        group={previewGroup}
        anchorRect={previewAnchorRect}
        selectedAgent={selectedAgent}
        saving={saving}
        routeTitle={hasRuntimeRoutePicker ? selectedRoute.title : undefined}
        onSelect={handleSelect}
        onMouseEnter={cancelPreviewClose}
        onMouseLeave={schedulePreviewClose}
      />
    </>
  )

  return createPortal(popover, document.body)
}

/** 触发模型选择器的按钮，嵌入侧边栏或工具栏 */
export function ModelPickerButton({ compact }: { compact?: boolean }) {
  const label = useModelStore((s) => s.label)
  const [open, setOpen] = useState(false)
  const btnRef = useRef<HTMLButtonElement>(null)

  const shortLabel = shortButtonLabel(label)

  if (compact) {
    return (
      <>
        <button
          ref={btnRef}
          style={{
            width: 48, height: 48, borderRadius: '50%',
            background: '#1e2026', border: '1px solid #3b3e46',
            color: '#c5c8d0', fontSize: 11, fontWeight: 700,
            cursor: 'pointer', transition: 'background 0.14s',
            display: 'grid', placeItems: 'center', lineHeight: 1.2,
            textAlign: 'center', padding: '2px',
          }}
          title={`AI 模型：${label || '服务器默认'}`}
          onClick={() => setOpen((v) => !v)}
          type="button"
        >
          {shortLabel.length > 5 ? shortLabel.slice(0, 5) : shortLabel}
        </button>
        {open && <ModelPickerPopover anchorRef={btnRef} onClose={() => setOpen(false)} />}
      </>
    )
  }

  return (
    <>
      <button
        ref={btnRef}
        className={styles.triggerBtn}
        type="button"
        title={`AI 模型：${label || '服务器默认'}`}
        onClick={() => setOpen((v) => !v)}
      >
        <span className={styles.triggerIcon}>🧠</span>
        <span className={styles.triggerLabel}>{shortLabel}</span>
      </button>
      {open && (
        <ModelPickerPopover
          anchorRef={btnRef}
          onClose={() => setOpen(false)}
        />
      )}
    </>
  )
}
