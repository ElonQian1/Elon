import { useState, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { useNavigate } from 'react-router-dom'
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
  const navigate = useNavigate()
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

  const loadNodes = useCallback(async () => {
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
  }, [])

  useEffect(() => {
    void loadNodes()
    const poll = window.setInterval(() => void loadNodes(), 5000)
    return () => window.clearInterval(poll)
  }, [loadNodes])

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
    ? (quickMode ? '正在查找你的电脑…' : '正在查找可用电脑…')
    : nodesError
      ? '暂时无法读取电脑状态'
      : nodes.length === 0
        ? (knownNodeCount > 0 ? '你的电脑暂时未连接' : '还没有连接开发平台')
        : selectableNodeCount === 0
          ? '电脑已连接，但暂时不能创建项目'
          : ''
  const firstBlockedNode = nodes.find((n) => !nodeCanAccept(n))
  const firstBlockedWarning = Array.isArray(firstBlockedNode?.capacity_warnings)
    ? clean(firstBlockedNode?.capacity_warnings[0])
    : ''
  const nodeHint = nodesLoading || nodesError
    ? ''
    : nodes.length === 0
      ? (knownNodeCount > 0
        ? '你之前连接过一台电脑，但它现在不在线。请在那台电脑启动“一龙开发平台”，并登录当前账号。'
        : '项目文件需要创建在你的电脑上。请先安装并启动“一龙开发平台”，登录当前账号。')
      : selectableNodeCount === 0
        ? (firstBlockedWarning
          ? `电脑已连接，但暂时不能创建项目。原因：${firstBlockedWarning}`
          : '电脑已连接，但暂时不能创建项目。请打开开发平台查看详情。')
        : ''
  const showNodeRecovery = !nodesLoading && (!selectedNode || !!nodesError || selectableNodeCount === 0)
  const nodeRecoveryTitle = nodesError
    ? '无法读取电脑状态'
    : nodes.length === 0
      ? (knownNodeCount > 0 ? '电脑暂时未连接' : '需要先连接你的电脑')
      : '电脑已连接，但暂时不能创建项目'
  const nodeSelect = (
    <label className={styles.field}>
      <span>项目创建位置</span>
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
      {showNodeRecovery && (
        <div className={styles.nodeRecovery} role="status" aria-live="polite">
          <div className={styles.nodeRecoveryText}>
            <strong>{nodeRecoveryTitle}</strong>
            <span>
              {nodesError
                ? '请先点击“我已启动，重新检测”；如果仍失败，再打开开发平台检查登录状态。'
                : '项目文件会创建在你的电脑上。安装并启动“一龙开发平台”后，这个窗口会自动检测。'}
            </span>
          </div>
          <div className={styles.nodeRecoveryActions}>
            <button
              type="button"
              className={styles.secondaryBtn}
              onClick={() => void loadNodes()}
              disabled={nodesLoading}
            >
              我已启动，重新检测
            </button>
            <button type="button" className={styles.linkBtn} onClick={() => navigate('/node')}>
              安装/启动开发平台
            </button>
          </div>
        </div>
      )}
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
