import { HelpCircle, Wrench } from 'lucide-react'
import type { CodexToolboxStatus, CodexToolboxTool, LocalCliToolStatus } from './types'
import styles from './NodePage.module.css'

interface Props {
  toolbox?: CodexToolboxStatus | null
  codex?: LocalCliToolStatus | null
  busy: boolean
  onRepair: () => void
}

const TOOLBOX_HELP_TEXT = 'Codex 工具箱是本机节点给 Codex CLI 准备的辅助命令。rg 是核心搜索工具，已可用即可正常处理大多数代码任务；fd、jq、7zip 是增强工具，缺失时只影响部分文件查找、JSON 处理或压缩解压能力，不代表节点不可用。'

export default function CodexToolboxCard({ toolbox, codex, busy, onRepair }: Props) {
  const tools = toolbox?.tools ?? []
  const rg = tools.find((tool) => tool.id === 'rg')
  const codexNeedsRepair = codexCliNeedsRepair(codex)
  const needsRepair = rg?.status !== 'ready' && rg?.repair_action === 'install_env_codex'
  return (
    <section className={styles.toolboxCard}>
      <div className={styles.toolboxHead}>
        <div>
          <div className={styles.toolboxTitleLine}>
            <span className={styles.codexLabel}>Codex 工具箱</span>
            <span className={styles.toolboxHelp}>
              <button
                type="button"
                className={styles.toolboxHelpButton}
                aria-label="Codex 工具箱说明"
                aria-describedby="codex-toolbox-help"
              >
                <HelpCircle size={14} strokeWidth={2.2} aria-hidden="true" />
              </button>
              <span id="codex-toolbox-help" role="tooltip" className={styles.toolboxHelpBubble}>
                {TOOLBOX_HELP_TEXT}
              </span>
            </span>
          </div>
          <h4>{toolboxTitle(rg, codexNeedsRepair)}</h4>
        </div>
        <span className={toolboxBadgeClass(rg, codexNeedsRepair)}>{toolboxBadgeText(rg, codexNeedsRepair)}</span>
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
      <small>{toolStatusText(tool)}</small>
      {tool.path && <code>{tool.path}</code>}
    </div>
  )
}

function toolboxTitle(rg?: CodexToolboxTool, codexNeedsRepair = false) {
  if (codexNeedsRepair) return 'Codex CLI 需要修复'
  if (!rg) return '等待工具状态'
  if (rg.status === 'ready') return '核心工具正常'
  if (rg.status === 'not_runnable') return 'rg 路径异常'
  return 'rg 核心搜索工具未安装'
}

function toolboxBadgeText(rg?: CodexToolboxTool, codexNeedsRepair = false) {
  if (codexNeedsRepair) return '需要修复'
  if (!rg) return '检测中'
  if (rg.status === 'ready') return '可正常使用'
  return '需要修复'
}

function toolboxBadgeClass(rg?: CodexToolboxTool, codexNeedsRepair = false) {
  const tone = codexNeedsRepair || (rg && rg.status !== 'ready') ? 'offline' : !rg ? 'checking' : 'online'
  return [styles.toolboxState, styles[tone]].join(' ')
}

function toolTone(tool: CodexToolboxTool) {
  if (tool.status === 'ready') return 'ready'
  if (isOptionalEnhancement(tool) && tool.status === 'missing') return 'optional'
  if (tool.status === 'missing') return 'missing'
  return 'warn'
}

function toolLine(tool: CodexToolboxTool) {
  const parts = [
    tierLabel(tool.tier),
    policyLabel(tool.install_policy),
    tool.will_inject ? '会注入 PATH' : '不会注入',
    tool.version || toolDiagnosticText(tool),
  ].filter(Boolean)
  return parts.join(' · ')
}

function toolStatusText(tool: CodexToolboxTool) {
  if (tool.status === 'ready') return '可正常使用'
  if (isOptionalEnhancement(tool) && tool.status === 'missing') return '可选增强未安装'
  if (tool.status === 'missing' || tool.status === 'not_runnable') return '需要修复'
  return '需确认'
}

function toolDiagnosticText(tool: CodexToolboxTool) {
  if (isOptionalEnhancement(tool) && tool.status === 'missing') return ''
  return tool.reason || tool.detail || ''
}

function isOptionalEnhancement(tool: CodexToolboxTool) {
  const id = String(tool.id ?? tool.name ?? '').toLowerCase()
  return tool.tier !== 'core' || id === 'fd' || id === 'jq' || id === '7zip'
}

function codexCliNeedsRepair(status?: LocalCliToolStatus | null) {
  if (!status || status.status === 'checking') return false
  if (status.status === 'ready' || status.available) return false
  if (status.available === false) return true
  return ['not_installed', 'not_runnable', 'not_logged_in'].includes(String(status.status ?? ''))
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
