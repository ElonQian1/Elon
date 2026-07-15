import { CheckCircle2, FilePenLine, FolderInput, ShieldCheck } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'

import type { DocumentAutomationMode, SuggestedFileOperation } from './projectDocumentSections'
import styles from './ProjectDocumentsWorkspace.module.css'

interface Props {
  operations: SuggestedFileOperation[]
  canApply: boolean
  applying: boolean
  automationMode: DocumentAutomationMode
  onApply: (input: {
    operationIds: string[]
    allowRename: boolean
    allowMove: boolean
  }) => Promise<void>
}

export default function ProjectDocumentFileOperations({ operations, canApply, applying, automationMode, onApply }: Props) {
  const [selected, setSelected] = useState<string[]>([])
  const [reviewed, setReviewed] = useState(false)
  const proposed = operations.filter((operation) => operation.status === 'proposed')
  const proposedIds = useMemo(
    () => operations.filter((operation) => operation.status === 'proposed').map((operation) => operation.id),
    [operations],
  )

  useEffect(() => {
    if (automationMode === 'git_backed_full' || automationMode === 'trusted_reversible') setSelected(proposedIds)
  }, [automationMode, proposedIds])

  const selectedOperations = proposed.filter((operation) => selected.includes(operation.id))
  const allowRename = selectedOperations.some((operation) => operation.kind === 'rename')
  const allowMove = selectedOperations.some((operation) => operation.kind === 'move')

  async function applySelected() {
    try {
      await onApply({ operationIds: selected, allowRename, allowMove })
      setSelected([])
      setReviewed(false)
    } catch {
      // Parent hook keeps the structured error visible and the selection intact for retry.
    }
  }

  return (
    <section className={styles.fileOperations}>
      <header>
        <div><ShieldCheck size={17} aria-hidden="true" /><h3>实体文件整理 <em>{proposed.length}</em></h3></div>
        <span>{automationMode === 'git_backed_full' ? 'Git 备份后完全开放整理权限' : automationMode === 'trusted_reversible' ? '已开放安全可恢复权限' : automationMode === 'review_all' ? '逐项审核后执行' : '只展示建议'}</span>
      </header>
      <div className={styles.fileOperationList}>
        {operations.map((operation) => {
          const applied = operation.status === 'applied'
          const checked = selected.includes(operation.id)
          return (
            <label key={operation.id} data-applied={applied}>
              <input
                type="checkbox"
                checked={applied || checked}
                disabled={applied || applying}
                onChange={() => setSelected((current) => checked
                  ? current.filter((id) => id !== operation.id)
                  : [...current, operation.id])}
              />
              <i>{applied ? <CheckCircle2 size={16} /> : operation.kind === 'rename' ? <FilePenLine size={16} /> : <FolderInput size={16} />}</i>
              <div>
                <strong>{operation.kind === 'rename' ? '重命名' : '移动'} · {operation.reason || '改善文档可检索性'}</strong>
                <code>{operation.source_path}</code>
                <span>→ {operation.target_path}</span>
              </div>
              <b>{applied ? '已执行' : automationMode === 'git_backed_full' || automationMode === 'trusted_reversible' ? 'AI 可执行' : '待审核'}</b>
            </label>
          )
        })}
        {!operations.length && <p>AI 没有提出实体文件调整。</p>}
      </div>
      {!!proposed.length && (
        <footer>
          {automationMode === 'review_all' && <label className={styles.reviewPermission}>
              <input type="checkbox" checked={reviewed} onChange={(event) => setReviewed(event.target.checked)} />
              <span>我已核对源路径和目标路径，允许本次选中的{allowRename ? '重命名' : ''}{allowRename && allowMove ? '和' : ''}{allowMove ? '移动' : ''}操作</span>
            </label>}
          {(automationMode === 'git_backed_full' || automationMode === 'trusted_reversible') && <p className={styles.trustedPermission}><ShieldCheck size={14} /> {automationMode === 'git_backed_full' ? '程序会先备份原始文档，整理完成后再提交结果。' : 'AI 可自动执行已选安全操作；你仍可取消任意一项。'}</p>}
          {automationMode === 'suggestions_only' && <p className={styles.permissionWarning}>当前权限为“仅生成建议”，不会执行这些文件操作。</p>}
          <p>只改变 Markdown 路径并同步虚拟归类；不覆盖、不删除、不改正文；Git 模式只提交文档，绝不自动 push。</p>
          {!canApply && <p className={styles.permissionWarning}>实体整理必须连接项目本机节点并取得最新 MCP 目录 revision；服务器回退副本只能阅读。</p>}
          <button type="button" disabled={!canApply || automationMode === 'suggestions_only' || (automationMode === 'review_all' && !reviewed) || selected.length === 0 || applying} onClick={applySelected}>
            {applying ? '正在安全执行…' : `执行已选 ${selected.length} 项`}
          </button>
        </footer>
      )}
    </section>
  )
}
