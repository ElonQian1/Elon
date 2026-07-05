import { useMemo, useState } from 'react'
import { api } from '../../api/client'
import type { Channel, ChannelCategory } from '../conversation/types'
import styles from './ProjectDetailPage.module.css'

interface Props {
  projectId: string
  channels: Channel[]
  categories: ChannelCategory[]
  canEdit: boolean
  onChanged: () => Promise<void> | void
  onOpenChannel: (channelId: string) => void
}

const DEFAULT_CHANNEL_KINDS = new Set([
  'announcements',
  'docs',
  'discussion',
  'requirements',
  'suggestions',
  'issues',
  'ai_development',
  'builds',
])

export default function ProjectChannelsTab({
  projectId,
  channels,
  categories,
  canEdit,
  onChanged,
  onOpenChannel,
}: Props) {
  const [name, setName] = useState('')
  const [categoryId, setCategoryId] = useState(categories[0]?.id ?? '')
  const [editing, setEditing] = useState<Record<string, string>>({})
  const [busy, setBusy] = useState('')
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')
  const channelsByCategory = useMemo(() => groupChannels(channels), [channels])

  async function createChannel(event: React.FormEvent) {
    event.preventDefault()
    const clean = name.trim()
    if (!projectId || !clean || !canEdit) return
    setBusy('create')
    setError('')
    setMessage('')
    try {
      await api.post(`/api/projects/${encodeURIComponent(projectId)}/channels`, {
        name: clean,
        category_id: categoryId || undefined,
      })
      setName('')
      setMessage('频道已创建')
      await onChanged()
    } catch (err) {
      setError(errorMessage(err, '创建频道失败'))
    } finally {
      setBusy('')
    }
  }

  async function renameChannel(channel: Channel) {
    const clean = (editing[channel.id] ?? channel.name).trim()
    if (!projectId || !clean || !canEdit || clean === channel.name) return
    setBusy(`rename:${channel.id}`)
    setError('')
    setMessage('')
    try {
      await api.patch(`/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channel.id)}`, {
        name: clean,
      })
      setEditing((current) => ({ ...current, [channel.id]: clean }))
      setMessage('频道名称已更新')
      await onChanged()
    } catch (err) {
      setError(errorMessage(err, '改名失败'))
    } finally {
      setBusy('')
    }
  }

  async function deleteChannel(channel: Channel) {
    if (!projectId || !canEdit || isDefaultChannel(channel)) return
    if (!window.confirm(`确认删除频道“${channel.name}”？频道消息也会被删除。`)) return
    setBusy(`delete:${channel.id}`)
    setError('')
    setMessage('')
    try {
      await api.delete(`/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channel.id)}`)
      setMessage('频道已删除')
      await onChanged()
    } catch (err) {
      setError(errorMessage(err, '删除失败'))
    } finally {
      setBusy('')
    }
  }

  return (
    <div className={styles.managementStack}>
      <section className={styles.panel}>
        <div className={styles.panelHeader}>
          <div>
            <strong>频道管理</strong>
            <span>创建、改名、进入频道；默认频道保护不允许删除。</span>
          </div>
          <span>{channels.length} 个频道</span>
        </div>
        <form className={styles.managementForm} onSubmit={createChannel}>
          <label className={styles.field}>
            <span>新频道名称</span>
            <input value={name} onChange={(event) => setName(event.target.value)} placeholder="例如 版本计划" disabled={!canEdit || busy === 'create'} />
          </label>
          <label className={styles.field}>
            <span>分类</span>
            <select value={categoryId} onChange={(event) => setCategoryId(event.target.value)} disabled={!canEdit || busy === 'create'}>
              <option value="">不放入分类</option>
              {categories.map((category) => (
                <option key={category.id} value={category.id}>{category.name}</option>
              ))}
            </select>
          </label>
          <button className={styles.primaryBtn} type="submit" disabled={!canEdit || busy === 'create' || !name.trim()}>
            {busy === 'create' ? '创建中' : '新建频道'}
          </button>
        </form>
        {!canEdit && <p className={styles.formHint}>当前角色可以浏览频道；新建、改名和删除需要项目编辑权限。</p>}
        {message && <p className={styles.formSuccess}>{message}</p>}
        {error && <p className={styles.formError}>{error}</p>}
      </section>

      {categoryRows(categories, channelsByCategory).map((row) => (
        <section className={styles.panel} key={row.id}>
          <div className={styles.panelHeader}>
            <div>
              <strong>{row.name}</strong>
              <span>{row.channels.length} 个频道</span>
            </div>
          </div>
          <div className={styles.rowList}>
            {row.channels.map((channel) => {
              const defaultChannel = isDefaultChannel(channel)
              const value = editing[channel.id] ?? channel.name
              const channelBusy = busy.endsWith(channel.id)
              return (
                <div className={styles.rowItem} key={channel.id}>
                  <div className={styles.rowMain}>
                    <input
                      value={value}
                      onChange={(event) => setEditing((current) => ({ ...current, [channel.id]: event.target.value }))}
                      disabled={!canEdit || channelBusy}
                    />
                    <span>{channel.kind}{defaultChannel ? ' · 默认频道' : ''}</span>
                  </div>
                  <div className={styles.rowActions}>
                    <button className={styles.textBtn} type="button" onClick={() => onOpenChannel(channel.id)}>打开</button>
                    <button className={styles.textBtn} type="button" disabled={!canEdit || channelBusy || value.trim() === channel.name} onClick={() => renameChannel(channel)}>
                      保存
                    </button>
                    <button className={styles.textBtn} data-danger="true" type="button" disabled={!canEdit || defaultChannel || channelBusy} onClick={() => deleteChannel(channel)}>
                      删除
                    </button>
                  </div>
                </div>
              )
            })}
            {row.channels.length === 0 && <p className={styles.empty}>暂无频道</p>}
          </div>
        </section>
      ))}
    </div>
  )
}

function groupChannels(channels: Channel[]) {
  const map = new Map<string, Channel[]>()
  channels.forEach((channel) => {
    const key = channel.category_id || ''
    map.set(key, [...(map.get(key) ?? []), channel])
  })
  return map
}

function categoryRows(categories: ChannelCategory[], channelsByCategory: Map<string, Channel[]>) {
  const rows = categories.map((category) => ({
    id: category.id,
    name: category.name,
    channels: channelsByCategory.get(category.id) ?? [],
  }))
  const uncategorized = channelsByCategory.get('') ?? []
  if (uncategorized.length) rows.push({ id: 'uncategorized', name: '未分类', channels: uncategorized })
  return rows
}

function isDefaultChannel(channel: Channel) {
  return DEFAULT_CHANNEL_KINDS.has(channel.kind ?? '')
}

function errorMessage(err: unknown, fallback: string) {
  return (err as { message?: string })?.message ?? fallback
}
