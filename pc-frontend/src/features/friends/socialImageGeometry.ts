export type ImageSize = { width: number; height: number }
export type ImageViewport = { scale: number; x: number; y: number; reading: boolean }

export function imageScales(image: ImageSize, stage: ImageSize) {
  const fit = Math.min(1, stage.width / image.width, stage.height / image.height)
  const width = Math.min(stage.width, 1000) / image.width
  return { fit, width, max: Math.max(4, width * 4), long: image.height / image.width >= 2.5 && image.height * width > stage.height * 1.5 }
}

export function clampImage(view: ImageViewport, image: ImageSize, stage: ImageSize): ImageViewport {
  const scales = imageScales(image, stage)
  const scale = Math.max(scales.fit, Math.min(scales.max, view.scale))
  const limitX = Math.max(0, (image.width * scale - stage.width) / 2)
  const limitY = Math.max(0, (image.height * scale - stage.height) / 2)
  return { ...view, scale, x: Math.max(-limitX, Math.min(limitX, view.x)), y: Math.max(-limitY, Math.min(limitY, view.y)) }
}

export function fitImage(image: ImageSize, stage: ImageSize, reading: boolean): ImageViewport {
  const scales = imageScales(image, stage)
  const scale = reading ? scales.width : scales.fit
  return clampImage({ scale, x: 0, y: reading ? Math.max(0, (image.height * scale - stage.height) / 2) : 0, reading }, image, stage)
}

export function zoomImage(view: ImageViewport, scale: number, point: { x: number; y: number }, image: ImageSize, stage: ImageSize) {
  const next = clampImage({ ...view, scale }, image, stage).scale
  const ratio = next / view.scale
  return clampImage({ ...view, scale: next, x: point.x - (point.x - view.x) * ratio, y: point.y - (point.y - view.y) * ratio }, image, stage)
}

export function resizeImage(view: ImageViewport, image: ImageSize, previous: ImageSize, next: ImageSize) {
  const before = imageScales(image, previous), after = imageScales(image, next)
  const ratio = view.reading ? after.width / before.width : after.fit / before.fit
  // Keep the same source row at the top when reading, not the old screen centre.
  const y = view.reading ? (previous.height / 2 + view.y) * ratio - next.height / 2 : view.y * ratio
  return clampImage({ ...view, scale: view.scale * ratio, x: view.x * ratio, y }, image, next)
}
