import type { Channel } from './types'
import styles from './ConversationPage.module.css'

interface Props {
  channels: Channel[]
  activeChannelId: string
  onSelectChannel: (channelId: string) => void
}

export default function ChannelNavList({ channels, activeChannelId, onSelectChannel }: Props) {
  return (
    <section className={styles.channelNavGroup} aria-label="项目频道">
      <div className={styles.channelSectionHeader}>
        <span>项目频道</span>
        <small>{channels.length}</small>
      </div>
      {channels.length === 0 ? (
        <div className={styles.channelEmpty}>还没有频道</div>
      ) : (
        channels.map((channel) => {
          const isDev = channel.kind === 'ai_development'
          return (
            <button
              key={channel.id}
              className={[
                styles.channelItem,
                isDev ? styles.devChannel : '',
                channel.id === activeChannelId ? styles.channelActive : '',
              ].join(' ')}
              onClick={() => onSelectChannel(channel.id)}
              type="button"
            >
              <span className={styles.channelGlyph}>{isDev ? '⚒' : '#'}</span>
              <span className={styles.channelMain}>
                <strong>{channel.name}</strong>
                {channel.description && <span>{channel.description}</span>}
              </span>
            </button>
          )
        })
      )}
    </section>
  )
}
