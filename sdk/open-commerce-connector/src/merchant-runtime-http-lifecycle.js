export function finishMerchantRuntimeHttpRequest(lifecycle) {
  lifecycle.inFlight = Math.max(0, lifecycle.inFlight - 1)
  if (lifecycle.inFlight !== 0) return
  for (const resolve of lifecycle.drainWaiters) resolve()
  lifecycle.drainWaiters.clear()
}

export async function closeMerchantRuntimeHttpServer(server, sockets, lifecycle, graceMs) {
  if (lifecycle.phase === 'starting' && lifecycle.listenPromise) {
    try {
      await lifecycle.listenPromise
    } catch {
      lifecycle.phase = 'closed'
    }
  }
  if (lifecycle.phase === 'closed') {
    return lifecycle.closeReceipt ?? closeReceipt(graceMs, 0, false, 0, lifecycle.inFlight)
  }
  if (lifecycle.phase === 'idle') {
    lifecycle.phase = 'closed'
    return closeReceipt(graceMs, 0, false, 0, lifecycle.inFlight)
  }

  lifecycle.phase = 'draining'
  const startedAt = Date.now()
  const inFlightAtStart = lifecycle.inFlight
  let forcedConnections = 0
  const serverClosed = new Promise((resolve, reject) => {
    server.close((error) => error ? reject(error) : resolve())
  })
  const drained = waitForDrain(lifecycle).then(() => server.closeIdleConnections?.())
  let timer
  const deadline = new Promise((resolve) => {
    timer = setTimeout(() => {
      forcedConnections = sockets.size
      server.closeAllConnections?.()
      for (const socket of sockets) socket.destroy()
      resolve('forced')
    }, graceMs)
    timer.unref?.()
  })

  let outcome
  try {
    outcome = await Promise.race([
      Promise.all([serverClosed, drained]).then(() => 'graceful'),
      deadline,
    ])
    if (outcome === 'forced') await serverClosed
  } finally {
    clearTimeout(timer)
    lifecycle.phase = 'closed'
  }
  return closeReceipt(
    graceMs,
    Date.now() - startedAt,
    outcome === 'forced',
    forcedConnections,
    lifecycle.inFlight,
    inFlightAtStart,
  )
}

function waitForDrain(lifecycle) {
  if (lifecycle.inFlight === 0) return Promise.resolve()
  return new Promise((resolve) => lifecycle.drainWaiters.add(resolve))
}

function closeReceipt(graceMs, elapsedMs, forced, forcedConnections, remainingInFlight, inFlightAtStart = 0) {
  return Object.freeze({
    schema: 'merchant_runtime.http_host.v1',
    status: 'closed',
    grace_ms: graceMs,
    elapsed_ms: elapsedMs,
    forced,
    forced_connections: forcedConnections,
    in_flight_at_start: inFlightAtStart,
    remaining_in_flight: remainingInFlight,
  })
}
