export const PROJECT_PLAZA_CARD_MIN_SCALE = 0.9

export function projectPlazaCardScale(
  centerDistancePx,
  snapDistancePx,
  minimumScale = PROJECT_PLAZA_CARD_MIN_SCALE,
) {
  const safeMinimum = Math.min(1, Math.max(0, minimumScale))
  const normalizedDistance = snapDistancePx > 0
    ? Math.min(1, Math.abs(centerDistancePx) / snapDistancePx)
    : centerDistancePx === 0 ? 0 : 1
  return 1 - ((1 - safeMinimum) * normalizedDistance)
}

export function projectPlazaCardScales(
  cardCentersPx,
  previewCenterPx,
  snapDistancePx,
  minimumScale = PROJECT_PLAZA_CARD_MIN_SCALE,
) {
  return cardCentersPx.map((center) => (
    projectPlazaCardScale(center - previewCenterPx, snapDistancePx, minimumScale)
  ))
}
