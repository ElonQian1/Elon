import type { TaskTone } from './types'

export interface TaskStageActionModel {
  canContinue: boolean
  canOpenNode: boolean
  continueLabel: string
}

const NO_ACTION: TaskStageActionModel = {
  canContinue: false,
  canOpenNode: false,
  continueLabel: '',
}

export function taskStageActionModel(
  stageKey: string,
  tone: TaskTone,
  stuck: boolean,
): TaskStageActionModel {
  if (stageKey === 'approval' || stageKey === 'finished' || tone === 'done' || tone === 'canceled') return NO_ACTION
  if (stageKey === 'resume-required') {
    return { canContinue: true, canOpenNode: true, continueLabel: '继续任务' }
  }
  if (stageKey === 'recovery-timeout') {
    return { canContinue: true, canOpenNode: true, continueLabel: '重试恢复' }
  }
  if (stageKey === 'timeout' || stageKey === 'tool-timeout') {
    return { canContinue: true, canOpenNode: true, continueLabel: '重试任务' }
  }
  if (stageKey === 'heartbeat' && stuck) {
    return { canContinue: true, canOpenNode: true, continueLabel: '检查并继续' }
  }
  return NO_ACTION
}
