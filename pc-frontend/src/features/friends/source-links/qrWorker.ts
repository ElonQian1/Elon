import jsQR from 'jsqr'
self.onmessage = ({ data }: MessageEvent<{ pixels: Uint8ClampedArray; width: number; height: number }>) => {
  const { pixels, width, height } = data
  const found: string[] = []
  for (let index = 0; index < 8; index++) {
    const code = jsQR(pixels, width, height)
    if (!code) break
    found.push(code.data)
    // Mask the complete detected symbol so the next pass can find another QR.
    const points = [code.location.topLeftCorner, code.location.topRightCorner, code.location.bottomLeftCorner, code.location.bottomRightCorner]
    const left = Math.max(0, Math.floor(Math.min(...points.map(p => p.x))) - 3)
    const right = Math.min(width, Math.ceil(Math.max(...points.map(p => p.x))) + 3)
    const top = Math.max(0, Math.floor(Math.min(...points.map(p => p.y))) - 3)
    const bottom = Math.min(height, Math.ceil(Math.max(...points.map(p => p.y))) + 3)
    for (let y = top; y < bottom; y++) pixels.fill(255, (y * width + left) * 4, (y * width + right) * 4)
  }
  self.postMessage(found)
}
