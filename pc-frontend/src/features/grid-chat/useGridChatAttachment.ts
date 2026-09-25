import { useEffect, useRef, useState } from 'react'
import { isExchangeWebviewAvailable, openExchangeWebSession } from '../exchange-webview/exchangeWebviewApi'
import type useLocalAiWebChatController from '../user-browser/useLocalAiWebChatController'
import { gridReadPort, readGridAttachment, readGridSelection } from './readGridChatSnapshot'
import type { GridAttachment, GridSelection } from './gridChatSnapshot'
import { prepareGridChatSend } from './gridChatSend'

type ChatController = ReturnType<typeof useLocalAiWebChatController>
interface Options { enabled: boolean; ownerKey: string; controller: ChatController }
export default function useGridChatAttachment({ enabled, ownerKey, controller }: Options) {
  const available = enabled && Boolean(ownerKey) && isExchangeWebviewAvailable()
  const targetReady = available && controller.sessionState?.contextReady === true
    && !controller.newConversationRecoveryActive
  const scope = targetReady ? JSON.stringify([ownerKey, controller.sessionIdentity,
    controller.sessionState?.windowLabel, controller.sessionState?.activeConversationId ?? '']) : ''
  const currentScope = useRef(scope)
  currentScope.current = scope
  const operation = useRef(0)
  const attachmentRef = useRef<GridAttachment | null>(null)
  const sendFlight = useRef(false)
  const [attachment, setAttachment] = useState<GridAttachment | null>(null)
  const [selection, setSelection] = useState<GridSelection | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  function remove() {
    operation.current++
    attachmentRef.current = null
    setAttachment(null)
    setSelection(null)
    setBusy(false)
    setError('')
  }
  useEffect(() => {
    remove()
    return () => { operation.current++ }
  }, [scope])

  async function read(id?: string) {
    if (!targetReady || !scope || busy || controller.busyAction) return
    const selected = selection
    const sequence = ++operation.current
    const active = () => operation.current === sequence && currentScope.current === scope
    attachmentRef.current = null
    setAttachment(null)
    setBusy(true)
    setError('')
    if (!id) setSelection(null)
    try {
      const port = gridReadPort(ownerKey)
      if (id && selected) {
        const next = await readGridAttachment(port, selected, id, scope, active)
        if (active()) {
          attachmentRef.current = next
          setAttachment(next)
          setSelection(null)
        }
      } else {
        const next = await readGridSelection(port, active)
        if (active()) setSelection(next)
      }
    } catch (reason) {
      if (active()) setError(reason instanceof Error ? reason.message : '读取网格失败，请检查币安官网连接。')
    } finally {
      if (active()) setBusy(false)
    }
  }
  async function openBinance() {
    if (!available || busy) return
    try { await openExchangeWebSession('binance', ownerKey) }
    catch { setError('无法打开币安官网，请检查 Win 客户端连接。') }
  }
  const run: ChatController['run'] = async (action, value, expectedDraft) => {
    if (action !== 'send_prompt') {
      if (['new_conversation', 'open_conversation', 'open_project'].includes(action)) remove()
      return controller.run(action, value, expectedDraft)
    }
    if (sendFlight.current) return null
    // A list or detail read must finish (or be canceled) before the user sends.
    if (busy || selection) {
      setError('请先完成网格选择，或取消附带后发送。')
      return null
    }
    const pending = attachmentRef.current
    if (!pending) return controller.run(action, value, expectedDraft)
    if (!targetReady || !controller.canSubmitDraft || controller.busyAction) {
      setError('请等待当前 ChatGPT 会话就绪后发送。')
      return null
    }
    let prompt: string
    try {
      prompt = prepareGridChatSend({ question: value ?? '', attachment: pending, scope: currentScope.current,
        setDraft: controller.setDraft, removeAttachment: remove })
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '无法准备网格附件。')
      return null
    }
    sendFlight.current = true
    try { return await controller.run(action, prompt, expectedDraft) }
    finally { sendFlight.current = false }
  }
  return { available, canRead: targetReady && !controller.busyAction, attachment, selection, busy, error,
    read, remove, openBinance, run }
}
export type GridChatAttachmentController = ReturnType<typeof useGridChatAttachment>
