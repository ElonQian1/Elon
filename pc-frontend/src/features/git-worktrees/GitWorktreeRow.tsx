import {
  AlertTriangle,
  CheckCircle2,
  Copy,
  ExternalLink,
  MessageSquareText,
  PlayCircle,
  SearchCode,
  ScrollText,
} from 'lucide-react'
import type { DraftMode } from './gitWorktreeActions'
import { matchLabel } from './gitWorktreeActions'
import type { ProjectGitWorktreeAuditEntry } from './types'
import styles from './GitWorktreesPage.module.css'

interface GitWorktreeRowProps {
  entry: ProjectGitWorktreeAuditEntry
  copied: boolean
  asking: boolean
  onCopy: () => void
  onOpen: (mode: DraftMode) => void
  onAsk: () => void
  onRead: () => void
}

export default function GitWorktreeRow({
  entry,
  copied,
  asking,
  onCopy,
  onOpen,
  onAsk,
  onRead,
}: GitWorktreeRowProps) {
  const conversation = entry.conversation
  const dirty = entry.has_uncommitted_changes
  return (
    <article className={styles.worktree} data-dirty={dirty ? 'true' : undefined} data-current={entry.current ? 'true' : undefined}>
      <div className={styles.worktreeMain}>
        <div className={styles.statusIcon} data-state={dirty ? 'dirty' : 'clean'}>
          {dirty ? <AlertTriangle size={16} aria-hidden="true" /> : <CheckCircle2 size={16} aria-hidden="true" />}
        </div>
        <div className={styles.pathBlock}>
          <div className={styles.pathLine}>
            <code title={entry.path}>{entry.path}</code>
            {entry.current && <span className={styles.badge}>当前</span>}
            {entry.bare && <span className={styles.badge}>bare</span>}
            {entry.detached && <span className={styles.badge}>detached</span>}
          </div>
          <div className={styles.metaLine}>
            <span>{entry.branch ?? '无分支'}</span>
            <span>{entry.head ?? '无 HEAD'}</span>
            <span>{entry.uncommitted_count} 项改动</span>
            <span>{entry.untracked_count} 个未跟踪</span>
          </div>
        </div>
      </div>

      <div className={styles.ownerBlock}>
        {conversation ? (
          <>
            <strong>{conversation.title || conversation.conversation_id}</strong>
            <span>{conversation.user_account || conversation.user_id}</span>
            <em>{matchLabel(conversation.match_kind)} · {conversation.match_confidence}%</em>
          </>
        ) : (
          <>
            <strong>{entry.current ? '项目主工作区' : '未识别会话'}</strong>
            <span>{entry.current ? '不是会话 worktree' : '需要人工确认来源'}</span>
            <em>{entry.recommended_action}</em>
          </>
        )}
      </div>

      <div className={styles.actions}>
        <button className={styles.iconBtn} onClick={onCopy} title="复制路径" type="button">
          <Copy size={15} aria-hidden="true" />
          <span>{copied ? '已复制' : '路径'}</span>
        </button>
        <button className={styles.iconBtn} onClick={() => onOpen('open')} disabled={!conversation} title="打开会话" type="button">
          <ExternalLink size={15} aria-hidden="true" />
          <span>打开</span>
        </button>
        <button className={styles.iconBtn} onClick={onRead} disabled={!conversation} title="读取会话上下文" type="button">
          <ScrollText size={15} aria-hidden="true" />
          <span>读取</span>
        </button>
        <button className={styles.iconBtn} onClick={onAsk} disabled={!conversation || asking} title="发送询问到会话讨论" type="button">
          <MessageSquareText size={15} aria-hidden="true" />
          <span>{asking ? '发送中' : '发问'}</span>
        </button>
        <button className={styles.iconBtn} onClick={() => onOpen('continue')} disabled={!conversation} title="继续处理草稿" type="button">
          <PlayCircle size={15} aria-hidden="true" />
          <span>继续</span>
        </button>
      </div>

      {(entry.status_error || entry.status_preview?.length) && (
        <details className={styles.statusPreview}>
          <summary><SearchCode size={14} aria-hidden="true" />状态预览</summary>
          {entry.status_error
            ? <p>{entry.status_error}</p>
            : (
              <pre>
                {(entry.status_preview ?? []).slice(0, 12).join('\n')}
                {entry.status_truncated ? '\n...' : ''}
              </pre>
            )}
        </details>
      )}
    </article>
  )
}
