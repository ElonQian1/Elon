import { useSyncExternalStore } from 'react'
import { v4 as uuidv4 } from 'uuid'
import { GroupAiTask, type GroupAiInput } from './groupAiTask'
import { createGroupAiPort } from './groupAiPort'
import { useAuthStore } from '../../../store/auth'

let task: GroupAiTask | null = null
let revision = 0
const listeners = new Set<() => void>()
function changed() { revision++; listeners.forEach(listener => listener()) }
useAuthStore.subscribe(state => {
  if (task && task.input.owner !== state.user?.id) {
    const previous = task
    task = null; changed(); void previous.cancel()
  }
})
export function getGroupAiTask() { return task }
export function startGroupAi(input: GroupAiInput, operation = uuidv4()) {
  if (task?.operation === operation) {
    if (JSON.stringify(task.input) !== JSON.stringify(input)) throw new Error('群聊任务标识已用于不同请求')
    return task
  }
  if (task && !['completed', 'cancelled'].includes(task.progress.phase)) throw new Error('已有群聊 AI 请求，请先处理或停止已有任务')
  task = new GroupAiTask(operation, input, createGroupAiPort(input.owner), changed)
  changed(); void task.start()
  return task
}
export function useGroupAiTask() {
  useSyncExternalStore(listener => { listeners.add(listener); return () => { listeners.delete(listener) } }, () => revision)
  return task
}
