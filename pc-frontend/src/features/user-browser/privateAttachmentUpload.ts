import { v4 as uuidv4 } from 'uuid'
import type { LocalAiWebSessionState } from './localAiBrowserApi'

export interface PrivateUploadFile {
  name: string; type: string; size: number; sha256?: string | null
  load(): Promise<Blob>
}
export interface PrivateUploadPort {
  command(value: string, requestId: string): Promise<unknown>
  state(): Promise<LocalAiWebSessionState>
  check(): void
}
let sequence = 0
export const privateUploadDiagnostic = /^private_upload_rspack_(build_unreviewed|role_pending|contract_mismatch|loader_mismatch|runtime_pending|composer_detached|composer_ambiguous|route_unsupported|document_unavailable|owner_ambiguous|owner_pending|identity_pending|identity_unavailable|conversation_mismatch|mode_unsupported|busy|attachments_or_tools_present|composer_state_pending|upload_timeout|upload_unconfirmed|upload_cancelled|upload_context_changed|upload_association_unconfirmed|upload_transaction_failed)$/
const requestId = () => `mcp_att${Date.now().toString(36)}${(++sequence).toString(36)}`
const wait = (ms: number) => new Promise(resolve => setTimeout(resolve, ms))

/** Only a confirmed private association permits the caller to send its text. */
export async function uploadPrivateAttachments(files: PrivateUploadFile[], port: PrivateUploadPort) {
  if (!files.length || files.length > 9 || files.some(f => f.size < 1 || f.size > 8388608)) throw new Error('请选择 1 至 9 个附件，每个不超过 8 MB')
  const batchId = uuidv4(), leases = files.map(() => uuidv4())
  let stage = 'begin'
  const failure = (kind: string, message: string) => Object.assign(new Error(message), { code: `private_upload_${stage}_${kind}` })
  async function send(payload: object) {
    port.check()
    const id = requestId()
    await port.command(JSON.stringify({ ...payload, batchId }), id)
    port.check(); return id
  }
  async function confirmed(id: string, action: string, timeout: number) {
    const end = Date.now() + timeout
    while (Date.now() < end) {
      port.check()
      const state = await port.state()
      port.check()
      const receipt = [state.commandResult, ...(state.commandResults || [])].find(r => r?.requestId === id && r.action === action)
      if (receipt) {
        if (!receipt.ok || action === 'request_attachment_upload' && receipt.detail !== 'private_attachment_associated') {
          const error = failure('rejected', '附件尚未完整上传到 ChatGPT，未发送文字。请检查文件和网络后重试')
          if (privateUploadDiagnostic.test(receipt.detail || '')) error.code = receipt.detail!
          throw error
        }
        return
      }
      await wait(150)
    }
    throw failure('timeout', '附件上传未确认，未发送文字；请检查网络后重新选择')
  }
  try {
    const id = await send({ step: 'begin', files: files.map((f, i) => ({ leaseId: leases[i], name: f.name, type: f.type, size: f.size, sha256: f.sha256 })) })
    await confirmed(id, 'stage_attachments', 5000)
    for (let index = 0; index < files.length; index++) {
      port.check()
      stage = 'read'
      const blob = await files[index].load()
      if (blob.size !== files[index].size) throw new Error('附件已变化，未发送文字')
      stage = 'chunk'
      for (let offset = 0; offset < blob.size; offset += 65536) {
        port.check()
        const bytes = new Uint8Array(await blob.slice(offset, offset + 65536).arrayBuffer())
        let binary = ''
        for (const byte of bytes) binary += String.fromCharCode(byte)
        await send({ step: 'chunk', leaseId: leases[index], offset, data: btoa(binary) })
      }
    }
    stage = 'associate'
    await confirmed(await send({ step: 'upload' }), 'request_attachment_upload', 120000)
  } catch (error) {
    try { port.check(); await send({ step: 'cancel' }) } catch { /* Closed/navigated hosts expire their own file leases. */ }
    if (error && typeof error === 'object' && 'code' in error && (/^private_upload_(begin|read|chunk|associate)_(failed|rejected|timeout)$/.test(String(error.code)) || privateUploadDiagnostic.test(String(error.code)))) throw error
    throw failure('failed', error instanceof Error ? error.message : '附件操作失败，未发送文字')
  }
}
