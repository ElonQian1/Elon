import { useEffect, useRef } from 'react'
import { useAuthStore } from '../../store/auth'
import { selectedAgentForRuntimeRoute } from '../models/routeModelPolicy'
import { useModelStore } from '../models/useModelStore'
import { DEFAULT_PROJECT_RUNTIME_ROUTE } from './runtimeRoutes'
import {
  initialProjectPrewarmFromStorage,
  PROJECT_PREWARM_COOLDOWN_MS,
  requestProjectPrewarm,
} from './projectPrewarm'
import type { Project } from './types'
import { useProjectStore } from './useProjectStore'

const OPEN_PREWARM_MAX_PROJECTS = 2
const OPEN_PREWARM_STORAGE_PREFIX = 'elon.pc.openProjectPrewarm.v1'
const OPEN_PREWARM_CONVERSATION_ID = 'default'

export function useProjectOpenPrewarm(enabled = true): void {
  const token = useAuthStore((state) => state.token)
  const userId = useAuthStore((state) => state.user?.id)
  const projects = useProjectStore((state) => state.projects)
  const projectsLoaded = useProjectStore((state) => state.projectsLoaded)
  const activeProjectId = useProjectStore((state) => state.activeProjectId)
  const loadProjects = useProjectStore((state) => state.loadProjects)
  const selectedAgent = useModelStore((state) => state.selectedAgent)
  const modelOptions = useModelStore((state) => state.options)
  const startedRef = useRef<Map<string, number>>(new Map())

  useEffect(() => {
    if (!enabled || !token || !userId || projectsLoaded) return
    loadProjects().catch((err: { status?: number; message?: string }) => {
      if (err?.status !== 401) {
        console.warn('[ProjectOpenPrewarm] load projects failed:', err?.message)
      }
    })
  }, [enabled, token, userId, projectsLoaded, loadProjects])

  useEffect(() => {
    if (!enabled || !userId || !projectsLoaded || projects.length === 0) return
    if (!initialProjectPrewarmFromStorage(window.localStorage)) return

    const now = Date.now()
    const agent = selectedAgentForRuntimeRoute(
      selectedAgent,
      modelOptions,
      DEFAULT_PROJECT_RUNTIME_ROUTE,
    )
    const timers: number[] = []

    for (const [index, project] of openPrewarmProjects(projects, activeProjectId).entries()) {
      const key = openPrewarmKey(userId, project.id, agent)
      const lastStartedAt = Math.max(
        startedRef.current.get(key) ?? 0,
        readStoredOpenPrewarmAt(window.localStorage, key),
      )
      if (now - lastStartedAt < PROJECT_PREWARM_COOLDOWN_MS) continue

      startedRef.current.set(key, now)
      writeStoredOpenPrewarmAt(window.localStorage, key, now)
      const timer = window.setTimeout(() => {
        requestProjectPrewarm(project.id, {
          conversation_id: OPEN_PREWARM_CONVERSATION_ID,
          agent: agent || undefined,
          trace_id: `pc_open_prewarm:${project.id}:${now}`,
        }).catch((err: { status?: number; message?: string }) => {
          console.warn('[ProjectOpenPrewarm] failed:', project.id, err?.status, err?.message)
        })
      }, index * 750)
      timers.push(timer)
    }

    return () => {
      for (const timer of timers) window.clearTimeout(timer)
    }
  }, [enabled, userId, projectsLoaded, projects, activeProjectId, selectedAgent, modelOptions])
}

function openPrewarmProjects(projects: Project[], activeProjectId: string): Project[] {
  const selected: Project[] = []
  const seen = new Set<string>()
  const push = (project: Project | undefined) => {
    if (!project?.id || seen.has(project.id)) return
    seen.add(project.id)
    selected.push(project)
  }

  push(projects.find((project) => project.id === activeProjectId))
  for (const project of projects) {
    if (selected.length >= OPEN_PREWARM_MAX_PROJECTS) break
    if (projectLikelyUsesPcRuntime(project)) push(project)
  }
  if (selected.length === 0) push(projects[0])
  return selected.slice(0, OPEN_PREWARM_MAX_PROJECTS)
}

function projectLikelyUsesPcRuntime(project: Project): boolean {
  return project.source_type === 'pc_managed'
    || project.source_type === 'local_path'
    || Boolean(project.node_id || project.workspace_path)
}

function openPrewarmKey(userId: string, projectId: string, agent: string): string {
  return `${OPEN_PREWARM_STORAGE_PREFIX}:${userId}:${projectId}:${agent || 'default'}`
}

function readStoredOpenPrewarmAt(storage: Storage, key: string): number {
  try {
    const value = Number(storage.getItem(key) ?? '0')
    return Number.isFinite(value) ? value : 0
  } catch {
    return 0
  }
}

function writeStoredOpenPrewarmAt(storage: Storage, key: string, value: number): void {
  try {
    storage.setItem(key, String(value))
  } catch {
    // Ignore storage failures; backend prewarm remains best effort.
  }
}
