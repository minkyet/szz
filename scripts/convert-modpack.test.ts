import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import JSZip from "jszip";
import os from "node:os";
import { convertModpack } from "./convert-modpack";

const TMP_DIR = path.join(os.tmpdir(), "szz-test-converter");

test.beforeEach(() => {
  if (fs.existsSync(TMP_DIR)) {
    fs.rmSync(TMP_DIR, { recursive: true, force: true });
  }
  fs.mkdirSync(TMP_DIR, { recursive: true });
});

test.afterEach(() => {
  if (fs.existsSync(TMP_DIR)) {
    fs.rmSync(TMP_DIR, { recursive: true, force: true });
  }
});

test("convertModpack converts a valid Modrinth .mrpack archive", async () => {
  const zip = new JSZip();
  const modrinthIndex = {
    formatVersion: 1,
    game: "minecraft",
    versionId: "1.0.0",
    name: "Test Modrinth Pack",
    dependencies: {
      minecraft: "1.20.1",
      "fabric-loader": "0.16.10",
    },
    files: [
      {
        path: "mods/fabric-api-0.92.12+1.20.1.jar",
        hashes: {
          sha1: "3e9cdd3e2f827ca9a259df9eb8e31949437b6bd4",
          sha512: "de9da2344043b3e9ef9fb7dbfff5f2577b06000c4f73f2a37959a80123141169084cece19133bd299b45feaaa4648bdaca2ea085684888cc8c55cc9134a477ca",
        },
        env: { client: "required", server: "required" },
        downloads: [
          "https://cdn.modrinth.com/data/P7dR8mSH/versions/rvI2dfzR/fabric-api-0.92.12%2B1.20.1.jar",
        ],
      },
      {
        path: "mods/server-only-mod.jar",
        hashes: { sha1: "1111111111111111111111111111111111111111" },
        env: { client: "unsupported", server: "required" },
        downloads: ["https://cdn.modrinth.com/server-only.jar"],
      },
    ],
  };

  zip.file("modrinth.index.json", JSON.stringify(modrinthIndex));
  zip.file("overrides/config/test.json", JSON.stringify({ key: "value" }));

  const packPath = path.join(TMP_DIR, "test-pack.mrpack");
  const outputPath = path.join(TMP_DIR, "output.json");
  const configPath = path.join(TMP_DIR, "custom-config.zip");

  const buffer = await zip.generateAsync({ type: "nodebuffer" });
  fs.writeFileSync(packPath, buffer);

  const manifest = await convertModpack({
    input: packPath,
    output: outputPath,
    outputConfig: configPath,
    serverId: "custom-fabric-server",
    serverAddress: "mc.test.com:25565",
  });

  assert.equal(manifest.serverId, "custom-fabric-server");
  assert.equal(manifest.name, "Test Modrinth Pack");
  assert.equal(manifest.minecraftVersion, "1.20.1");
  assert.equal(manifest.loader.type, "fabric");
  assert.equal(manifest.loader.version, "0.16.10");
  assert.equal(manifest.serverAddress, "mc.test.com:25565");

  // Client unsupported mod was filtered out
  assert.equal(manifest.mods.length, 1);
  assert.equal(manifest.mods[0].name, "fabric-api-0.92.12+1.20.1");
  assert.equal(manifest.mods[0].source, "modrinth");
  assert.equal(manifest.mods[0].sha1, "3e9cdd3e2f827ca9a259df9eb8e31949437b6bd4");

  // Config was packaged
  assert.ok(fs.existsSync(configPath));
  const configZip = await JSZip.loadAsync(fs.readFileSync(configPath));
  assert.ok(configZip.file("config/test.json"));
});

test("convertModpack guards against Zip-Slip traversal attacks in overrides", async () => {
  const zip = new JSZip();
  zip.file("modrinth.index.json", JSON.stringify({
    formatVersion: 1,
    game: "minecraft",
    name: "Evil Pack",
    dependencies: { minecraft: "1.20.1" },
    files: [],
  }));
  zip.file("overrides/../../etc/passwd", "malicious payload");

  const packPath = path.join(TMP_DIR, "evil-pack.mrpack");
  const outputPath = path.join(TMP_DIR, "evil.json");

  const buffer = await zip.generateAsync({ type: "nodebuffer" });
  fs.writeFileSync(packPath, buffer);

  await assert.rejects(
    async () => {
      await convertModpack({
        input: packPath,
        output: outputPath,
      });
    },
    /Zip-Slip directory traversal detected/
  );
});

test("convertModpack converts a CurseForge zip archive using API", async () => {
  const originalFetch = globalThis.fetch;
  let interceptedApiKey: string | null = null;

  globalThis.fetch = async (url: string | URL | Request, init?: RequestInit): Promise<Response> => {
    const urlStr = url.toString();
    if (urlStr.includes("api.curseforge.com/v1/mods/files")) {
      const headers = (init?.headers as Record<string, string>) || {};
      interceptedApiKey = headers["x-api-key"] || null;
      return new Response(
        JSON.stringify({
          data: [
            {
              id: 3011440,
              modId: 238222,
              fileName: "jei_1.12.2-4.16.1.302.jar",
              downloadUrl: "https://edge.forgecdn.net/files/3011/440/jei_1.12.2-4.16.1.302.jar",
              hashes: [{ value: "abcd1234ef567890", algo: 1 }],
            },
          ],
        }),
        { status: 200, headers: { "Content-Type": "application/json" } }
      );
    }
    return originalFetch(url, init);
  };

  try {
    const zip = new JSZip();
    const cfManifest = {
      minecraft: {
        version: "1.12.2",
        modLoaders: [{ id: "forge-14.23.5.2860", primary: true }],
      },
      manifestType: "minecraftModpack",
      manifestVersion: 1,
      name: "SkyFactory 4 Test",
      files: [{ projectID: 238222, fileID: 3011440, required: true }],
      overrides: "overrides",
    };

    zip.file("manifest.json", JSON.stringify(cfManifest));
    zip.file("overrides/config/splash.properties", "enabled=true\n");

    const packPath = path.join(TMP_DIR, "skyfactory-test.zip");
    const outputPath = path.join(TMP_DIR, "skyfactory-test.json");

    const buffer = await zip.generateAsync({ type: "nodebuffer" });
    fs.writeFileSync(packPath, buffer);

    const manifest = await convertModpack({
      input: packPath,
      output: outputPath,
      serverId: "skyfactory-4",
      cfApiKey: "test-api-key-12345",
    });

    assert.equal(manifest.serverId, "skyfactory-4");
    assert.equal(manifest.name, "SkyFactory 4 Test");
    assert.equal(manifest.minecraftVersion, "1.12.2");
    assert.equal(manifest.loader.type, "forge");
    assert.equal(manifest.loader.version, "14.23.5.2860");
    assert.equal(manifest.mods.length, 1);
    assert.equal(manifest.mods[0].name, "jei_1.12.2-4.16.1.302");
    assert.equal(manifest.mods[0].source, "curseforge");
    assert.equal(manifest.mods[0].sha1, "abcd1234ef567890");
    assert.equal(interceptedApiKey, "test-api-key-12345");
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("convertModpack throws clear error when CurseForge API key is missing", async () => {
  const zip = new JSZip();
  const cfManifest = {
    minecraft: { version: "1.12.2", modLoaders: [{ id: "forge-14.23.5.2860", primary: true }] },
    manifestType: "minecraftModpack",
    manifestVersion: 1,
    name: "Missing Key Test",
    files: [{ projectID: 238222, fileID: 3011440, required: true }],
  };
  zip.file("manifest.json", JSON.stringify(cfManifest));

  const packPath = path.join(TMP_DIR, "missing-key-test.zip");
  const outputPath = path.join(TMP_DIR, "missing-key-test.json");
  const buffer = await zip.generateAsync({ type: "nodebuffer" });
  fs.writeFileSync(packPath, buffer);

  const savedEnvKey = process.env.CURSEFORGE_API_KEY;
  delete process.env.CURSEFORGE_API_KEY;

  try {
    await assert.rejects(
      async () => {
        await convertModpack({
          input: packPath,
          output: outputPath,
          serverId: "missing-key",
        });
      },
      /CurseForge API Key is required/
    );
  } finally {
    if (savedEnvKey !== undefined) {
      process.env.CURSEFORGE_API_KEY = savedEnvKey;
    }
  }
});

