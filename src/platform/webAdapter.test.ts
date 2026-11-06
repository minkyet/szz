import test from "node:test";
import assert from "node:assert/strict";
import { WebAdapter } from "./webAdapter";
import { UnsupportedPlatformError } from "./errors";

test("WebAdapter declares correct web sandbox capabilities", () => {
  const adapter = new WebAdapter();
  assert.equal(adapter.capabilities.canManageLocalInstances, false);
  assert.equal(adapter.capabilities.canDirectPing, false);
  assert.equal(adapter.capabilities.canManageAuth, false);
  assert.equal(adapter.capabilities.canConfigureJvm, false);
  assert.equal(adapter.capabilities.canPruneStorage, false);
});

test("WebAdapter getCatalog returns bundled fixture servers", async () => {
  const adapter = new WebAdapter();
  const catalog = await adapter.getCatalog();
  assert.ok(catalog.length > 0, "Catalog should contain bundled servers");
  assert.ok(catalog.some((item) => item.id.includes("cobblemon") || item.name.includes("Cobblemon")));
});

test("WebAdapter getAuthState returns Guest active profile", async () => {
  const adapter = new WebAdapter();
  const auth = await adapter.getAuthState();
  assert.equal(auth.accounts.length, 1);
  assert.equal(auth.active_account?.username, "Guest");
});

test("WebAdapter throws UnsupportedPlatformError for local instance management", async () => {
  const adapter = new WebAdapter();
  await assert.rejects(
    async () => {
      await adapter.installInstance({ instanceId: "inst-1" });
    },
    (err: any) => {
      assert.ok(err instanceof UnsupportedPlatformError);
      assert.equal(err.capability, "canManageLocalInstances");
      assert.equal(err.method, "installInstance");
      return true;
    }
  );

  await assert.rejects(
    async () => {
      await adapter.uninstallInstance("inst-1");
    },
    (err: any) => {
      assert.ok(err instanceof UnsupportedPlatformError);
      assert.equal(err.capability, "canManageLocalInstances");
      assert.equal(err.method, "uninstallInstance");
      return true;
    }
  );
});

test("WebAdapter throws UnsupportedPlatformError for account mutation", async () => {
  const adapter = new WebAdapter();
  await assert.rejects(
    async () => {
      await adapter.createDevAccount("Alice");
    },
    (err: any) => {
      assert.ok(err instanceof UnsupportedPlatformError);
      assert.equal(err.capability, "canManageAuth");
      assert.equal(err.method, "createDevAccount");
      return true;
    }
  );
});

test("WebAdapter play updates window.location.href when window exists", async () => {
  const adapter = new WebAdapter();
  let assignedHref: string | null = null;

  // Mock global window
  const originalWindow = (globalThis as any).window;
  (globalThis as any).window = {
    location: {
      set href(val: string) {
        assignedHref = val;
      },
      get href() {
        return assignedHref || "";
      },
    },
  };

  try {
    await adapter.play("cobblemon-server/v1");
    assert.equal(assignedHref, `szz://launch/${encodeURIComponent("cobblemon-server/v1")}`);
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test("WebAdapter onDeepLink returns cleanup no-op function", async () => {
  const adapter = new WebAdapter();
  const unsub = await adapter.onDeepLink(() => {});
  assert.equal(typeof unsub, "function");
  unsub();
});
