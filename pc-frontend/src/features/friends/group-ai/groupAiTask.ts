import type { LocalAiMessageSnapshot, LocalAiWebSessionState } from '../../user-browser/localAiBrowserApi'
import type { GroupAiAttachment } from './groupAiAttachments'

export interface GroupAiRequest {
  id: string; group_id: string; trigger_message_id: string; state: string; prompt: string
  engine: string; web_provider: string; dispatch_permit: boolean; result_message_id: string | null
  attachments?: GroupAiAttachment[]
}
export interface GroupAiSelection {
  message_ids: string[]; message_revisions: Record<string, number>; question: string; allow_continue?: boolean
}
export interface GroupAiInput {
  owner: string; group: string; title: string; source: string; selection: GroupAiSelection
  provider: 'chatgpt' | 'google-ai-mode'
}
export interface GroupAiPort {
  prepare(operation: string, input: GroupAiInput): Promise<GroupAiRequest>
  action(operation: string, input: GroupAiInput, request: GroupAiRequest, action: string, content?: string): Promise<GroupAiRequest>
  host(operation: string, input: GroupAiInput, action: string, value?: string, requestId?: string): Promise<LocalAiWebSessionState>
  upload?(operation: string, input: GroupAiInput, files: GroupAiAttachment[], check: () => void): Promise<void>
  checkOwner(): void
  now(): number
  wait(ms: number): Promise<void>
}
export type GroupAiPhase = 'preparing' | 'answering' | 'delivering' | 'completed' | 'failed' | 'uncertain' | 'cancelled'
export interface GroupAiProgress { phase: GroupAiPhase; message: string; busy: boolean; hasAnswer: boolean }

let sequence = 0
export function groupAiCommandId(now = Date.now()): string {
  sequence = (sequence + 1) % 1024
  // The shared private sender accepts a positive base36 signed-long, not a UUID.
  return 'mcp_' + (now * 1024 + sequence).toString(36)
}
const normalized = (v: string) => v.trim().replace(/\s+/g, ' ')
const body = (message: LocalAiMessageSnapshot['messages'][number]) => message.content.map(part => {
  if (part.type === 'code') return '\n```' + (part.language || '') + '\n' + part.text + '\n```\n'
  return part.text || ''
}).join('\n').trim()

export function completedGroupAiReply(snapshot: LocalAiMessageSnapshot, prompt: string): string | null {
  const source = snapshot.messages.map(m => m.role === 'user' && normalized(body(m)) === normalized(prompt)).lastIndexOf(true)
  if (source < 0) return null
  const replies = snapshot.messages.slice(source + 1)
  if (replies.some(m => m.role === 'user')) return null
  const answer = [...replies].reverse().find(m => m.role === 'assistant' && body(m))
  if (!answer || answer.state !== 'completed' || (snapshot.streaming && snapshot.privateStreamState !== 'completed')) return null
  return body(answer)
}

export function groupAiDocumentReady(state: LocalAiWebSessionState, provider: string): boolean {
  const s = state.semanticEvent as LocalAiMessageSnapshot | null
  if (!s || s.type !== 'message_snapshot' || state.loading || state.semanticCacheStatus !== 'live'
    || !state.contextReady || s.loginRequired || s.streaming || s.messages.length || s.draft.trim()) return false
  try {
    const url = new URL(state.currentUrl), observed = new URL(s.url)
    if (url.origin !== observed.origin || url.pathname !== observed.pathname || url.hash || url.username || url.password) return false
    return provider === 'chatgpt'
      ? url.origin === 'https://chatgpt.com' && url.pathname === '/' && url.search === '?temporary-chat=true'
        && (s.composerReady || (s as LocalAiMessageSnapshot & { privateSendReady?: boolean }).privateSendReady === true)
      : ['https://www.google.com', 'https://google.com'].includes(url.origin) && url.pathname === '/aimode' && s.composerReady
  } catch { return false }
}

export class GroupAiTask {
  request?: GroupAiRequest
  answer = ''
  dispatched = false
  sendRequestId = ''
  stopped = false
  private attachmentAttempted = false
  progress: GroupAiProgress = { phase: 'preparing', message: '正在准备群聊 AI', busy: false, hasAnswer: false }
  constructor(readonly operation: string, readonly input: GroupAiInput, private port: GroupAiPort, private changed: () => void) {}
  private update(phase: GroupAiPhase, message: string, busy: boolean) {
    this.progress = { phase, message, busy, hasAnswer: !!this.answer }; this.changed()
  }
  private check() {
    this.port.checkOwner()
    if (this.stopped) throw new Error('已取消本次群聊分析')
  }
  private async host(action: string, value?: string, requestId?: string) {
    this.check()
    const state = await this.port.host(this.operation, this.input, action, value, requestId)
    this.check(); return state
  }
  private async action(action: string, content?: string) {
    this.check()
    if (!this.request) throw new Error('尚未准备群聊请求')
    const request = await this.port.action(this.operation, this.input, this.request, action, content)
    this.check()
    if (request.id !== this.request.id || request.group_id !== this.input.group) throw new Error('群聊请求身份不匹配')
    this.request = request; return request
  }
  async start() {
    if (this.progress.busy || this.dispatched || this.stopped) return
    this.update('preparing', '正在准备群聊 AI', true)
    try {
      this.check()
      this.request = await this.port.prepare(this.operation, this.input)
      this.check()
      if (this.request.group_id !== this.input.group || this.request.trigger_message_id !== this.input.source) throw new Error('群聊请求身份不匹配')
      if (this.request.state === 'completed') { this.update('completed', 'AI 回答已发送到群聊', false); return }
      if (this.request.state !== 'prepared') { this.dispatched = true; throw new Error('请求已经派发，请核对结果，未重复发送') }
      if (!Array.isArray(this.request.attachments)) throw new Error('服务器尚未支持附件清单，请等待服务更新；未发送给 AI')
      await this.host('open')
      const deadline = this.port.now() + 60_000
      let ready = false
      while (this.port.now() < deadline) {
        const state = await this.host('state')
        const snapshot = state.semanticEvent as LocalAiMessageSnapshot | null
        if (snapshot?.loginRequired || state.windowStatus === 'blocked') throw new Error('请先打开 AI 网页完成登录或验证，再重试')
        if (groupAiDocumentReady(state, this.input.provider)) { ready = true; break }
        await this.host('snapshot')
        await this.port.wait(800)
      }
      if (!ready) throw new Error('AI 网页连接超时，请检查网络后重试')
      if (this.request.attachments?.length) {
        if (this.input.provider !== 'chatgpt' || !this.port.upload) throw new Error('当前客户端或 AI 来源不能上传所选附件，未发送文字')
        this.update('preparing', '正在上传所选图片和文件', true)
        this.attachmentAttempted = true
        await this.port.upload(this.operation, this.input, this.request.attachments, () => this.check())
        this.check()
        await this.action('status')
      }
      if (this.input.provider === 'chatgpt') await this.preparePrivateSender()
      this.check()
      // Set before requesting permission: a lost server response is never safe to replay.
      this.dispatched = true
      const permission = await this.action('dispatch')
      if (!permission.dispatch_permit) throw new Error('请求已在处理，未重复发送')
      this.sendRequestId = groupAiCommandId()
      await this.host('send_prompt', permission.prompt, this.sendRequestId)
      await this.receiveAndDeliver()
    } catch (error) { await this.fail(error) }
    finally { if (this.stopped) await this.close() }
  }
  private async preparePrivateSender() {
    for (let attempt = 0; attempt < 3; attempt++) {
      const id = groupAiCommandId()
      await this.host('prepare', undefined, id)
      const deadline = this.port.now() + 5500
      while (this.port.now() < deadline) {
        const state = await this.host('state')
        const receipt = [state.commandResult, ...(state.commandResults || [])].find(r => r?.requestId === id && r.action === 'private_protocol_probe')
        if (receipt) {
          try {
            const detail = JSON.parse(receipt.detail)
            if (receipt.ok && detail.code === 'ready' && detail.stage === 'ready') return
          } catch { /* Keep the existing official sender if private admission is unavailable. */ }
          break
        }
        await this.port.wait(350)
      }
      await this.port.wait(500)
    }
  }
  private async receiveAndDeliver() {
    this.update('answering', 'AI 正在分析所选消息', true)
    const deadline = this.port.now() + 180_000
    while (this.port.now() < deadline) {
      const state = await this.host('state')
      const snapshot = state.semanticEvent as LocalAiMessageSnapshot | null
      if (state.semanticCacheStatus === 'live' && snapshot?.type === 'message_snapshot') {
        const answer = completedGroupAiReply(snapshot, this.request!.prompt)
        if (answer) { this.answer = answer; await this.deliver(); return }
      }
      const receipt = [state.commandResult, ...(state.commandResults || [])].find(r => r?.requestId === this.sendRequestId && r.action === 'send_prompt')
      if (receipt && !receipt.ok) throw new Error('网页尚未确认发送成功。请检查已有会话；不会自动重发问题')
      await this.host('snapshot')
      await this.port.wait(1000)
    }
    throw new Error('尚未收到完整回答。可继续检查，系统不会重复发送问题')
  }
  private async deliver() {
    this.update('delivering', '正在发送 AI 回答到群聊', true)
    const result = await this.action('complete', this.answer)
    if (result.state !== 'completed' || !result.result_message_id) throw new Error('群消息尚未确认送达')
    this.update('completed', 'AI 回答已发送到群聊', false)
    await this.close()
  }
  async resume() {
    if (this.progress.busy || this.stopped) return
    if (!this.dispatched) { await this.start(); return }
    this.update('answering', '正在核对已有请求', true)
    try {
      const state = await this.action('status')
      if (state.state === 'completed') { this.update('completed', 'AI 回答已发送到群聊', false); await this.close(); return }
      if (this.answer) await this.deliver()
      else await this.receiveAndDeliver()
    } catch (error) { await this.fail(error) }
  }
  async show() { await this.host('show') }
  async cancel() {
    if (this.progress.phase === 'completed') return
    this.stopped = true
    this.update('cancelled', '已停止继续处理；已经发出的群消息不会撤回', false)
    if (this.request) {
      try { this.port.checkOwner(); await this.port.action(this.operation, this.input, this.request, this.dispatched ? 'uncertain' : 'cancel') } catch { /* No replay. */ }
    }
    await this.close()
  }
  private async close() {
    try { await this.port.host(this.operation, this.input, 'close') } catch { /* Host may already be closed. */ }
  }
  private async fail(error: unknown) {
    if (this.stopped) return
    try { this.port.checkOwner() } catch { await this.cancel(); return }
    const message = error instanceof Error ? error.message : '群聊 AI 处理失败'
    // A retry must not append to files already associated with the previous draft.
    if (!this.dispatched && this.attachmentAttempted) { await this.close(); this.attachmentAttempted = false }
    this.update(this.dispatched ? 'uncertain' : 'failed', message, false)
    if (this.dispatched && this.request) {
      try { await this.action('uncertain') } catch { /* Preserve uncertain status locally. */ }
    }
  }
}
