import { Bot, MousePointer2, Smartphone } from 'lucide-react'
import type { PwaDesignSession } from './usePwaDesignSession'
import styles from './SourcePreview.module.css'

interface Props {
  session: PwaDesignSession
}

type StepState = 'done' | 'active' | 'pending'

interface DesignerStep {
  key: string
  title: string
  hint: string
  state: StepState
}

function workflowSteps(session: PwaDesignSession): DesignerStep[] {
  const changedCount = Object.keys(session.draft?.elements ?? {}).length
  return [
    {
      key: 'interact',
      title: '1. 先像手机一样操作',
      hint: session.ready ? '登录、点击、滚动到目标页面。' : '等待 PWA 页面连接。',
      state: session.ready ? 'done' : 'active',
    },
    {
      key: 'select',
      title: '2. 选择要改的组件',
      hint: session.selection ? '已选中组件，右侧会显示可调样式。' : '点击“选择组件”，再点页面里的元素。',
      state: session.selection ? 'done' : session.ready ? 'active' : 'pending',
    },
    {
      key: 'tune',
      title: '3. 手动实时微调',
      hint: changedCount ? `已有 ${changedCount} 个元素进入草稿。` : '改大小、圆角、字体、颜色，PWA 会即时重绘。',
      state: changedCount ? 'done' : session.selection ? 'active' : 'pending',
    },
    {
      key: 'sync',
      title: '4. 交给 AI 写回双端',
      hint: changedCount ? '生成低 Token 草稿，写回 APK 与 PWA 后再验证。' : '有草稿后才需要写回源码。',
      state: changedCount ? 'active' : 'pending',
    },
  ]
}

function nextActionLabel(session: PwaDesignSession): string {
  if (!session.ready) return '等待 PWA 连接'
  if (!session.selection && session.mode !== 'select') return '选择组件'
  if (session.mode === 'select') return '返回操作页面'
  if (Object.keys(session.draft?.elements ?? {}).length) return '写回源码并验证'
  return '先用下方样式面板微调'
}

export function PwaDesignerWorkflowGuide({ session }: Props) {
  const changedCount = Object.keys(session.draft?.elements ?? {}).length
  const canSync = changedCount > 0 && session.syncState.phase !== 'BUILD_VERIFYING' && !session.syncState.taskId
  const action = nextActionLabel(session)
  return (
    <section className={styles.pwaDesignerWorkflowCard} aria-label="PWA 设计师三步流程">
      <div className={styles.pwaDesignerWorkflowHeader}>
        <div>
          <strong>新手按这个顺序做</strong>
          <span>真实 PWA 交互 → 选择组件 → 手动微调 → AI 低 Token 写回 APK/PWA。</span>
        </div>
        <button
          type="button"
          disabled={!session.ready || (!canSync && Boolean(session.selection) && session.mode !== 'select')}
          onClick={() => {
            if (!session.ready) return
            if (!session.selection && session.mode !== 'select') {
              session.setMode('select')
              return
            }
            if (session.mode === 'select') {
              session.setMode('interact')
              return
            }
            if (canSync) void session.syncNow()
          }}
        >
          {canSync ? <Bot size={14} /> : session.mode === 'select' ? <Smartphone size={14} /> : <MousePointer2 size={14} />}
          {action}
        </button>
      </div>
      <ol className={styles.pwaDesignerWorkflowSteps}>
        {workflowSteps(session).map((step) => (
          <li key={step.key} data-step-state={step.state}>
            <strong>{step.title}</strong>
            <span>{step.hint}</span>
          </li>
        ))}
      </ol>
    </section>
  )
}
