const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const catalog = JSON.parse(fs.readFileSync(
  path.join(root, 'server/src/official_project_catalog/catalog.json'),
  'utf8',
))
const source = fs.readFileSync(
  path.join(root, 'server/src/official_project_catalog/mod.rs'),
  'utf8',
)
const previewSource = fs.readFileSync(
  path.join(root, 'server/src/official_project_catalog/public_preview.rs'),
  'utf8',
)
const storeSource = fs.readFileSync(path.join(root, 'server/src/project_store.rs'), 'utf8')
const routesSource = fs.readFileSync(path.join(root, 'server/src/router/social_routes.rs'), 'utf8')
const plazaSource = fs.readFileSync(
  path.join(root, 'pc-frontend/src/features/plaza/ProjectPlazaView.tsx'),
  'utf8',
)
const previewDialogSource = fs.readFileSync(
  path.join(root, 'pc-frontend/src/features/plaza/OfficialProjectPreviewDialog.tsx'),
  'utf8',
)

assert.equal(catalog.schema, 'yilong.official_project_catalog.v1')
assert.ok(Array.isArray(catalog.projects) && catalog.projects.length > 0)
assert.equal(new Set(catalog.projects.map((project) => project.id)).size, catalog.projects.length)
assert.match(source, /include_str!\("catalog\.json"\)/)
assert.match(source, /for project in &catalog\.projects/)
assert.match(previewSource, /yilong\.official_project_preview\.v1/)
assert.match(previewSource, /manifest_url/)
assert.match(previewSource, /resource URLs are intentionally excluded/)
assert.match(storeSource, /get_store_project_preview/)
assert.match(routesSource, /\/api\/store\/projects\/:id\/preview/)
assert.match(plazaSource, /<OfficialProjectPreviewDialog/)
assert.match(previewDialogSource, /了解项目详情/)
assert.doesNotMatch(previewDialogSource, /\/join|paper-launch|paper\/launch/)

for (const project of catalog.projects) {
  assert.ok(project.id && project.name && project.display_name && project.description)
  assert.ok(project.landing.title && project.landing.summary)

  if (!project.blueprint && !project.release) continue
  assert.ok(project.blueprint && project.release, `${project.id}: blueprint and release must appear together`)
  assert.equal(project.blueprint.schema, 'yilong.erp.blueprint.v1')
  assert.equal(project.blueprint.source_project_id, project.id)
  assert.equal(project.release.schema, 'yilong.erp.release.v1')
  assert.equal(project.release.blueprint_key, project.blueprint.blueprint_key)
  assert.match(project.release.source_git_commit, /^[0-9a-f]{40}$/)

  const blueprintModules = new Set(project.blueprint.modules.map((module) => module.module_key))
  for (const module of project.release.modules) {
    assert.ok(blueprintModules.has(module.module_key), `${project.id}: release module ${module.module_key} is missing from blueprint`)
  }
}

console.log(`Official project catalog contracts passed (${catalog.projects.length} project(s))`)
