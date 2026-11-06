import test from "node:test";
import assert from "node:assert/strict";
import { DesktopAdapter } from "./desktopAdapter";

test("DesktopAdapter initializes with full native desktop capabilities", () => {
  const adapter = new DesktopAdapter();
  assert.equal(adapter.capabilities.canManageLocalInstances, true);
  assert.equal(adapter.capabilities.canDirectPing, true);
  assert.equal(adapter.capabilities.canManageAuth, true);
  assert.equal(adapter.capabilities.canConfigureJvm, true);
  assert.equal(adapter.capabilities.canPruneStorage, true);
});

test("DesktopAdapter declares onDeepLink method", () => {
  const adapter = new DesktopAdapter();
  assert.equal(typeof adapter.onDeepLink, "function");
});
