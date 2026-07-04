export const WIN_CLIENT_DOWNLOAD_URL = '/api/node-agent/download/windows-client'
export const WIN_CLIENT_LAUNCH_URL = 'elon-node://open'

export function launchWinClientProtocol() {
  const iframe = document.createElement('iframe')
  iframe.style.display = 'none'
  iframe.src = WIN_CLIENT_LAUNCH_URL
  document.body.appendChild(iframe)
  window.setTimeout(() => iframe.remove(), 2000)
}
