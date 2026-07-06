import { HelpCircle, Wrench } from 'lucide-react'
import type { CodexToolboxStatus, CodexToolboxTool, LocalCliToolStatus } from './types'
import styles from './NodePage.module.css'

interface Props {
  toolbox?: CodexToolboxStatus | null
  codex?: LocalCliToolStatus | null
  busy: boolean
  onRepair: () => void
}

const TOOLBOX_HELP_TEXT = '你可以把 Codex 工具箱理解成 Codex 在这台电脑上干活时自动使用的随身工具包。它不是让你手动操作的功能；rg 正常时，Codex 就能快速搜索代码并处理大多数任务。fd、jq、7zip 只是少数场景的增强项，缺失不代表节点不可用。'

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
      <p className={styles.toolboxIntro}>{toolboxIntro(rg, codexNeedsRepair)}</p>
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
        <span>{toolPurpose(tool)}</span>
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

function toolStatusText(tool: CodexToolboxTool) {
  if (tool.status === 'ready') return '可正常使用'
  if (isOptionalEnhancement(tool) && tool.status === 'missing') return '可选增强未安装'
  if (tool.status === 'missing' || tool.status === 'not_runnable') return '需要修复'
  return '需确认'
}

function toolboxIntro(rg?: CodexToolboxTool, codexNeedsRepair = false) {
  if (codexNeedsRepair) {
    return '这是给 Codex 在本机干活时自动使用的工具包。当前 Codex CLI 需要修复，所以需要先处理 Codex 本体。'
  }
  if (rg?.status === 'ready') {
    return '这是给 Codex 在本机干活时自动使用的工具包。核心搜索工具 rg 已正常，Codex 可以快速搜索代码并处理大多数任务；下面几个只是可选增强。'
  }
  return '这是给 Codex 在本机干活时自动使用的工具包。核心搜索工具 rg 还没准备好，修复后 Codex 搜索代码会更稳定、更快。'
}

function toolPurpose(tool: CodexToolboxTool) {
  const id = String(tool.id ?? tool.name ?? '').toLowerCase()
  if (id === 'rg') return '核心：帮 Codex 快速搜索项目里的代码和文字'
  if (id === 'fd') return '增强：按文件名更快找到目标文件'
  if (id === 'jq') return '增强：读取和整理 JSON 配置或接口数据'
  if (id === '7zip') return '增强：处理压缩包、解压或打包文件'
  return isOptionalEnhancement(tool) ? '增强：少数任务会用到的辅助能力' : '核心：Codex 执行任务时会自动使用'
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
