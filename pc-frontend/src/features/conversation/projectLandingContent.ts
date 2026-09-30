import type { ProjectLanding } from './types'

/** Project-space API owns introduction content for PC, APK and PWA. */
export function resolveProjectLandingContent(_projectId: string, landing: ProjectLanding | null): ProjectLanding | null {
  return landing
}
