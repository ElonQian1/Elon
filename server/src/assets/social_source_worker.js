importScripts('/assets/vendor/jsqr.js');
self.onmessage = ({ data }) => {
  const { pixels, width, height } = data, found = [];
  for (let i = 0; i < 8; i++) {
    const qr = jsQR(pixels, width, height); if (!qr) break;
    found.push(qr.data);
    const points = [qr.location.topLeftCorner, qr.location.topRightCorner, qr.location.bottomLeftCorner, qr.location.bottomRightCorner];
    const left = Math.max(0, Math.floor(Math.min(...points.map(p => p.x))) - 3), right = Math.min(width, Math.ceil(Math.max(...points.map(p => p.x))) + 3);
    const top = Math.max(0, Math.floor(Math.min(...points.map(p => p.y))) - 3), bottom = Math.min(height, Math.ceil(Math.max(...points.map(p => p.y))) + 3);
    for (let y = top; y < bottom; y++) pixels.fill(255, (y * width + left) * 4, (y * width + right) * 4);
  }
  self.postMessage(found);
};
