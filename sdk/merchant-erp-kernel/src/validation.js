import { fail } from "./errors.js";

const ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$/;
const CURRENCY_PATTERN = /^[A-Z]{3}$/;

export function requireObject(value, field) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    fail("INVALID_INPUT", `${field} must be an object`);
  }
  return value;
}

export function requireId(value, field) {
  if (typeof value !== "string" || !ID_PATTERN.test(value)) {
    fail("INVALID_INPUT", `${field} must be a stable identifier`);
  }
  return value;
}

export function optionalText(value, field, maxLength = 240) {
  if (value === undefined || value === null || value === "") {
    return null;
  }
  if (typeof value !== "string") {
    fail("INVALID_INPUT", `${field} must be text`);
  }
  const normalized = value.trim();
  if (!normalized || normalized.length > maxLength) {
    fail("INVALID_INPUT", `${field} must contain 1-${maxLength} characters`);
  }
  return normalized;
}

export function requireCurrency(value, field = "currency") {
  if (typeof value !== "string" || !CURRENCY_PATTERN.test(value)) {
    fail("INVALID_INPUT", `${field} must be a three-letter uppercase currency`);
  }
  return value;
}

export function requirePositiveInteger(value, field, maximum = 1_000_000_000) {
  if (!Number.isSafeInteger(value) || value <= 0 || value > maximum) {
    fail("INVALID_INPUT", `${field} must be a positive safe integer`);
  }
  return value;
}

export function requireNonNegativeInteger(value, field) {
  if (!Number.isSafeInteger(value) || value < 0) {
    fail("INVALID_INPUT", `${field} must be a non-negative safe integer`);
  }
  return value;
}

export function requireUniqueItems(items, key, field) {
  if (!Array.isArray(items) || items.length === 0 || items.length > 100) {
    fail("INVALID_INPUT", `${field} must contain 1-100 items`);
  }
  const seen = new Set();
  for (const item of items) {
    requireObject(item, field);
    const value = requireId(item[key], `${field}.${key}`);
    if (seen.has(value)) {
      fail("INVALID_INPUT", `${field} contains duplicate ${key}`);
    }
    seen.add(value);
  }
  return items;
}

export function checkedMultiply(left, right, field) {
  const value = left * right;
  if (!Number.isSafeInteger(value)) {
    fail("AMOUNT_OVERFLOW", `${field} exceeds safe integer range`);
  }
  return value;
}

export function checkedAdd(left, right, field) {
  const value = left + right;
  if (!Number.isSafeInteger(value)) {
    fail("AMOUNT_OVERFLOW", `${field} exceeds safe integer range`);
  }
  return value;
}
