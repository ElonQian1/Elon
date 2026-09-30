import mainManifest from '../../../../.elon/project-landing.json'
import type { ProjectLanding } from './types'

/** Ship the platform's own introduction with its frontend release, avoiding stale node snapshots. */
export function resolveProjectLandingContent(projectId: string, landing: ProjectLanding | null): ProjectLanding | null {
  if (projectId !== 'elon-self') return landing
  return {
    ...landing,
    ...mainManifest,
    downloads: Object.entries(mainManifest.downloads).map(([platform, value]) => ({ ...value, platform })),
    source: { mode: 'bundled_main_project', status: 'available' },
  }
}
