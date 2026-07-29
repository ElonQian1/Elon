import { FileText, GitFork, Sparkles, X } from 'lucide-react'
import { useState, type FormEvent } from 'react'

import type {
  DiscussionActionRequest,
  DiscussionNode,
} from './projectDocumentDiscussionModel'
import styles from './ProjectDocumentDiscussionMap.module.css'

type DiscussionActionMode = 'continue' | 'fork' | 'promote'

interface Props {
  node: DiscussionNode
  mode: DiscussionActionMode
  busy: boolean
  onCancel: () => void
  onSubmit: (request: DiscussionActionRequest) => void
}

const DOCUMENT_TYPES = [
  ['requirement', '需求说明'],
  ['decision', '决策记录'],
  ['feature', '功能说明'],
  ['task', '实施任务'],
] as const

export default function ProjectDocumentDiscussionActionDialog({
  node,
  mode,
  busy,
  onCancel,
  onSubmit,
}: Props) {
  const [details, setDetails] = useState('')
  const [documentType, setDocumentType] = useState('requirement')
  const [targetPath, setTargetPath] = useState('')
  const requiresDetails = mode !== 'promote'
  const title = mode === 'fork' ? '创建备选分支' : mode === 'promote' ? '晋升为正式文档' : '继续讨论'
  const description = mode === 'fork'
    ? '写下与当前节点不同的方案、理由或约束。它会先保存为可追溯来源，再由 AI 创建分支。'
    : mode === 'promote'
      ? '选择正式文档类型；AI 只会晋升已经确认且证据充分的节点，否则报告缺口。'
      : '补充新问题、证据、约束或下一步。它会先保存为可追溯来源，再由 AI 增量编译。'

  function submit(event: FormEvent) {
    event.preventDefault()
    if (requiresDetails && !details.trim()) return
    onSubmit({
      details: details.trim(),
      documentType,
      targetPath: targetPath.trim().replace(/\\/g, '/'),
    })
  }

  return (
    <div className={styles.dialogBackdrop} role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget && !busy) onCancel()
    }}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="discussion-action-title">
        <header>
          <span>{mode === 'fork' ? <GitFork size={17} /> : mode === 'promote' ? <FileText size={17} /> : <Sparkles size={17} />}</span>
          <div><strong id="discussion-action-title">{title}</strong><small>{node.title}</small></div>
          <button type="button" title="关闭" disabled={busy} onClick={onCancel}><X size={15} /></button>
        </header>
        <form onSubmit={submit}>
          <p>{description}</p>
          <label>
            <span>{mode === 'promote' ? '补充要求（可选）' : '本次新增讨论内容'}</span>
            <textarea autoFocus rows={7} value={details} onChange={(event) => setDetails(event.target.value)}
              placeholder={mode === 'fork' ? '例如：如果不依赖中心化索引，是否可以采用联邦发现？需要比较成本与冷启动。'
                : mode === 'promote' ? '例如：保留反对意见，并把未验证部分标记为假设。'
                  : '例如：先验证商户订阅，再开放消费者 AI；需要补充第一阶段指标和失败条件。'} />
          </label>
          {mode === 'promote' && <>
            <label><span>文档类型</span><select value={documentType} onChange={(event) => setDocumentType(event.target.value)}>
              {DOCUMENT_TYPES.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
            </select></label>
            <label><span>目标路径（可选）</span><input value={targetPath} onChange={(event) => setTargetPath(event.target.value)}
              placeholder="docs/current/product/feature-name.md" /></label>
          </>}
          <footer className={styles.dialogActions}>
            <button type="button" disabled={busy} onClick={onCancel}>取消</button>
            <button type="submit" disabled={busy || (requiresDetails && !details.trim())}>
              {busy ? '正在保存来源…' : mode === 'promote' ? '让 AI 评估晋升' : '保存并交给 AI'}
            </button>
          </footer>
        </form>
      </section>
    </div>
  )
}
