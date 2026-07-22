import assert from 'node:assert/strict'
import {
  PROJECT_PLAZA_CARD_MIN_SCALE,
  projectPlazaCardScale,
  projectPlazaCardScales,
} from '../src/features/plaza/projectCarouselScale.mjs'

assert.equal(projectPlazaCardScale(0, 300), 1)
assert.equal(projectPlazaCardScale(150, 300), 0.95)
assert.equal(projectPlazaCardScale(300, 300), PROJECT_PLAZA_CARD_MIN_SCALE)
assert.equal(projectPlazaCardScale(-900, 300), PROJECT_PLAZA_CARD_MIN_SCALE)
assert.equal(projectPlazaCardScale(0, 0), 1)
assert.equal(projectPlazaCardScale(1, 0), PROJECT_PLAZA_CARD_MIN_SCALE)
assert.equal(projectPlazaCardScale(300, 300, 2), 1)
assert.equal(projectPlazaCardScale(300, 300, -1), 0)

assert.deepEqual(projectPlazaCardScales([], 100, 200), [])
assert.deepEqual(projectPlazaCardScales([100], 100, 200), [1])
assert.deepEqual(projectPlazaCardScales([100, 300], 100, 200), [1, 0.9])
assert.deepEqual(projectPlazaCardScales([-300, -100, 100, 300], 100, 200), [0.9, 0.9, 1, 0.9])

console.log('project plaza carousel scale tests passed')
