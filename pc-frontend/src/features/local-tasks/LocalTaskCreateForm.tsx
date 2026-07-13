import { useEffect, useState } from 'react'
import { Play, ShieldAlert } from 'lucide-react'
import { listLocalFullAccessGrants } from './localTaskApi'
import type { LocalFullAccessGrant, LocalTaskCreateInput } from './types'
import styles from './LocalTasksPage.module.css'

interface Props {
  busy: boolean
  onCreate: (input: LocalTaskCreateInput) => Promise<boolean>
}

export default function LocalTaskCreateForm({ busy, onCreate }: Props) {
  const [initial] = useState(loadInitialContext)
  const [expanded, setExpanded] = useState(true)
  const [projectId, setProjectId] = useState(initial.projectId)
  const [channelId, setChannelId] = useState(initial.channelId)
  const [conversationId, setConversationId] = useState(newConversationId)
  const [workspacePath, setWorkspacePath] = useState(initial.workspacePath)
  const [prompt, setPrompt] = useState('')
  const [grants, setGrants] = useState<LocalFullAccessGrant[]>([])

  useEffect(() => {
    let cancelled = false
    void listLocalFullAccessGrants().then((items) => {
      if (cancelled) return
      setGrants(items)
      const first = items[0]
      if (first && !initial.projectId.trim() && !initial.workspacePath.trim()) {
        setProjectId(first.project_id)
        setWorkspacePath(first.workspace_path)
      }
    }).catch(() => {
      // Manual project/path entry remains available when grant discovery fails.
    })
    return () => { cancelled = true }
  }, [initial])

  async function submit(event: React.FormEvent) {
    event.preventDefault()
    const input: LocalTaskCreateInput = {
      project_id: projectId.trim(),
      channel_id: channelId.trim() || undefined,
      conversation_id: conversationId.trim(),
      workspace_path: workspacePath.trim(),
      prompt: prompt.trim(),
      runtime_permission: 'full_access',
    }
    if (!input.workspace_path || !input.prompt || busy) return
    try {
      window.localStorage.setItem('elon.pc.localTasks.context.v1', JSON.stringify({
        projectId: input.project_id,
        channelId: input.channel_id ?? '',
        workspacePath: input.workspace_path,
      }))
    } catch {
      // Local storage is a convenience only.
    }
    if (await onCreate(input)) {
      setPrompt('')
      setConversationId(newConversationId())
    }
  }

  return (
    <section className={styles.createCard}>
      <button
        type="button"
        className={styles.createToggle}
        onClick={() => setExpanded((value) => !value)}
        aria-expanded={expanded}
      >
        <span>
          <strong>启动本机 Codex</strong>
          <small>任务直接交给这台电脑，不依赖云端调度</small>
        </span>
        <em>{expanded ? '收起' : '展开'}</em>
      </button>
      {expanded && (
        <form className={styles.createForm} onSubmit={submit}>
          {grants.length > 0 && (
            <label className={styles.fieldWide}>
              <span>本机已授权项目</span>
              <select
                value={matchingGrantIndex(grants, projectId, workspacePath)}
                onChange={(event) => {
                  const grant = grants[Number(event.target.value)]
                  if (!grant) return
                  setProjectId(grant.project_id)
                  setWorkspacePath(grant.workspace_path)
                }}
              >
                <option value="">手动填写其他项目目录</option>
                {grants.map((grant, index) => (
                  <option key={`${grant.project_id}:${grant.workspace_path}`} value={index}>
                    {grant.project_id} · {grant.workspace_path}
                  </option>
                ))}
              </select>
            </label>
          )}
          <label className={styles.fieldWide}>
            <span>工作目录</span>
            <input
              value={workspacePath}
              onChange={(event) => setWorkspacePath(event.target.value)}
              placeholder="D:\\work\\project"
              required
            />
          </label>
          <label className={styles.fieldWide}>
            <span>交给 Codex 的任务</span>
            <textarea
              value={prompt}
              onChange={(event) => setPrompt(event.target.value)}
              placeholder="描述要在本机完成的工作…"
              rows={4}
              required
            />
          </label>
          <label>
            <span>权限</span>
            <select
              value="full_access"
              disabled
            >
              <option value="full_access">完全访问（已授权目录）</option>
            </select>
          </label>
          <label>
            <span>项目标识</span>
            <input value={projectId} onChange={(event) => setProjectId(event.target.value)} required />
          </label>
          <label>
            <span>频道标识（可选）</span>
            <input
              value={channelId}
              onChange={(event) => setChannelId(event.target.value)}
              placeholder="留空自动写回 AI开发 频道"
            />
          </label>
          <label>
            <span>会话标识</span>
            <input value={conversationId} onChange={(event) => setConversationId(event.target.value)} required />
          </label>
          <p className={styles.permissionWarning}>
            <ShieldAlert size={14} aria-hidden="true" />
            首次使用该项目目录时会在本机弹窗确认完全访问；离线任务只使用你自己的 Codex 登录。
          </p>
          <button
            className={styles.primaryButton}
            type="submit"
            disabled={busy || !workspacePath.trim() || !prompt.trim()}
          >
            <Play size={15} fill="currentColor" aria-hidden="true" />
            {busy ? '正在启动…' : '在本机启动'}
          </button>
        </form>
      )}
    </section>
  )
}

function loadInitialContext(): { projectId: string; channelId: string; workspacePath: string } {
  const query = new URLSearchParams(location.search)
  const fromQuery = {
    projectId: query.get('project_id')?.trim() ?? '',
    channelId: query.get('channel_id')?.trim() ?? '',
    workspacePath: query.get('workspace_path')?.trim() ?? '',
  }
  if (fromQuery.projectId || fromQuery.channelId || fromQuery.workspacePath) return fromQuery
  try {
    const raw = window.localStorage.getItem('elon.pc.localTasks.context.v1')
    const saved = raw ? JSON.parse(raw) as Record<string, unknown> : {}
    return {
      projectId: String(saved.projectId ?? '').trim(),
      channelId: String(saved.channelId ?? '').trim(),
      workspacePath: String(saved.workspacePath ?? '').trim(),
    }
  } catch {
    return { projectId: '', channelId: '', workspacePath: '' }
  }
}

function matchingGrantIndex(
  grants: LocalFullAccessGrant[],
  projectId: string,
  workspacePath: string,
): string {
  const normalizedPath = workspacePath.trim().replace(/\//g, '\\').toLocaleLowerCase('en-US')
  const index = grants.findIndex((grant) => grant.project_id === projectId.trim()
    && grant.workspace_path.trim().replace(/\//g, '\\').toLocaleLowerCase('en-US') === normalizedPath)
  return index >= 0 ? String(index) : ''
}

function newConversationId(): string {
  if (typeof crypto.randomUUID === 'function') return `local-${crypto.randomUUID()}`
  return `local-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
}
