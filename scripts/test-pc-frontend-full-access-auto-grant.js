const assert = require('assert')
const fs = require('fs')
const path = require('path')

const repoRoot = path.resolve(__dirname, '..')
const pcRoot = path.join(repoRoot, 'pc-frontend')
const ts = require(path.join(pcRoot, 'node_modules', 'typescript'))
const originalTsLoader = require.extensions['.ts']

require.extensions['.ts'] = function loadTsModule(module, filename) {
  const source = fs.readFileSync(filename, 'utf8')
  const output = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
    fileName: filename,
  })
  module._compile(output.outputText, filename)
}

async function main() {
  const runtimePath = path.join(pcRoot, 'src', 'features', 'conversation', 'localPcRuntime.ts')
  const source = fs.readFileSync(runtimePath, 'utf8')
  const { ensureLocalFullAccessGrant } = require(runtimePath)
  const calls = []
  const result = await ensureLocalFullAccessGrant({
    adminUrl: 'http://127.0.0.1:7799',
    projectId: 'project-1',
    projectName: 'Project One',
    workspacePath: 'D:\\work\\project-1',
    runtimePermission: 'full_access',
    useLocalRouteA: true,
  }, {
    request: async (requestPath, options) => {
      calls.push({ path: requestPath, options })
      if (requestPath === '/api/full-access/grants' && !options) return { grants: [] }
      if (requestPath === '/api/cloud-projects') {
        return {
          node_id: 'node-current',
          projects: [{
            id: 'project-1',
            node_id: 'node-current',
            workspace_path: 'd:/work/project-1/',
          }],
        }
      }
      if (requestPath === '/api/full-access/grants' && options?.method === 'POST') return { ok: true }
      throw new Error(`unexpected request ${requestPath}`)
    },
  })

  assert.strictEqual(result, 'granted')
  assert.deepStrictEqual(calls.map((call) => call.path), [
    '/api/full-access/grants',
    '/api/cloud-projects',
    '/api/full-access/grants',
  ])
  assert.deepStrictEqual(JSON.parse(calls[2].options.body), {
    project_id: 'project-1',
    workspace_path: 'D:\\work\\project-1',
    confirm_full_access: true,
  })
  assert.ok(!source.includes('window.confirm'), 'automatic grants must never call window.confirm')
  assert.ok(source.includes("request('/api/cloud-projects')"), 'automatic grants must verify the current cloud project binding')

  let postedUnboundGrant = false
  await assert.rejects(
    ensureLocalFullAccessGrant({
      adminUrl: 'http://127.0.0.1:7799',
      projectId: 'project-unbound',
      workspacePath: 'D:\\work\\unbound',
      runtimePermission: 'full_access',
      useLocalRouteA: true,
    }, {
      request: async (requestPath, options) => {
        if (requestPath === '/api/full-access/grants' && !options) return { grants: [] }
        if (requestPath === '/api/cloud-projects') return { node_id: 'node-current', projects: [] }
        postedUnboundGrant = true
        return { ok: true }
      },
    }),
    /尚未绑定到当前登录账号、节点和本机目录/,
  )
  assert.strictEqual(postedUnboundGrant, false, 'an unbound project/workspace must never be granted')

  console.log('pc-frontend full-access auto-grant tests passed')
}

main()
  .finally(() => { require.extensions['.ts'] = originalTsLoader })
  .catch((error) => {
    console.error(error)
    process.exitCode = 1
  })
