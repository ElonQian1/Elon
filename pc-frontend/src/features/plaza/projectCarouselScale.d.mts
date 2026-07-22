export const PROJECT_PLAZA_CARD_MIN_SCALE: number

export function projectPlazaCardScale(
  centerDistancePx: number,
  snapDistancePx: number,
  minimumScale?: number,
): number

export function projectPlazaCardScales(
  cardCentersPx: number[],
  previewCenterPx: number,
  snapDistancePx: number,
  minimumScale?: number,
): number[]
