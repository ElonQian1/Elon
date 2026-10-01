export function openImageScan(imageUrl?: string) {
  window.dispatchEvent(new CustomEvent('elon-open-scanner', { detail: { imageUrl } }))
}
