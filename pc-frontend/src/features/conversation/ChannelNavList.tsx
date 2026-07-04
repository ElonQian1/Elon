import { ChevronDown, ChevronUp, Pin } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { readPinnedChannelIds, writePinnedChannelIds } from './channelNavPrefs'
import type { Channel } from './types'
import styles from './ConversationPage.module.css'

const COLLAPSED_CHANNEL_LIMIT = 5

interface Props {
  projectId: string
  channels: Channel[]
  activeChannelId: string
  onSelectChannel: (channelId: string) => void
}

export default function ChannelNavList({ projectId, channels, activeChannelId, onSelectChannel }: Props) {
  const [expanded, setExpanded] = useState(false)
  const [pinnedIds, setPinnedIds] = useState<string[]>(() => readPinnedChannelIds(projectId))

  useEffect(() => {
    setExpanded(false)
    setPinnedIds(readPinnedChannelIds(projectId))
  }, [projectId])

  const channelIds = useMemo(() => new Set(channels.map((channel) => channel.id)), [channels])
  const pinnedIdSet = useMemo(() => new Set(pinnedIds.filter((id) => channelIds.has(id))), [channelIds, pinnedIds])
  const visibleChannels = useMemo(() => {
    if (expanded || channels.length <= COLLAPSED_CHANNEL_LIMIT + 1) return channels
    const visible = new Map<string, Channel>()
    for (const channel of channels) {
      if (visible.size < COLLAPSED_CHANNEL_LIMIT) visible.set(channel.id, channel)
      if (channel.id === activeChannelId || pinnedIdSet.has(channel.id) || Number(channel.unread_count ?? 0) > 0) {
        visible.set(channel.id, channel)
      }
    }
    return Array.from(visible.values())
  }, [activeChannelId, channels, expanded, pinnedIdSet])
  const hiddenCount = Math.max(0, channels.length - visibleChannels.length)

  function togglePinned(channelId: string) {
    const next = pinnedIdSet.has(channelId)
      ? pinnedIds.filter((id) => id !== channelId)
      : [...pinnedIds, channelId]
    setPinnedIds(next)
    writePinnedChannelIds(projectId, next)
  }

  return (
    <section className={styles.channelNavGroup} aria-label="项目频道">
      <div className={styles.channelSectionHeader}>
        <span>项目频道</span>
        <small>{expanded || hiddenCount === 0 ? channels.length : `${visibleChannels.length}/${channels.length}`}</small>
      </div>
      {channels.length === 0 ? (
        <div className={styles.channelEmpty}>还没有频道</div>
      ) : (
        visibleChannels.map((channel) => {
          const isDev = channel.kind === 'ai_development'
          const pinned = pinnedIdSet.has(channel.id)
          const unreadCount = Number(channel.unread_count ?? 0)
          const active = channel.id === activeChannelId
          return (
            <div
              key={channel.id}
              className={[
                styles.channelItemRow,
                isDev ? styles.devChannel : '',
                active ? styles.channelActive : '',
              ].join(' ')}
            >
              <button className={styles.channelItem} onClick={() => onSelectChannel(channel.id)} type="button">
                <span className={styles.channelGlyph}>{isDev ? '⚒' : '#'}</span>
                <span className={styles.channelMain}>
                  <strong>{channel.name}</strong>
                  {channel.description && <span>{channel.description}</span>}
                </span>
                {unreadCount > 0 && <span className={styles.channelUnread}>{unreadCount > 99 ? '99+' : unreadCount}</span>}
              </button>
              <button
                aria-label={pinned ? '取消固定频道' : '固定频道'}
                className={[styles.channelPinBtn, pinned ? styles.channelPinned : ''].join(' ')}
                onClick={(event) => {
                  event.stopPropagation()
                  togglePinned(channel.id)
                }}
                title={pinned ? '取消固定频道' : '固定频道'}
                type="button"
              >
                <Pin size={12} strokeWidth={2.4} aria-hidden="true" />
              </button>
            </div>
          )
        })
      )}
      {hiddenCount > 0 && (
        <button className={styles.channelExpandBtn} type="button" onClick={() => setExpanded(true)}>
          <ChevronDown size={13} strokeWidth={2.2} aria-hidden="true" />
          <span>展开显示</span>
          <small>{hiddenCount}</small>
        </button>
      )}
      {expanded && channels.length > COLLAPSED_CHANNEL_LIMIT + 1 && (
        <button className={styles.channelExpandBtn} type="button" onClick={() => setExpanded(false)}>
          <ChevronUp size={13} strokeWidth={2.2} aria-hidden="true" />
          <span>收起频道</span>
        </button>
      )}
    </section>
  )
}
