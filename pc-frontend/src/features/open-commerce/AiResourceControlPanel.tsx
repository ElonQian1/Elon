import { useCallback, useEffect, useState } from 'react'
import {
  ArrowDown,
  ArrowUp,
  Play,
  RefreshCw,
  Save,
} from 'lucide-react'
import { aiResourceApi } from './aiResourceApi'
import type {
  AiResourceClass,
  AiResourceOverview,
  AiResourcePolicy,
  AiRoutePreview,
} from './aiResourceTypes'
import { errorText, formatMicros } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  listItemStyle,
} from './openCommerceStyles'

const resourceLabels: Record<AiResourceClass, string> = {
  own_codex: '自己的 Codex',
  remote_node: '本人 PC 节点',
  shared_codex: '已授权共享 Codex',
  platform_model: '平台模型',
}

export default function AiResourceControlPanel({
  projectId,
  canEdit,
}: {
  projectId: string
  canEdit: boolean
}) {
  const [overview, setOverview] = useState<AiResourceOverview | null>(null)
  const [policy, setPolicy] = useState<AiResourcePolicy | null>(null)
  const [taskKind, setTaskKind] = useState('code')
  const [preferredModel, setPreferredModel] = useState('')
  const [requireLocal, setRequireLocal] = useState(false)
  const [preview, setPreview] = useState<AiRoutePreview | null>(null)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    setMessage('')
    try {
      const response = await aiResourceApi.overview(projectId)
      setOverview(response)
      setPolicy(response.policy)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function savePolicy() {
    if (!policy) return
    setBusy(true)
    setMessage('')
    try {
      const saved = await aiResourceApi.updatePolicy(projectId, {
        enabled_classes: policy.enabled_classes,
        priority: policy.priority,
        allow_fallback: policy.allow_fallback,
        privacy_mode: policy.privacy_mode,
        max_estimated_unit_cost_micros: policy.max_estimated_unit_cost_micros,
      })
      setPolicy(saved)
      setMessage('项目 AI 资源策略已保存。')
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function runPreview() {
    setBusy(true)
    setMessage('')
    try {
      setPreview(await aiResourceApi.preview(projectId, {
        task_kind: taskKind,
        preferred_model: preferredModel.trim() || undefined,
        require_local_execution: requireLocal,
      }))
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  function toggleClass(resourceClass: AiResourceClass) {
    if (!policy) return
    const enabled = policy.enabled_classes.includes(resourceClass)
    const nextEnabled = enabled
      ? policy.enabled_classes.filter((item) => item !== resourceClass)
      : [...policy.enabled_classes, resourceClass]
    const nextPriority = enabled
      ? policy.priority.filter((item) => item !== resourceClass)
      : [...policy.priority, resourceClass]
    setPolicy({ ...policy, enabled_classes: nextEnabled, priority: nextPriority })
  }

  function moveClass(index: number, direction: -1 | 1) {
    if (!policy) return
    const target = index + direction
    if (target < 0 || target >= policy.priority.length) return
    const priority = [...policy.priority]
    ;[priority[index], priority[target]] = [priority[target], priority[index]]
    setPolicy({ ...policy, priority })
  }

  const models = Array.from(new Set(
    overview?.resources.map((resource) => resource.model).filter(Boolean) as string[] ?? [],
  ))

  return (
    <div className={base.panel}>
      <header className={base.hero} style={commerceStyles.workspaceHeader}>
        <div>
          <h2>共享 AI 资源控制面</h2>
          <p>统一盘点已有资源并预演路由；凭据、第三方余额和真实执行仍由原有安全边界管理。</p>
        </div>
        <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新资源">
          <RefreshCw size={15} />
        </button>
      </header>

      <section className={base.stats}>
        {Object.entries(resourceLabels).map(([resourceClass, label]) => (
          <div key={resourceClass}>
            <span>{label}</span>
            <strong>{overview?.resources.filter((resource) => resource.resource_class === resourceClass).length ?? 0}</strong>
            <small>额度状态单独核验</small>
          </div>
        ))}
      </section>

      <div style={commerceStyles.grid}>
        <section className={base.integrationSection}>
          <header><strong>项目策略</strong><span style={badgeStyle()}>CONTROL PLANE</span></header>
          <div className={base.formCard} style={commerceStyles.sectionBody}>
            <div style={commerceStyles.list}>
              {policy?.priority.map((resourceClass, index) => (
                <div style={commerceStyles.priorityRow} key={resourceClass}>
                  <span style={commerceStyles.priorityIndex}>{index + 1}</span>
                  <label style={commerceStyles.checkRow}>
                    <input type="checkbox" checked onChange={() => toggleClass(resourceClass)} disabled={!canEdit} />
                    {resourceLabels[resourceClass]}
                  </label>
                  <div style={commerceStyles.headerActions}>
                    <button style={actionStyle('icon', !canEdit || index === 0)} type="button" onClick={() => moveClass(index, -1)} disabled={!canEdit || index === 0} title="提高优先级"><ArrowUp size={13} /></button>
                    <button style={actionStyle('icon', !canEdit || index === policy.priority.length - 1)} type="button" onClick={() => moveClass(index, 1)} disabled={!canEdit || index === policy.priority.length - 1} title="降低优先级"><ArrowDown size={13} /></button>
                  </div>
                </div>
              ))}
              {(Object.keys(resourceLabels) as AiResourceClass[])
                .filter((resourceClass) => !policy?.enabled_classes.includes(resourceClass))
                .map((resourceClass) => (
                  <label style={commerceStyles.checkRow} key={resourceClass}>
                    <input type="checkbox" checked={false} onChange={() => toggleClass(resourceClass)} disabled={!canEdit} />
                    {resourceLabels[resourceClass]}
                  </label>
                ))}
            </div>
            <label>
              路由倾向
              <select value={policy?.privacy_mode ?? 'prefer_local'} onChange={(event) => policy && setPolicy({ ...policy, privacy_mode: event.target.value as AiResourcePolicy['privacy_mode'] })} disabled={!canEdit}>
                <option value="prefer_local">本地优先</option>
                <option value="balanced">按资源优先级</option>
                <option value="prefer_available">就绪资源优先</option>
              </select>
            </label>
            <label style={commerceStyles.checkRow}>
              <input type="checkbox" checked={policy?.allow_fallback ?? false} onChange={(event) => policy && setPolicy({ ...policy, allow_fallback: event.target.checked })} disabled={!canEdit} />
              主资源不可用时允许候选回退
            </label>
            <label>
              已知单位成本上限（micros）
              <input type="number" min="0" value={policy?.max_estimated_unit_cost_micros ?? ''} onChange={(event) => policy && setPolicy({ ...policy, max_estimated_unit_cost_micros: event.target.value ? Number(event.target.value) : undefined })} disabled={!canEdit} />
            </label>
            <button style={actionStyle('primary', !canEdit || busy || !policy)} type="button" onClick={savePolicy} disabled={!canEdit || busy || !policy}><Save size={13} />保存策略</button>
          </div>
        </section>

        <section className={base.integrationSection}>
          <header><strong>真实资源清单</strong><span style={badgeStyle()}>{overview?.resources.length ?? 0}</span></header>
          <div className={base.formCard} style={{ ...commerceStyles.sectionBody, ...commerceStyles.scrollArea }}>
            {overview?.resources.map((resource) => (
              <article className={base.formCard} style={listItemStyle()} key={resource.resource_id}>
                <header style={commerceStyles.itemHeader}><h3 style={commerceStyles.itemTitle}>{resource.label}</h3><span style={badgeStyle(resource.availability.includes('unverified') ? 'warn' : 'neutral')}>{resource.availability}</span></header>
                <p style={commerceStyles.itemText}>{resourceLabels[resource.resource_class]} · {resource.execution_scope} · {resource.cost_basis}</p>
                <small style={commerceStyles.itemMeta}>{resource.model ?? resource.provider} · {resource.estimated_unit_cost_micros === undefined ? '成本未知' : formatMicros(resource.estimated_unit_cost_micros)}</small>
                <code style={commerceStyles.itemMeta}>{resource.evidence.join(' · ')}</code>
              </article>
            ))}
            {overview?.resources.length === 0 && <p className={base.empty}>当前用户没有可见的 AI 资源。</p>}
          </div>
        </section>
      </div>

      <section className={base.integrationSection}>
        <header><strong>路由预演</strong><span style={badgeStyle('warn')}>不会执行任务</span></header>
        <div className={base.formCard} style={commerceStyles.sectionBody}>
          <div style={commerceStyles.grid}>
            <label>任务类型<select value={taskKind} onChange={(event) => setTaskKind(event.target.value)}><option value="code">代码</option><option value="chat">对话</option><option value="analysis">分析</option><option value="image">图像</option></select></label>
            <label>指定模型<select value={preferredModel} onChange={(event) => setPreferredModel(event.target.value)}><option value="">不限模型</option>{models.map((model) => <option key={model} value={model}>{model}</option>)}</select></label>
            <label style={commerceStyles.checkRow}><input type="checkbox" checked={requireLocal} onChange={(event) => setRequireLocal(event.target.checked)} />仅本人节点本地执行</label>
          </div>
          <button style={actionStyle('secondary', busy)} type="button" onClick={runPreview} disabled={busy}><Play size={13} />预演路由</button>
          {preview && <RoutePreview preview={preview} />}
        </div>
      </section>

      {message && <div style={commerceStyles.message}>{message}</div>}
    </div>
  )
}

function RoutePreview({ preview }: { preview: AiRoutePreview }) {
  return (
    <div style={commerceStyles.list}>
      <article className={base.formCard} style={listItemStyle(Boolean(preview.selected))} data-selected={Boolean(preview.selected)}>
        <header style={commerceStyles.itemHeader}><h3 style={commerceStyles.itemTitle}>{preview.selected?.label ?? '无匹配资源'}</h3><span style={badgeStyle('warn')}>未执行</span></header>
        <p style={commerceStyles.itemText}>{preview.reasons.join(' · ')}</p>
        <small style={commerceStyles.itemMeta}>外部额度未核验 · 候选回退 {preview.fallbacks.length} 个</small>
      </article>
    </div>
  )
}
