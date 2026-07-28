import {
  normalizePwaRoute,
  type PwaRouteIdentity,
} from './pwaDesignDraft'

export interface PwaRouteState extends PwaRouteIdentity {
  href: string
  title: string
  viewport: PwaRouteIdentity['viewport'] & {
    deviceScaleFactor?: number
    visualWidth?: number
    visualHeight?: number
    pointer?: 'coarse' | 'fine' | 'none'
  }
  scroll?: { x: number; y: number }
}

export function mergePwaRouteState(payload: PwaRouteState): PwaRouteState {
  const normalized = normalizePwaRoute(payload)
  return {
    ...payload,
    ...normalized,
    viewport: {
      ...payload.viewport,
      ...normalized.viewport,
    },
  }
}
