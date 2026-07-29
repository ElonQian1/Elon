import type { SourcePreviewSaveState } from '../source-preview/types'
import type { RuntimeDraftStatus } from '../live/runtimeDraftModel'
import type { useLiveUiSession } from '../live/useLiveUiSession'
import styles from './UiDesignProgressBar.module.css'

type ProgressState = 'waiting' | 'active' | 'done' | 'error'

interface ProgressStep { label: string; detail: string; state: ProgressState }

export function AndroidUiDesignProgress({
  liveUi,
  draftStatus,
}: {
  liveUi: ReturnType<typeof useLiveUiSession>
  draftStatus: RuntimeDraftStatus
}) {
  const hasChanges = (liveUi.session?.historyCount ?? 0) > 0
  const buildStatus = liveUi.buildVerifyResult?.status
  const steps: ProgressStep[] = [
    {
      label: '草稿',
      detail: hasChanges || draftStatus !== 'confirmed' ? '已有设计调整' : '等待调整',
      state: draftStatus === 'rejected' ? 'error' : hasChanges || draftStatus !== 'confirmed' ? 'done' : 'waiting',
    },
    {
      label: 'Android 同步',
      detail: syncDetail(liveUi.state, draftStatus),
      state: syncState(liveUi.state, draftStatus),
    },
    {
      label: '源码写回',
      detail: liveUi.commitResult ? '已保存源码' : hasChanges ? '等待确认写回' : '尚无修改',
      state: liveUi.commitResult ? 'done' : hasChanges ? 'active' : 'waiting',
    },
    {
      label: '构建验证',
      detail: buildStatus ? verifyLabel(buildStatus) : liveUi.commitResult ? '等待无 Patch 验证' : '尚未开始',
      state: buildStatus === 'BUILD_VERIFIED' ? 'done' : buildStatus ? 'error' : liveUi.commitResult ? 'active' : 'waiting',
    },
  ]
  return <UiDesignProgressBar steps={steps} />
}

export function SourceUiDesignProgress({
  hasDocument,
  pendingCount,
  saveState,
}: {
  hasDocument: boolean
  pendingCount: number
  saveState: SourcePreviewSaveState
}) {
  return <UiDesignProgressBar compact steps={[
    { label: '草稿', detail: pendingCount ? `${pendingCount} 个组件已调整` : hasDocument ? '可即时编辑' : '等待加载源码', state: pendingCount ? 'active' : hasDocument ? 'done' : 'waiting' },
    { label: 'Android 同步', detail: '等待真帧校准', state: 'waiting' },
    { label: '源码写回', detail: saveState === 'saved' ? '源码已保存' : saveState === 'saving' ? '正在写回' : saveState === 'error' ? '写回失败' : pendingCount ? '等待确认' : '尚无修改', state: saveState === 'saved' ? 'done' : saveState === 'error' ? 'error' : saveState === 'saving' || pendingCount ? 'active' : 'waiting' },
    { label: '构建验证', detail: saveState === 'saved' ? '等待 Android 验证' : '尚未开始', state: saveState === 'saved' ? 'active' : 'waiting' },
  ]} />
}

function UiDesignProgressBar({ steps, compact = false }: { steps: ProgressStep[]; compact?: boolean }) {
  return (
    <ol className={styles.progress} aria-label="设计落地进度">
      {steps.map((step, index) => (
        <li
          key={step.label}
          data-state={step.state}
          aria-current={step.state === 'active' ? 'step' : undefined}
          title={`${step.label}：${step.detail}`}
        >
          <span className={styles.marker}>{step.state === 'done' ? '✓' : index + 1}</span>
          <span><strong>{step.label}</strong>{!compact && <small>{step.detail}</small>}</span>
        </li>
      ))}
    </ol>
  )
}

function syncState(connection: ReturnType<typeof useLiveUiSession>['state'], draft: RuntimeDraftStatus): ProgressState {
  if (draft === 'rejected') return 'error'
  if (draft === 'syncing' || draft === 'calibrating') return 'active'
  return connection === 'connected' ? 'done' : connection === 'connecting' ? 'active' : 'waiting'
}

function syncDetail(connection: ReturnType<typeof useLiveUiSession>['state'], draft: RuntimeDraftStatus) {
  if (draft === 'rejected') return 'Android 拒绝修改'
  if (draft === 'syncing') return '正在发送 Patch'
  if (draft === 'calibrating') return '等待真帧校准'
  return connection === 'connected' ? 'Runtime 已连接' : connection === 'connecting' ? '正在连接' : '等待连接'
}

function verifyLabel(status: NonNullable<ReturnType<typeof useLiveUiSession>['buildVerifyResult']>['status']) {
  if (status === 'BUILD_VERIFIED') return '无 Patch 验证通过'
  if (status === 'SOURCE_MISMATCH') return '源码与草稿不一致'
  if (status === 'TARGET_MISMATCH') return '与设计图仍有差异'
  return '未配置目标图'
}
