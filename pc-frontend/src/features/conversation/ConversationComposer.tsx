import type { FormEventHandler, KeyboardEventHandler, RefObject } from 'react'
import type { RouteModelButtonCopy } from '../models/routeModelPolicy'
import { AttachmentButton, AttachmentChip } from './AttachmentButton'
import type { UploadedAttachment } from './AttachmentButton'
import styles from './ConversationComposer.module.css'

interface ConversationComposerProps {
  projectId: string
  input: string
  attachments: UploadedAttachment[]
  sendError: string
  modelButtonCopy: RouteModelButtonCopy
  modelButtonRef: RefObject<HTMLButtonElement>
  textareaRef: RefObject<HTMLTextAreaElement>
  directPcCliActive: boolean
  shouldPreferLocalNode: boolean
  localNodeReady: boolean
  directPcCliAvailable: boolean
  composerDisabled: boolean
  sending: boolean
  placeholder: string
  onSubmit: FormEventHandler<HTMLFormElement>
  onOpenModelPicker: () => void
  onToggleDirectPcCli: (enabled: boolean) => void
  onInputChange: (value: string) => void
  onKeyDown: KeyboardEventHandler<HTMLTextAreaElement>
  onAttach: (attachment: UploadedAttachment) => void
  onRemoveAttachment: (attachmentId: string) => void
}

export default function ConversationComposer({
  projectId,
  input,
  attachments,
  sendError,
  modelButtonCopy,
  modelButtonRef,
  textareaRef,
  directPcCliActive,
  shouldPreferLocalNode,
  localNodeReady,
  directPcCliAvailable,
  composerDisabled,
  sending,
  placeholder,
  onSubmit,
  onOpenModelPicker,
  onToggleDirectPcCli,
  onInputChange,
  onKeyDown,
  onAttach,
  onRemoveAttachment,
}: ConversationComposerProps) {
  return (
    <form onSubmit={onSubmit}>
      {attachments.length > 0 && (
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, padding: '6px 16px 0' }}>
          {attachments.map((attachment) => (
            <AttachmentChip
              key={attachment.attachment_id}
              attachment={attachment}
              onRemove={() => onRemoveAttachment(attachment.attachment_id)}
            />
          ))}
        </div>
      )}
      <div className={styles.composer}>
        <button
          ref={modelButtonRef}
          className={styles.composerModelBtn}
          type="button"
          title={modelButtonCopy.title}
          onClick={onOpenModelPicker}
        >
          <span>{modelButtonCopy.source}</span>
          <strong>{modelButtonCopy.detail}</strong>
        </button>

        <label
          className={styles.directCliToggle}
          data-active={directPcCliActive ? 'true' : 'false'}
          data-default-local={shouldPreferLocalNode && localNodeReady ? 'true' : 'false'}
          data-disabled={!directPcCliAvailable || composerDisabled ? 'true' : 'false'}
          title="自动模式会在可用时使用本机节点；打开后强制交给本机 AI CLI"
        >
          <input
            type="checkbox"
            checked={directPcCliActive}
            disabled={!directPcCliAvailable || composerDisabled}
            onChange={(event) => onToggleDirectPcCli(event.target.checked)}
          />
          <span className={styles.directCliSwitch} aria-hidden="true" />
          <span className={styles.directCliCopy}>
            <strong>{directPcCliActive ? '直连CLI' : '自动'}</strong>
            <em>{!directPcCliAvailable ? '未就绪' : directPcCliActive ? '直连' : '自动'}</em>
          </span>
        </label>

        <textarea
          ref={textareaRef}
          className={styles.composerTextarea}
          value={input}
          onChange={(event) => onInputChange(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder={placeholder}
          disabled={composerDisabled}
          rows={1}
        />

        <AttachmentButton
          projectId={projectId}
          disabled={composerDisabled}
          onAttached={onAttach}
        />

        <button
          className={styles.sendBtn}
          type="submit"
          disabled={(!input.trim() && attachments.length === 0) || composerDisabled}
        >
          {sending ? '…' : '发送'}
        </button>
      </div>
      {sendError && <p className={styles.sendError}>{sendError}</p>}
    </form>
  )
}
