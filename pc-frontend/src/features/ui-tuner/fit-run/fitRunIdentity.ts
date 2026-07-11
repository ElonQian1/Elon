import type { CreateFitRunInput, FitRunDocument, FitRunEnvironment, FitRunPairInput } from './types'

type FitRunIdentitySource = Pick<CreateFitRunInput, 'pair' | 'environment'>
  | Pick<FitRunDocument, 'pair' | 'environment'>

export function fitRunPairKey(source?: FitRunIdentitySource) {
  if (!source) return ''
  return JSON.stringify({
    targetSha256: source.pair.targetSha256,
    definitionId: source.pair.definitionId,
    instanceKey: source.pair.instanceKey ?? '',
    calibrationId: source.pair.calibrationId ?? '',
    targetRect: rectIdentity(source.pair.targetRect),
    projectedTargetRect: rectIdentity(source.pair.projectedTargetRect),
    environment: environmentIdentity(source.environment),
  })
}

export function sameFitRunPair(
  run: Pick<FitRunDocument, 'pair' | 'environment'>,
  input: FitRunIdentitySource,
) {
  return fitRunPairKey(run) === fitRunPairKey(input)
}

function rectIdentity(rect: FitRunPairInput['targetRect']) {
  return [rect.left, rect.top, rect.right, rect.bottom]
}

function environmentIdentity(environment?: FitRunEnvironment) {
  return {
    screenId: environment?.screenId ?? '',
    scenario: environment?.scenario ?? '',
    theme: environment?.theme ?? '',
    locale: environment?.locale ?? '',
    viewportWidth: environment?.viewportWidth ?? null,
    viewportHeight: environment?.viewportHeight ?? null,
    density: environment?.density ?? null,
    fontScale: environment?.fontScale ?? null,
    rotation: environment?.rotation ?? null,
    insets: Object.entries(environment?.insets ?? {}).sort(([left], [right]) => left.localeCompare(right)),
  }
}
