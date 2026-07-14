import { CheckCircle2, FilePenLine, FolderInput, ShieldCheck } from 'lucide-react'
import { useState } from 'react'

import type { SuggestedFileOperation } from './projectDocumentSections'
import styles from './ProjectDocumentsWorkspace.module.css'

interface Props {
  operations: SuggestedFileOperation[]
  canApply: boolean
  applying: boolean
  onApply: (input: {
    operationIds: string[]
    allowRename: boolean
    allowMove: boolean
  }) => Promise<void>
}

export default function ProjectDocumentFileOperations({ operations, canApply, applying, onApply }: Props) {
  const [selected, setSelected] = useState<string[]>([])
  const [reviewed, setReviewed] = useState(false)
  const proposed = operations.filter((operation) => operation.status === 'proposed')
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
        <span>逐项授权，不给 AI 任意文件写权限</span>
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
              <b>{applied ? '已执行' : '待审核'}</b>
            </label>
          )
        })}
        {!operations.length && <p>AI 没有提出实体文件调整。</p>}
      </div>
      {!!proposed.length && (
        <footer>
          <label className={styles.reviewPermission}>
            <input type="checkbox" checked={reviewed} onChange={(event) => setReviewed(event.target.checked)} />
            <span>我已核对源路径和目标路径，允许本次选中的{allowRename ? '重命名' : ''}{allowRename && allowMove ? '和' : ''}{allowMove ? '移动' : ''}操作</span>
          </label>
          <p>只改变 Markdown 路径并同步虚拟归类；不覆盖、不删除、不改正文，也不会自动 commit 或 push。</p>
          {!canApply && <p className={styles.permissionWarning}>实体整理必须连接项目本机节点并取得最新 MCP 目录 revision；服务器回退副本只能阅读。</p>}
          <button type="button" disabled={!canApply || !reviewed || selected.length === 0 || applying} onClick={applySelected}>
            {applying ? '正在安全执行…' : `执行已选 ${selected.length} 项`}
          </button>
        </footer>
      )}
    </section>
  )
}
