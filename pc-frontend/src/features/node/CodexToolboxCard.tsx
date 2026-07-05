import { Wrench } from 'lucide-react'
import type { CodexToolboxStatus, CodexToolboxTool } from './types'
import styles from './NodePage.module.css'

interface Props {
  toolbox?: CodexToolboxStatus | null
  busy: boolean
  onRepair: () => void
}

export default function CodexToolboxCard({ toolbox, busy, onRepair }: Props) {
  const tools = toolbox?.tools ?? []
  const rg = tools.find((tool) => tool.id === 'rg')
  const needsRepair = rg?.status !== 'ready' && rg?.repair_action === 'install_env_codex'
  return (
    <section className={styles.toolboxCard}>
      <div className={styles.toolboxHead}>
        <div>
          <span className={styles.codexLabel}>Codex 工具箱</span>
          <h4>{toolboxTitle(rg)}</h4>
        </div>
        <span className={toolboxBadgeClass(rg)}>{toolboxBadgeText(rg)}</span>
      </div>
      <p>{toolbox?.summary || 'Win 端会把已存在的小工具临时注入 Codex CLI 子进程。'}</p>
      {toolbox?.codex_program && <code className={styles.toolboxPath}>{toolbox.codex_program}</code>}
      <div className={styles.toolboxList}>
        {tools.map((tool) => <ToolRow key={tool.id || tool.name} tool={tool} />)}
      </div>
      <div className={styles.codexActions}>
        <button className={[styles.btn, needsRepair ? styles.primary : ''].join(' ')} onClick={onRepair} disabled={busy}>
          <Wrench size={14} strokeWidth={2.2} aria-hidden="true" />
          {needsRepair ? '修复 rg / Codex 环境' : '检查/修复工具箱'}
        </button>
      </div>
    </section>
  )
}

function ToolRow({ tool }: { tool: CodexToolboxTool }) {
  return (
    <div className={[styles.toolboxTool, styles[`toolboxTool_${toolTone(tool)}`]].join(' ')}>
      <div>
        <strong>{tool.id || tool.name}</strong>
        <span>{toolLine(tool)}</span>
      </div>
      <small>{sourceLabel(tool.source)}</small>
      {tool.path && <code>{tool.path}</code>}
    </div>
  )
}

function toolboxTitle(rg?: CodexToolboxTool) {
  if (!rg) return '等待工具状态'
  if (rg.status === 'ready') return 'rg 已可用于 Codex CLI'
  if (rg.status === 'not_runnable') return 'rg 路径异常'
  return 'rg 未安装'
}

function toolboxBadgeText(rg?: CodexToolboxTool) {
  if (!rg) return '检测中'
  if (rg.status === 'ready') return '已加速'
  if (rg.status === 'missing') return '可修复'
  return '需处理'
}

function toolboxBadgeClass(rg?: CodexToolboxTool) {
  const tone = !rg ? 'checking' : rg.status === 'ready' ? 'online' : rg.status === 'missing' ? 'checking' : 'offline'
  return [styles.toolboxState, styles[tone]].join(' ')
}

function toolTone(tool: CodexToolboxTool) {
  if (tool.status === 'ready') return 'ready'
  if (tool.status === 'missing') return 'missing'
  return 'warn'
}

function toolLine(tool: CodexToolboxTool) {
  const parts = [
    tierLabel(tool.tier),
    policyLabel(tool.install_policy),
    tool.will_inject ? '会注入 PATH' : '不会注入',
    tool.version || tool.reason || tool.detail || '',
  ].filter(Boolean)
  return parts.join(' · ')
}

function tierLabel(value?: string) {
  if (value === 'core') return '核心'
  if (value === 'profile') return '按需'
  if (value === 'optional') return '可选'
  return value || ''
}

function policyLabel(value?: string) {
  if (value === 'AutoSmall') return '可自动修复'
  if (value === 'ManualRepair') return '手动安装'
  if (value === 'NeverAuto') return '不自动安装'
  return value || ''
}

function sourceLabel(value?: string) {
  if (value === 'elon_managed') return '一龙绿色目录'
  if (value === 'codex_desktop') return 'Codex Desktop'
  if (value === 'cargo') return 'Cargo'
  if (value === 'scoop') return 'Scoop'
  if (value === 'chocolatey') return 'Chocolatey'
  if (value === 'program_files') return 'Program Files'
  if (value === 'missing') return '未找到'
  return 'PATH'
}
