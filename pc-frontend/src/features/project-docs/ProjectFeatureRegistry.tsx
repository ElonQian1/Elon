import { useCallback, useEffect, useMemo, useState } from 'react'
import { Bot, FileText, ListTodo, RefreshCw, Search, ShieldAlert } from 'lucide-react'

import type { DocumentOrganizationTrackingRuntime } from './projectDocumentOrganizationStatus'
import {
  listAllProjectFeatures,
  type ProjectFeatureSnapshot,
  type ProjectFeatureStatus,
} from './projectFeatureRegistryModel'
import styles from './ProjectFeatureRegistry.module.css'

interface Props {
  runtime: DocumentOrganizationTrackingRuntime
  canStartAi: boolean
  onRunAi: (instruction: string) => void
  onOpenDocument: (path: string) => void
  onTotalChange?: (total: number) => void
}

const ACTIVE_STATUSES: ProjectFeatureStatus[] = [
  'draft', 'proposed', 'accepted', 'ready', 'claimed', 'in_progress', 'blocked', 'implemented',
]

export default function ProjectFeatureRegistry({
  runtime,
  canStartAi,
  onRunAi,
  onOpenDocument,
  onTotalChange,
}: Props) {
  const [features, setFeatures] = useState<ProjectFeatureSnapshot[]>([])
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const load = useCallback(async () => {
    if (!runtime.enabled || !runtime.adminUrl || !runtime.projectRoot) return
    setLoading(true)
    setError('')
    try {
      const page = await listAllProjectFeatures({
        adminUrl: runtime.adminUrl,
        projectRoot: runtime.projectRoot,
        statuses: ACTIVE_STATUSES,
      })
      setFeatures(page.features)
      onTotalChange?.(page.total)
    } catch (reason) {
      setError(errorMessage(reason, '读取功能登记失败'))
    } finally {
      setLoading(false)
    }
  }, [onTotalChange, runtime.adminUrl, runtime.enabled, runtime.projectRoot])

  useEffect(() => { void load() }, [load])

  const visible = useMemo(() => {
    const normalized = query.trim().toLowerCase()
    if (!normalized) return features
    return features.filter((feature) => [
      feature.id,
      feature.title,
      feature.summary,
      feature.requirement_path,
      ...feature.tags,
      ...feature.task_paths,
    ].some((value) => value.toLowerCase().includes(normalized)))
  }, [features, query])

  if (!runtime.enabled || !runtime.adminUrl || !runtime.projectRoot) {
    return (
      <section className={styles.unavailable}>
        <ListTodo size={28} />
        <h2>功能需求登记</h2>
        <p>当前项目没有连接本机节点。需求 Markdown 仍可编辑，但代理任务状态、认领和证据漂移需要本机 Feature Registry。</p>
      </section>
    )
  }

  const actionable = features.filter((feature) => ['accepted', 'ready', 'blocked'].includes(feature.status)).length
  const drifted = features.filter(isFeatureDrifted).length
  return (
    <section className={styles.workspace}>
      <header className={styles.header}>
        <div>
          <ListTodo size={22} aria-hidden="true" />
          <span><strong>功能需求登记</strong><small>Git 真源 · 需求正文不复制</small></span>
        </div>
        <button type="button" onClick={() => { void load() }} disabled={loading} title="刷新功能登记">
          <RefreshCw size={16} className={loading ? styles.spinning : ''} />
        </button>
      </header>

      <div className={styles.summary}>
        <article><span>活动功能</span><strong>{features.length}</strong></article>
        <article><span>可推进</span><strong>{actionable}</strong></article>
        <article data-warning={drifted > 0}><span>信源漂移</span><strong>{drifted}</strong></article>
      </div>

      <div className={styles.toolbar}>
        <label><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索功能、需求路径或实现范围" /></label>
        <button type="button" disabled={!canStartAi} onClick={() => onRunAi(registerInstruction())}>
          <Bot size={15} />让 AI 登记新需求
        </button>
      </div>

      <p className={styles.notice}>新需求先由 Codex 使用原生编辑工具写入正式 Markdown，再通过 <code>project_feature_workflow</code> 的 register action 登记。注册表只提供工作流导航；源码、测试和当前约束文档始终拥有更高优先级。</p>
      {error && <div className={styles.error}>{error}</div>}
      {!loading && !visible.length && <div className={styles.empty}>当前没有命中的活动功能。可以让 AI 创建需求文档并登记。</div>}

      <div className={styles.list}>
        {visible.map((feature) => (
          <article className={styles.card} key={feature.id} data-drift={isFeatureDrifted(feature)}>
            <header>
              <div><span data-priority={feature.priority}>{feature.priority.toUpperCase()}</span><strong>{feature.title}</strong></div>
              <i data-status={feature.status}>{statusLabel(feature.status)}</i>
            </header>
            <p>{feature.summary}</p>
            <dl>
              <div><dt>需求</dt><dd><code>{feature.requirement_path}</code></dd></div>
              <div><dt>验收</dt><dd>{feature.acceptance_criteria_count} 项</dd></div>
              <div><dt>证据</dt><dd>{feature.implementation_evidence_count} 条{feature.implementation_evidence_checked ? '' : '（按需校验）'}</dd></div>
              <div><dt>负责人</dt><dd>{feature.owner || feature.claim?.agent_id || '未指定'}</dd></div>
            </dl>
            {!!feature.dependency_blockers.length && <p className={styles.warning}>依赖未完成：{feature.dependency_blockers.join('、')}</p>}
            {isFeatureDrifted(feature) && (
              <p className={styles.warning}><ShieldAlert size={14} />{driftLabel(feature.drift_status)}；代理不得从旧登记直接实现。</p>
            )}
            <footer>
              <button type="button" onClick={() => onOpenDocument(feature.requirement_path)}><FileText size={14} />打开需求</button>
              <button type="button" disabled={!canStartAi} onClick={() => onRunAi(workInstruction(feature))}>
                <Bot size={14} />{workLabel(feature)}
              </button>
            </footer>
          </article>
        ))}
      </div>
    </section>
  )
}

function registerInstruction() {
  return '请登记一个新的正式功能需求。这是显式功能生命周期任务：先调用 project_feature_workflow 的 describe action 按需取得字段合同。再用原生工作区编辑工具创建或完善一份 version_status=current 的 Markdown 需求文档，写清目标、非目标、验收标准、依赖和预计实现范围；不要放在 drafts、inbox、history、archive 或 discussions。随后用 project_feature_workflow 的 list action 获取 registry revision，再用 register action 登记稳定 feature id、优先级、需求路径、任务路径、验收标准和 actor。不要复制需求正文到注册表，不要自动开始实现；最后回报需求路径、feature id 和登记状态。'
}

function workInstruction(feature: ProjectFeatureSnapshot) {
  if (isFeatureDrifted(feature)) {
    return `请修复已登记功能 ${feature.id}（${feature.title}）的信源漂移。这是显式功能生命周期任务：字段不明确先调用 project_feature_workflow 的 describe action；再用 plan 和 check_drift action，只用原生工具核对 ${feature.requirement_path}、当前源码与测试，判断变化是否有意。若需求变化应保留，使用最新 registry revision 调用 rebind_requirement action 并说明原因；该操作必须回退到 proposed，不能自动重新接受或继续实现。若变化不是有意，不要重绑旧事实，停止并报告需要恢复的文件。`
  }
  if (feature.status === 'draft' || feature.status === 'proposed') {
    return `请完善已登记功能 ${feature.id}（${feature.title}）。这是显式功能生命周期任务：字段不明确先调用 project_feature_workflow 的 describe action；再用 plan action，并用原生工具读取 ${feature.requirement_path}，核对目标、非目标、依赖、任务路径和可验证验收标准。需要调整登记时使用最新 registry revision 调用 update action；只有需求和验收标准已明确时才用 transition action 按状态机推进 proposed -> accepted -> ready。不要开始实现，也不要复制需求正文到注册表。`
  }
  return `请处理已登记功能 ${feature.id}（${feature.title}）。这是显式功能生命周期任务：字段不明确先调用 project_feature_workflow 的 describe action；先用 plan action 核对需求哈希、依赖、验收标准和当前认领，再用原生工具读取 ${feature.requirement_path} 与当前源码/测试。若状态 ready 且依赖完成，使用最新 registry revision 调用 claim action；随后用 transition action 按状态机推进、实现，并通过 record_evidence action 绑定真实文件/测试证据。信源冲突时以当前源码测试和 current 约束文档为准，停止并报告漂移，不要绕过 claim、revision 或 verified 证据要求。`
}

function statusLabel(status: ProjectFeatureStatus) {
  return ({
    draft: '草稿', proposed: '待评审', accepted: '已接受', ready: '可认领', claimed: '已认领',
    in_progress: '开发中', blocked: '阻塞', implemented: '已实现待验收', verified: '已验证',
    released: '已发布', retired: '已退役',
  } as Record<ProjectFeatureStatus, string>)[status]
}

function workLabel(feature: ProjectFeatureSnapshot) {
  if (isFeatureDrifted(feature)) return '让 AI 处理漂移'
  if (feature.status === 'draft' || feature.status === 'proposed') return '让 AI 完善'
  if (feature.status === 'implemented') return '让 AI 验收'
  if (feature.status === 'blocked') return '让 AI 排障'
  return '让 AI 处理'
}

function driftLabel(status: ProjectFeatureSnapshot['drift_status']) {
  return status === 'requirement_drifted' ? '需求文档已变化' : '实现证据已变化'
}

function isFeatureDrifted(feature: ProjectFeatureSnapshot) {
  return feature.drift_status === 'requirement_drifted'
    || feature.drift_status === 'implementation_evidence_drifted'
}

function errorMessage(reason: unknown, fallback: string) {
  return reason instanceof Error && reason.message ? reason.message : fallback
}
