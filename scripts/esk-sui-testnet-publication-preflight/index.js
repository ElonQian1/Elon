'use strict'

const path = require('node:path')
const { createTemplate } = require('./template')
const { validateCandidate } = require('./validate')
const { loadAndVerifyRepository } = require('./repository')
const { createPreflightPlan } = require('./plan')

const DEFAULT_REPOSITORY_ROOT = path.resolve(__dirname, '..', '..')

function preflightCandidate(candidate, repoRoot = DEFAULT_REPOSITORY_ROOT) {
  const normalized = validateCandidate(candidate)
  const repository = loadAndVerifyRepository(repoRoot)
  return createPreflightPlan(normalized, repository)
}

module.exports = { DEFAULT_REPOSITORY_ROOT, createTemplate, preflightCandidate }
