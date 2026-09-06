'use strict';

const { createHash } = require('node:crypto');
const { parseStrictJson } = require('../esk-paid-reconciliation/strict-json');
const schema = require('../../contracts/esk/early-support-policy-draft-v1.schema.json');

const MAX_INPUT_BYTES = 64 * 1024;
const DECISION_FIELDS = Object.freeze([...schema.properties.decisions.required]);
const UNSAFE_KEYS = new Set(['__proto__', 'constructor', 'prototype']);
const ERROR_CODES = new Set([
  'INVALID_INPUT', 'INPUT_TOO_LARGE', 'INVALID_UTF8', 'INVALID_JSON',
  'DUPLICATE_JSON_KEY', 'INPUT_TOO_DEEP', 'UNSAFE_KEY', 'INVALID_STRUCTURE',
  'INVALID_VALUE', 'INVALID_DATE', 'INVALID_DATE_ORDER', 'INVALID_ARGUMENTS',
  'INPUT_NOT_REGULAR_FILE', 'INPUT_READ_FAILED', 'INTERNAL_ERROR',
]);

class PolicyInputError extends Error {
  constructor(code) {
    const safeCode = ERROR_CODES.has(code) ? code : 'INTERNAL_ERROR';
    super(safeCode);
    this.name = 'PolicyInputError';
    this.code = safeCode;
  }
}

function fail(code) {
  throw new PolicyInputError(code);
}

function inspectValues(value) {
  if (typeof value === 'string') {
    // JSON escape sequences can encode lone surrogates despite valid UTF-8.
    for (const character of value) {
      const point = character.codePointAt(0);
      if (point >= 0xd800 && point <= 0xdfff) fail('INVALID_VALUE');
    }
  } else if (value && typeof value === 'object') {
    for (const key of Object.keys(value)) {
      if (UNSAFE_KEYS.has(key)) fail('UNSAFE_KEY');
      inspectValues(key);
      inspectValues(value[key]);
    }
  }
}

// The bundled schema is the sole source for fields, enums and text bounds.
// Only its local validation vocabulary is needed; no external refs are loaded.
function validate(value, rule) {
  if (rule.$ref) {
    const name = rule.$ref.slice('#/$defs/'.length);
    return validate(value, schema.$defs[name]);
  }
  if (Object.hasOwn(rule, 'const') && value !== rule.const) fail('INVALID_VALUE');
  if (rule.enum && !rule.enum.includes(value)) fail('INVALID_VALUE');
  if (rule.type) {
    const actual = value === null ? 'null' : Array.isArray(value) ? 'array' : typeof value;
    if (![rule.type].flat().includes(actual)) {
      fail(rule.type === 'object' ? 'INVALID_STRUCTURE' : 'INVALID_VALUE');
    }
  }
  if (rule.type === 'object') {
    if (rule.required.some((key) => !Object.hasOwn(value, key))
        || Object.keys(value).some((key) => !Object.hasOwn(rule.properties, key))) {
      fail('INVALID_STRUCTURE');
    }
    for (const key of rule.required) validate(value[key], rule.properties[key]);
  }
  if (typeof value === 'string') {
    const length = [...value].length;
    if ((rule.minLength !== undefined && length < rule.minLength)
        || (rule.maxLength !== undefined && length > rule.maxLength)
        || (rule.pattern && !new RegExp(rule.pattern, 'u').test(value))) {
      fail('INVALID_VALUE');
    }
  }
}

function validateDate(value) {
  if (value === null) return;
  if (typeof value !== 'string'
      || !/^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$/.test(value)
      || value.startsWith('0000-')) fail('INVALID_DATE');
  const date = new Date(value);
  if (!Number.isFinite(date.getTime())
      || date.toISOString() !== value.replace('Z', '.000Z')) fail('INVALID_DATE');
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

function evaluatePolicyBuffer(buffer) {
  if (!Buffer.isBuffer(buffer)) fail('INVALID_INPUT');
  if (buffer.length > MAX_INPUT_BYTES) fail('INPUT_TOO_LARGE');
  let policy;
  try {
    policy = parseStrictJson(buffer);
  } catch (error) {
    throw new PolicyInputError(error.code);
  }
  inspectValues(policy);
  // Date errors retain their stable code even for a malformed timestamp string.
  if (policy && policy.decisions && typeof policy.decisions === 'object') {
    for (const key of ['program_start_at', 'program_end_at']) {
      if (Object.hasOwn(policy.decisions, key)) validateDate(policy.decisions[key]);
    }
  }
  validate(policy, schema);
  const decisions = policy.decisions;
  if (decisions.program_start_at !== null && decisions.program_end_at !== null
      && decisions.program_end_at <= decisions.program_start_at) fail('INVALID_DATE_ORDER');
  const needsMinimum = decisions.protection_scope === 'principal_and_minimum_return';
  const missing = DECISION_FIELDS.filter((key) => decisions[key] === null
    && (key !== 'minimum_return_terms' || needsMinimum));
  const issues = [];
  if (decisions.minimum_return_terms !== null && !needsMinimum) {
    issues.push(decisions.protection_scope === null
      ? 'MINIMUM_RETURN_SCOPE_UNDECIDED' : 'MINIMUM_RETURN_TERMS_NOT_APPLICABLE');
  }
  return {
    schema: 'elon.esk.early_support_policy_review.v1',
    input_digest: createHash('sha256').update(canonicalJson(policy), 'utf8').digest('hex'),
    policy_status: 'draft',
    review_status: issues.length ? 'needs_correction'
      : missing.length ? 'needs_decisions' : 'ready_for_policy_review',
    missing_decisions: missing,
    consistency_issues: issues,
    production_authorized: false,
    funding_verified: false,
    funds_moved: false,
  };
}

module.exports = { MAX_INPUT_BYTES, DECISION_FIELDS, PolicyInputError, evaluatePolicyBuffer };
