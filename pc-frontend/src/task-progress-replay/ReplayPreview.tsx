import {
  AlertTriangle,
  Bug,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  Download,
  Eye,
  EyeOff,
  FileUp,
  Info,
  Pause,
  Play,
  RefreshCw,
  RotateCcw,
  SkipBack,
  SkipForward,
  Wrench,
} from 'lucide-react'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { auditReplayDom, type ReplayDomSnapshot } from './audit'
import { replayCaptures } from './captures'
import {
  buildReplayFrames,
  captureReplayIssues,
  replayFrameDelay,
  selectReplayKeyFrames,
  type ReplayIssue,
  type ReplayPreviewConfig,
} from './model'
import { ReplayConversation } from './ReplayConversation'
import { ReplayFilmstrip } from './ReplayFilmstrip'
import { useTaskReplayCapture } from './useTaskReplayCapture'
import './task-progress-replay.css'

const speedOptions = [0.25, 1, 4]

export function ReplayPreview({ config }: { config: ReplayPreviewConfig }) {
  const { capture, loading, error, refreshedAt, live, refresh, importCapture } = useTaskReplayCapture(config)
  const frames = useMemo(() => buildReplayFrames(capture), [capture])
  const [frameIndex, setFrameIndex] = useState(() => boundedFrame(config.frame ?? 0, frames.length))
  const [playing, setPlaying] = useState(false)
  const [speed, setSpeed] = useState(config.speed)
  const [expandTools, setExpandTools] = useState(config.expandTools)
  const [showFilmstrip, setShowFilmstrip] = useState(config.filmstrip)
  const [domIssues, setDomIssues] = useState<Record<number, ReplayIssue[]>>({})
  const previousAudit = useRef<ReplayDomSnapshot>()
  const importInput = useRef<HTMLInputElement>(null)
  const frame = frames[boundedFrame(frameIndex, frames.length)] ?? frames[0]
  const keyFrames = useMemo(() => selectReplayKeyFrames(frames), [frames])
  const dataIssues = useMemo(() => captureReplayIssues(capture, frames), [capture, frames])
  const issues = useMemo(() => dedupeIssues([...dataIssues, ...Object.values(domIssues).flat()]), [dataIssues, domIssues])

  useEffect(() => {
    setPlaying(false)
    setFrameIndex(boundedFrame(config.frame ?? 0, frames.length))
    setDomIssues({})
    previousAudit.current = undefined
  }, [capture.id, config.frame, frames.length])

  useEffect(() => {
    if (!playing || !frame || frame.index >= frames.length - 1) {
      if (playing && frame?.index === frames.length - 1) setPlaying(false)
      return
    }
    const next = frames[frame.index + 1]
    const timer = window.setTimeout(() => setFrameIndex(next.index), replayFrameDelay(frame, next, speed))
    return () => window.clearTimeout(timer)
  }, [frame, frames, playing, speed])

  const selectFrame = useCallback((nextFrame: number) => {
    const bounded = boundedFrame(nextFrame, frames.length)
    setFrameIndex(bounded)
    updateReplayUrl('frame', String(bounded))
  }, [frames.length])

  const handleRendered = useCallback((root: HTMLElement) => {
    const auditedFrame = frame.index
    window.requestAnimationFrame(() => window.requestAnimationFrame(() => {
      const result = auditReplayDom(root, auditedFrame, previousAudit.current)
      previousAudit.current = result.snapshot
      setDomIssues((current) => {
        if (JSON.stringify(current[auditedFrame] ?? []) === JSON.stringify(result.issues)) return current
        return { ...current, [auditedFrame]: result.issues }
      })
    }))
  }, [frame.index])

  const handleImport = useCallback(async (file: File | undefined) => {
    if (!file) return
    try {
      importCapture(JSON.parse(await file.text()))
    } catch (importError) {
      window.alert((importError as { message?: string }).message || '无法导入回放文件。')
    }
  }, [importCapture])

  return (
    <main className="replayWorkbenchPage">
      <aside className="replayWorkbenchRail">
        <div className="replayWorkbenchTitle">
          <strong>任务逐帧回放</strong>
          <span>{live ? '真实快照' : capture.source === 'import' ? '导入记录' : '黄金记录'}</span>
        </div>
        <label className="replayCaptureSelect">
          <span>记录</span>
          <select value={live || capture.source === 'import' ? '' : capture.id} onChange={(event) => switchGoldenCapture(event.target.value)}>
            {(live || capture.source === 'import') && <option value="">{capture.title}</option>}
            {replayCaptures.map((item) => <option value={item.id} key={item.id}>{item.title}</option>)}
          </select>
        </label>
        <dl className="replayFacts">
          <div><dt>任务</dt><dd title={capture.taskId}>{shortId(capture.taskId)}</dd></div>
          <div><dt>状态</dt><dd data-tone={capture.taskStatus}>{capture.taskStatus}</dd></div>
          <div><dt>消息</dt><dd>{capture.messages.length}</dd></div>
          <div><dt>事件</dt><dd>{capture.events.length}</dd></div>
          <div><dt>总时长</dt><dd>{formatTime(frames[frames.length - 1]?.atMs ?? 0)}</dd></div>
          {refreshedAt > 0 && <div><dt>刷新</dt><dd>{new Date(refreshedAt).toLocaleTimeString('zh-CN', { hour12: false })}</dd></div>}
        </dl>
        <div className="replayFileActions">
          {live && <IconButton title="重新录制快照" onClick={() => void refresh()} disabled={loading}><RefreshCw size={15} /></IconButton>}
          <IconButton title="导入回放 JSON" onClick={() => importInput.current?.click()}><FileUp size={15} /></IconButton>
          <IconButton title="导出回放 JSON" onClick={() => exportCapture(capture)}><Download size={15} /></IconButton>
          <input ref={importInput} type="file" accept="application/json,.json" hidden onChange={(event) => void handleImport(event.target.files?.[0])} />
        </div>
        <IssueSummary issues={issues} />
      </aside>

      <section className="replayWorkbenchStage">
        <header className="replayWorkbenchHeader">
          <div>
            <strong>{capture.title}</strong>
            <span>{frame.title}</span>
          </div>
          <span className="replayFrameCounter">{frame.index} / {frames.length - 1}</span>
        </header>

        <div className="replayToolbar" role="toolbar" aria-label="回放控制">
          <div className="replayTransport">
            <IconButton title="回到开始" onClick={() => selectFrame(0)} disabled={frame.index === 0}><SkipBack size={15} /></IconButton>
            <IconButton title="上一帧" onClick={() => selectFrame(frame.index - 1)} disabled={frame.index === 0}><ChevronLeft size={17} /></IconButton>
            <IconButton title={playing ? '暂停回放' : '播放回放'} primary onClick={() => setPlaying((value) => !value)}>
              {playing ? <Pause size={15} /> : <Play size={15} />}
            </IconButton>
            <IconButton title="下一帧" onClick={() => selectFrame(frame.index + 1)} disabled={frame.index >= frames.length - 1}><ChevronRight size={17} /></IconButton>
            <IconButton title="跳到结束" onClick={() => selectFrame(frames.length - 1)} disabled={frame.index >= frames.length - 1}><SkipForward size={15} /></IconButton>
          </div>
          <input
            className="replayRange"
            type="range"
            min="0"
            max={Math.max(0, frames.length - 1)}
            value={frame.index}
            aria-label="选择回放帧"
            onChange={(event) => selectFrame(Number(event.target.value))}
          />
          <div className="replaySpeed" aria-label="播放速度">
            {speedOptions.map((option) => (
              <button type="button" key={option} data-active={speed === option ? 'true' : undefined} onClick={() => setSpeed(option)}>{option}×</button>
            ))}
          </div>
          <IconButton title={expandTools ? '收起工具详情' : '展开工具详情'} onClick={() => setExpandTools((value) => !value)} active={expandTools}>
            <Wrench size={15} />
          </IconButton>
          <IconButton title={showFilmstrip ? '隐藏关键帧' : '显示关键帧'} onClick={() => setShowFilmstrip((value) => !value)} active={showFilmstrip}>
            {showFilmstrip ? <EyeOff size={15} /> : <Eye size={15} />}
          </IconButton>
          <IconButton title="重新播放" onClick={() => { selectFrame(0); setPlaying(true) }}><RotateCcw size={15} /></IconButton>
        </div>

        <div className="replayTimelineMeta">
          <span>+{formatTime(frame.atMs)}</span>
          <span>{frame.messageCount} 条消息</span>
          <span>{frame.eventCount} 个事件</span>
          <span data-change={frame.visualChange ? 'true' : undefined}>{frame.visualChange ? '画面变化' : '仅原始事件'}</span>
        </div>

        <article className="replayViewport" aria-busy={loading}>
          <div className="replayTopbar">
            <strong>AI 开发频道</strong>
            <span>{error || (loading ? '正在读取快照' : frame.title)}</span>
          </div>
          {error ? <div className="replayLoadError">{error}</div> : <ReplayConversation capture={capture} frame={frame} expandTools={expandTools} onRendered={handleRendered} />}
        </article>

        {showFilmstrip && (
          <ReplayFilmstrip capture={capture} frames={keyFrames} activeFrame={frame.index} expandTools={expandTools} onSelect={selectFrame} />
        )}

        <details className="replayInspector">
          <summary><Bug size={14} />原始帧</summary>
          <pre>{JSON.stringify(frame.source === 'event' ? capture.events[frame.sourceIndex] : frame.source === 'message' ? capture.messages[frame.sourceIndex] : { startedAt: capture.startedAt }, null, 2)}</pre>
        </details>
      </section>
    </main>
  )
}

function IssueSummary({ issues }: { issues: ReplayIssue[] }) {
  const errors = issues.filter((item) => item.severity === 'error').length
  const warnings = issues.filter((item) => item.severity === 'warning').length
  const infos = issues.filter((item) => item.severity === 'info').length
  return (
    <section className="replayIssues" data-clear={issues.length === 0 ? 'true' : undefined}>
      <header>
        {errors > 0 ? <AlertTriangle size={15} /> : warnings > 0 ? <Info size={15} /> : <CheckCircle2 size={15} />}
        <strong>{issues.length === 0 ? '检查通过' : `${errors} 错误 · ${warnings} 警告 · ${infos} 提示`}</strong>
      </header>
      {issues.slice(0, 6).map((item) => (
        <div key={item.id} data-severity={item.severity} title={item.detail}>
          <span>{item.frameIndex == null ? '数据' : `帧 ${item.frameIndex}`}</span>
          <strong>{item.title}</strong>
        </div>
      ))}
    </section>
  )
}

function IconButton({ title, onClick, children, disabled = false, primary = false, active = false }: {
  title: string
  onClick: () => void
  children: React.ReactNode
  disabled?: boolean
  primary?: boolean
  active?: boolean
}) {
  return (
    <button
      type="button"
      className="replayIconButton"
      title={title}
      aria-label={title}
      data-primary={primary ? 'true' : undefined}
      data-active={active ? 'true' : undefined}
      onClick={onClick}
      disabled={disabled}
    >
      {children}
    </button>
  )
}

function boundedFrame(frame: number, length: number): number {
  return Math.max(0, Math.min(Math.max(0, length - 1), frame))
}

function formatTime(milliseconds: number): string {
  if (milliseconds < 1000) return `${milliseconds}ms`
  const seconds = milliseconds / 1000
  if (seconds < 60) return `${seconds.toFixed(seconds < 10 ? 1 : 0)}s`
  return `${Math.floor(seconds / 60)}m ${Math.round(seconds % 60)}s`
}

function shortId(value: string): string {
  return value.length > 20 ? `${value.slice(0, 12)}…${value.slice(-5)}` : value
}

function switchGoldenCapture(captureId: string) {
  if (!captureId) return
  const url = new URL(window.location.href)
  url.searchParams.set('source', 'replay')
  url.searchParams.set('capture', captureId)
  for (const key of ['task', 'taskId', 'channel', 'channelId', 'conversation', 'conversationId', 'frame']) url.searchParams.delete(key)
  window.location.assign(`${url.pathname}${url.search}${url.hash}`)
}

function updateReplayUrl(key: string, value: string) {
  const url = new URL(window.location.href)
  url.searchParams.set(key, value)
  window.history.replaceState(null, '', `${url.pathname}${url.search}${url.hash}`)
}

function exportCapture(capture: unknown) {
  const blob = new Blob([JSON.stringify(capture, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `task-replay-${(capture as { id?: string }).id || 'capture'}.json`
  link.click()
  URL.revokeObjectURL(url)
}

function dedupeIssues(issues: ReplayIssue[]): ReplayIssue[] {
  const seen = new Set<string>()
  return issues.filter((issue) => {
    const key = `${issue.id}|${issue.frameIndex ?? ''}`
    if (seen.has(key)) return false
    seen.add(key)
    return true
  })
}
