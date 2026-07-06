import type { FormEvent, KeyboardEvent, RefObject } from 'react'
import { SendHorizontal } from 'lucide-react'
import { AttachmentButton, AttachmentChip, type UploadedAttachment } from './AttachmentButton'
import ComposerRuntimeToggles from './ComposerRuntimeToggles'
import type { MemberConversationEntry } from './memberConversationApi'
import type { RuntimeRoute } from './runtimeRoutes'
import type { RouteModelButtonCopy } from '../models/routeModelPolicy'
import type { AgentOption } from '../models/types'
import styles from './ConversationComposer.module.css'

interface ConversationComposerProps {
  attachmentDropActive: boolean
  attachments: UploadedAttachment[]
  attachmentUploading: boolean
  composerDisabled: boolean
  composerRuntimeRoute: RuntimeRoute
  directPcCliActive: boolean
  directPcCliAvailable: boolean
  input: string
  isOwnConversationTarget: boolean
  localNodeReady: boolean
  memberConversations: MemberConversationEntry[]
  modelButtonCopy: RouteModelButtonCopy
  modelButtonRef: RefObject<HTMLButtonElement>
  modelOptions: AgentOption[]
  placeholder: string
  selectedAgent: string
  sendError: string
  attachmentError: string
  sending: boolean
  sessionView: string | 'new' | null
  shouldPreferLocalNode: boolean
  submitDisabled: boolean
  textareaRef: RefObject<HTMLTextAreaElement>
  activeProjectId: string
  onAutoResize: () => void
  onDirectPcCliChange: (enabled: boolean) => void
  onFilesSelected: (files: File[]) => void
  onInputChange: (value: string) => void
  onKeyDown: (event: KeyboardEvent<HTMLTextAreaElement>) => void
  onOpenAttachment: (attachment: UploadedAttachment) => void
  onRemoveAttachment: (attachmentId: string) => void
  onSubmit: (event: FormEvent<HTMLFormElement>) => void
  onToggleModelPicker: () => void
}

export default function ConversationComposer({
  attachmentDropActive,
  attachments,
  attachmentUploading,
  composerDisabled,
  composerRuntimeRoute,
  directPcCliActive,
  directPcCliAvailable,
  input,
  isOwnConversationTarget,
  localNodeReady,
  memberConversations,
  modelButtonCopy,
  modelButtonRef,
  modelOptions,
  placeholder,
  selectedAgent,
  sendError,
  attachmentError,
  sending,
  sessionView,
  shouldPreferLocalNode,
  submitDisabled,
  textareaRef,
  activeProjectId,
  onAutoResize,
  onDirectPcCliChange,
  onFilesSelected,
  onInputChange,
  onKeyDown,
  onOpenAttachment,
  onRemoveAttachment,
  onSubmit,
  onToggleModelPicker,
}: ConversationComposerProps) {
  const sendLabel = sending ? '发送中' : attachmentUploading ? '上传中' : '发送'

  return (
    <form
      className={styles.form}
      data-drop-active={attachmentDropActive ? 'true' : 'false'}
      onSubmit={onSubmit}
    >
      <div className={styles.panel}>
        {attachments.length > 0 && (
          <div className={styles.attachmentTray}>
            {attachments.map((attachment) => (
              <AttachmentChip
                key={attachment.attachment_id}
                attachment={attachment}
                onOpen={onOpenAttachment}
                onRemove={() => onRemoveAttachment(attachment.attachment_id)}
              />
            ))}
          </div>
        )}

        <div className={styles.metaDock}>
          <button
            ref={modelButtonRef}
            className={styles.modelBtn}
            type="button"
            title={modelButtonCopy.title}
            onClick={onToggleModelPicker}
          >
            <span>{modelButtonCopy.source}</span>
            <strong>{modelButtonCopy.detail}</strong>
          </button>

          <ComposerRuntimeToggles
            activeProjectId={activeProjectId}
            directPcCliActive={directPcCliActive}
            shouldPreferLocalNode={shouldPreferLocalNode}
            localNodeReady={localNodeReady}
            directPcCliAvailable={directPcCliAvailable}
            composerDisabled={composerDisabled}
            onDirectPcCliChange={onDirectPcCliChange}
            isOwnConversationTarget={isOwnConversationTarget}
            sessionView={sessionView}
            memberConversations={memberConversations}
            selectedAgent={selectedAgent}
            modelOptions={modelOptions}
            composerRuntimeRoute={composerRuntimeRoute}
          />
        </div>

        <div className={styles.inputDock}>
          <AttachmentButton
            disabled={composerDisabled}
            uploading={attachmentUploading}
            onFilesSelected={onFilesSelected}
          />
          <textarea
            ref={textareaRef}
            className={styles.textarea}
            value={input}
            onChange={(event) => {
              onInputChange(event.target.value)
              onAutoResize()
            }}
            onKeyDown={onKeyDown}
            placeholder={placeholder}
            disabled={composerDisabled}
            rows={1}
          />
          <button
            className={styles.sendBtn}
            type="submit"
            disabled={submitDisabled}
            title={sendLabel}
            aria-label={sendLabel}
          >
            {sending ? <span className={styles.sendingMark}>...</span> : <SendHorizontal size={17} aria-hidden="true" />}
          </button>
        </div>

        {(sendError || attachmentError) && (
          <div className={styles.errorStack}>
            {sendError && <p>{sendError}</p>}
            {attachmentError && <p>{attachmentError}</p>}
          </div>
        )}
      </div>
    </form>
  )
}
