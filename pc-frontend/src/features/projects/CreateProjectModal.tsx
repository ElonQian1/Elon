import { useState, useEffect } from 'react'
import { createPortal } from 'react-dom'
import { api } from '../../api/client'
import { clean } from '../../lib/utils'
import { nodeId, nodeCanAccept, nodeLabel } from './nodeHelpers'
import type { ProjectNode, CreateProjectResult } from './types'
import styles from './CreateProjectModal.module.css'

const STORAGE_NONE = 'none'
const STORAGE_AUTO = 'auto'

export interface CreateProjectOptions {
  quickMode?: boolean
  onCreated?: (project: { id?: string; name?: string }) => void
}

interface Props extends CreateProjectOptions {
  onClose: () => void
}

export function CreateProjectModal({ quickMode = false, onCreated, onClose }: Props) {
  const [name, setName] = useState('')
  const [desc, setDesc] = useState('')
  const [template, setTemplate] = useState('android_kotlin')
  const [repoUrl, setRepoUrl] = useState('')
  const [branch, setBranch] = useState('')
  const [selectedNode, setSelectedNode] = useState('')
  const [storageChoice, setStorageChoice] = useState(STORAGE_NONE)
  const [nodes, setNodes] = useState<ProjectNode[]>([])
  const [storageNodes, setStorageNodes] = useState<ProjectNode[]>([])
  const [nodesLoading, setNodesLoading] = useState(true)
  const [nodesError, setNodesError] = useState('')
  const [knownNodeCount, setKnownNodeCount] = useState(0)
  const [storageHint, setStorageHint] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    loadNodes()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  async function loadNodes() {
    setNodesLoading(true)
    setNodesError('')
    setSelectedNode('')
    try {
      const data = await api.get<{ nodes?: ProjectNode[] }>('/api/me/nodes')
      const allNodes = data.nodes ?? []
      const online = allNodes.filter((n) => n?.online && nodeId(n))
      const ordered = [...online].sort(
        (a, b) => Number(!nodeCanAccept(a)) - Number(!nodeCanAccept(b)),
      )
      const storage = online.filter(
        (n) => (n.storage_ready || n.storage?.enabled) && n.storage_repo_url_configured,
      )
      setKnownNodeCount(allNodes.filter(nodeId).length)
      setNodes(ordered)
      setStorageNodes(storage)
      setStorageHint(
        storage.length
          ? '默认先创建在开发环境上；需要跨 PC 迁移时再启用代码存储。'
          : '项目会直接创建在所选开发环境上。',
      )
      const firstSelectable = ordered.find(nodeCanAccept)
      setSelectedNode(firstSelectable ? nodeId(firstSelectable) : '')
    } catch (err) {
      setNodesError((err as { message?: string }).message ?? '加载失败')
      setKnownNodeCount(0)
      setNodes([])
      setStorageNodes([])
    } finally {
      setNodesLoading(false)
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    const trimName = clean(name)
    if (!trimName) { setError('请输入项目名'); return }
    if (!selectedNode) { setError('请先选择开发环境'); return }
    setError('')
    setSubmitting(true)
    try {
      const result = await api.post<CreateProjectResult>('/api/projects', {
        name: trimName,
        description: clean(desc) || null,
        template: template || 'android_kotlin',
        repo_url: clean(repoUrl) || null,
        branch: clean(branch) || null,
        execution_target: 'pc_node',
        node_id: selectedNode,
        storage_node_id:
          clean(repoUrl) || storageChoice === STORAGE_AUTO || storageChoice === STORAGE_NONE
            ? null
            : storageChoice,
        skip_storage: !!clean(repoUrl) || storageChoice === STORAGE_NONE,
      })
      onCreated?.(result.project ?? {})
      onClose()
    } catch (err) {
      setError((err as { message?: string }).message ?? '创建失败')
    } finally {
      setSubmitting(false)
    }
  }

  const selectableNodeCount = nodes.filter(nodeCanAccept).length
  const nodeSelectStatus = nodesLoading
    ? (quickMode ? '正在加载…' : '正在加载可用开发环境…')
    : nodesError
      ? nodesError
      : nodes.length === 0
        ? (knownNodeCount > 0 ? '开发环境离线' : '没有在线开发环境')
        : selectableNodeCount === 0
          ? '暂无可创建项目的开发环境'
          : ''
  const firstBlockedNode = nodes.find((n) => !nodeCanAccept(n))
  const firstBlockedWarning = Array.isArray(firstBlockedNode?.capacity_warnings)
    ? clean(firstBlockedNode?.capacity_warnings[0])
    : ''
  const nodeHint = nodesLoading || nodesError
    ? ''
    : nodes.length === 0
      ? (knownNodeCount > 0
        ? '你的开发环境当前不在线。请启动本机 PC 节点，登录后再刷新页面。'
        : '还没有检测到你的开发环境。请先启动本机 PC 节点，登录后它会自动绑定到当前账号。')
      : selectableNodeCount === 0
        ? (firstBlockedWarning || '在线开发环境暂不能创建项目，请到节点页检查运行时和容量状态。')
        : ''
  const nodeSelect = (
    <label className={styles.field}>
      <span>开发环境</span>
      <select
        value={selectedNode}
        onChange={(e) => setSelectedNode(e.target.value)}
        disabled={nodesLoading || !!nodesError}
      >
        {nodeSelectStatus && <option value="">{nodeSelectStatus}</option>}
        {nodes.map((n) => {
          const id = nodeId(n)
          const canAccept = nodeCanAccept(n)
          return (
            <option key={id} value={id} disabled={!canAccept}>
              {nodeLabel(n)}
            </option>
          )
        })}
      </select>
      {nodeHint && <small className={styles.hint}>{nodeHint}</small>}
    </label>
  )
  const submitDisabled = submitting || nodesLoading || !!nodesError || !selectedNode

  const modal = (
    <>
      <div className={styles.backdrop} onClick={onClose} />
      <div className={styles.modal} role="dialog" aria-modal="true">
        <header className={styles.header}>
          <div>
            <h2 className={styles.title}>{quickMode ? '创建项目' : '新建项目'}</h2>
            <p className={styles.subtitle}>{quickMode ? '先从一个轻量项目开始' : '云端 APK 开发项目'}</p>
          </div>
          {!quickMode && (
            <button className={styles.closeBtn} type="button" onClick={onClose}>×</button>
          )}
        </header>

        <form className={styles.form} onSubmit={handleSubmit}>
          <label className={styles.field}>
            <span>{quickMode ? '项目名称' : '项目名'}</span>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={quickMode ? '哥哥哈很之旅' : '例如：记账小助手'}
              autoFocus
              required
            />
          </label>

          {!quickMode && (
            <label className={styles.field}>
              <span>项目描述（可选）</span>
              <input value={desc} onChange={(e) => setDesc(e.target.value)} placeholder="简短描述" />
            </label>
          )}

          <label className={styles.field}>
            <span>模板</span>
            <select value={template} onChange={(e) => setTemplate(e.target.value)}>
              <option value="android_kotlin">Android Kotlin（推荐）</option>
              <option value="blank">空项目</option>
            </select>
          </label>

          {!quickMode && (
            <>
              {nodeSelect}

              <label className={styles.field}>
                <span>代码存储</span>
                <select value={storageChoice} onChange={(e) => setStorageChoice(e.target.value)}>
                  <option value={STORAGE_NONE}>暂不使用代码存储（推荐）</option>
                  {storageNodes.length > 0 && (
                    <option value={STORAGE_AUTO}>自动选择代码存储（高级）</option>
                  )}
                  {storageNodes.map((n) => {
                    const id = nodeId(n)
                    return <option key={id} value={id}>{nodeLabel(n)} · 可跨 PC</option>
                  })}
                </select>
                {storageHint && <small className={styles.hint}>{storageHint}</small>}
              </label>

              <label className={styles.field}>
                <span>导入 Git 仓库（可选）</span>
                <input value={repoUrl} onChange={(e) => setRepoUrl(e.target.value)} placeholder="https://github.com/..." />
              </label>

              <label className={styles.field}>
                <span>分支（可选）</span>
                <input value={branch} onChange={(e) => setBranch(e.target.value)} placeholder="main" />
              </label>
            </>
          )}

          {quickMode && (
            nodeSelect
          )}

          {error && <p className={styles.error}>{error}</p>}

          <div className={styles.actions}>
            {!quickMode && (
              <button type="button" className={styles.cancelBtn} onClick={onClose} disabled={submitting}>
                取消
              </button>
            )}
            <button type="submit" className={styles.submitBtn} disabled={submitDisabled}>
              {submitting ? '创建中…' : quickMode ? '创建项目' : '创建'}
            </button>
          </div>
        </form>
      </div>
    </>
  )

  return createPortal(modal, document.body)
}
