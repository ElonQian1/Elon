import type { ReplayCapture, ReplayFrame } from './model'
import { ReplayConversation } from './ReplayConversation'

export function ReplayFilmstrip({ capture, frames, activeFrame, expandTools, onSelect }: {
  capture: ReplayCapture
  frames: ReplayFrame[]
  activeFrame: number
  expandTools: boolean
  onSelect: (frame: number) => void
}) {
  return (
    <section className="replayFilmstrip" aria-label="关键帧胶片">
      <header>
        <strong>关键帧</strong>
        <span>{frames.length} 帧</span>
      </header>
      <div className="replayFilmstripTrack">
        {frames.map((frame) => (
          <div
            className="replayFilmstripFrame"
            data-active={activeFrame === frame.index ? 'true' : undefined}
            key={frame.index}
            role="button"
            tabIndex={0}
            onClick={() => onSelect(frame.index)}
            onKeyDown={(event) => {
              if (event.key !== 'Enter' && event.key !== ' ') return
              event.preventDefault()
              onSelect(frame.index)
            }}
            title={`跳到第 ${frame.index} 帧：${frame.title}`}
          >
            <span className="replayFilmstripMeta">
              <strong>{frame.index}</strong>
              <span>+{formatFrameTime(frame.atMs)}</span>
            </span>
            <span className="replayFilmstripViewport">
              <span className="replayFilmstripScale">
                <ReplayConversation capture={capture} frame={frame} expandTools={expandTools} compact />
              </span>
            </span>
            <span className="replayFilmstripTitle">{frame.title}</span>
          </div>
        ))}
      </div>
    </section>
  )
}

function formatFrameTime(milliseconds: number): string {
  if (milliseconds < 1000) return `${milliseconds}ms`
  return `${(milliseconds / 1000).toFixed(milliseconds < 10000 ? 1 : 0)}s`
}
