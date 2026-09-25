import { gridPrompt, type GridAttachment } from './gridChatSnapshot'

/** Materialize before handoff so even a locally rejected send retains the complete draft.
 * Once handed off, the existing sender alone owns restoration and uncertain-send reconciliation. */
export function prepareGridChatSend(options: {
  question: string
  attachment: GridAttachment
  scope: string
  now?: number
  setDraft: (prompt: string) => void
  removeAttachment: () => void
}) {
  const prompt = gridPrompt(options.question, options.attachment, options.scope, options.now)
  options.setDraft(prompt)
  options.removeAttachment()
  return prompt
}
