const assert = require('node:assert/strict')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { buildSync } = require('esbuild')

const projectRoot = path.resolve(__dirname, '..')
const temporaryDirectory = fs.mkdtempSync(path.join(os.tmpdir(), 'elon-runtime-draft-'))
const outputFile = path.join(temporaryDirectory, 'runtimeDraftModel.cjs')

try {
  buildSync({
    entryPoints: [path.join(projectRoot, 'src/features/ui-tuner/live/runtimeDraftModel.ts')],
    outfile: outputFile,
    bundle: true,
    format: 'cjs',
    platform: 'node',
    logLevel: 'silent',
  })

  const {
    EMPTY_RUNTIME_DRAFT_STATE,
    acknowledgeRuntimeDraft,
    applyRuntimeDraftOperations,
    confirmRuntimeDraftFrame,
    projectRuntimeVisual,
    runtimeDraftStatus,
  } = require(outputFile)

  const baseFrame = frameAt(1_700_000_000_000)
  const node = createNode()
  const visual = projectRuntimeVisual(node, {
    width: { type: 'dp', value: 120 },
    height: { type: 'dp', value: 48 },
    textSize: { type: 'sp', value: 16 },
    backgroundColor: { type: 'argb', value: '#CC112233' },
    'cornerRadius.all': { type: 'dp', value: 12 },
  })

  assert.equal(visual.rect.width, 315, 'dp 宽度必须乘 density')
  assert.equal(visual.rect.height, 126, 'dp 高度必须乘 density')
  assert.equal(visual.fontSize, 52.5, 'sp 字号必须乘 density 与 fontScale')
  assert.equal(visual.borderRadius, 31.5, 'dp 圆角必须乘 density')
  assert.equal(visual.background, '#112233CC', 'Android AARRGGBB 必须转换为 CSS RRGGBBAA')

  const first = applyRuntimeDraftOperations(
    EMPTY_RUNTIME_DRAFT_STATE,
    node,
    [{ property: 'height', value: { type: 'dp', value: 52 } }],
    baseFrame,
  )
  const second = applyRuntimeDraftOperations(
    first,
    node,
    [{ property: 'height', value: { type: 'dp', value: 56 } }],
    baseFrame,
  )
  const staleAck = acknowledgeRuntimeDraft(second, node.runtimeNodeId, first.revision, appliedAck())
  assert.equal(staleAck, second, '旧 ACK 不能覆盖较新的本地草稿')
  assert.equal(runtimeDraftStatus(staleAck), 'local')

  const originalNow = Date.now
  Date.now = () => 1_700_000_000_000
  try {
    const acked = acknowledgeRuntimeDraft(second, node.runtimeNodeId, second.revision, appliedAck())
    assert.equal(runtimeDraftStatus(acked), 'calibrating')
    const earlyFrame = confirmRuntimeDraftFrame(acked, frameAt(1_700_000_000_100))
    assert.equal(runtimeDraftStatus(earlyFrame), 'calibrating', 'ACK 前后的旧帧不能清除草稿层')
    const confirmed = confirmRuntimeDraftFrame(acked, frameAt(1_700_000_000_200))
    assert.equal(runtimeDraftStatus(confirmed), 'confirmed', 'Android 新帧到达后才清除本地草稿层')
    assert.equal(Object.keys(confirmed.nodes).length, 0)
  } finally {
    Date.now = originalNow
  }

  const rejected = acknowledgeRuntimeDraft(first, node.runtimeNodeId, first.revision, {
    ...appliedAck(),
    status: 'REJECTED',
    error: 'unsupported property',
  })
  assert.equal(runtimeDraftStatus(rejected), 'rejected')
  assert.ok(rejected.nodes[node.runtimeNodeId], 'Android 拒绝时 PC 草稿必须保留供用户修正')

  console.log('runtime draft model: all assertions passed')
} finally {
  fs.rmSync(temporaryDirectory, { recursive: true, force: true })
}

function createNode() {
  return {
    runtimeNodeId: 'rn_button',
    definitionId: 'checkout.pay_button',
    screenId: 'checkout',
    kind: 'android.widget.Button',
    className: 'android.widget.Button',
    text: '立即支付',
    geometry: {
      boundsInDisplayPx: { left: 40, top: 100, right: 460, bottom: 226, width: 420, height: 126 },
      density: 2.625,
      fontScale: 1.25,
      rotation: 0,
      visible: true,
    },
    properties: {},
    capabilities: {},
  }
}

function frameAt(milliseconds) {
  return {
    dataUrl: 'data:image/png;base64,AA==',
    width: 1080,
    height: 2400,
    bytes: 1,
    capturedAt: new Date(milliseconds).toISOString(),
  }
}

function appliedAck() {
  return {
    status: 'APPLIED',
    requestId: 'request-1',
    newTreeRevision: 2,
  }
}
