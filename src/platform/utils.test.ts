import test from "node:test";
import assert from "node:assert/strict";
import { isTauri } from "./utils";
import { UnsupportedPlatformError } from "./errors";

test("UnsupportedPlatformError sets properties correctly", () => {
  const err = new UnsupportedPlatformError("canManageLocalInstances", "installServer");
  assert.equal(err.name, "UnsupportedPlatformError");
  assert.equal(err.capability, "canManageLocalInstances");
  assert.equal(err.method, "installServer");
  assert.match(err.message, /canManageLocalInstances/);
  assert.match(err.message, /installServer/);
});

test("isTauri returns a boolean in current node/test runtime", () => {
  const result = isTauri();
  assert.equal(typeof result, "boolean");
});

test("isTauri respects global overrides when present", () => {
  const g = globalThis as { isTauri?: boolean };
  const original = g.isTauri;
  try {
    g.isTauri = true;
    assert.equal(isTauri(), true);
  } finally {
    if (original === undefined) {
      delete g.isTauri;
    } else {
      g.isTauri = original;
    }
  }
});
