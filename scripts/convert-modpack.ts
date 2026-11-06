import fs from "node:fs";
import path from "node:path";
import JSZip from "jszip";

// Load .env file if available
function loadEnv() {
  if (typeof process.loadEnvFile === "function") {
    try {
      process.loadEnvFile();
    } catch {
      // Ignore if .env doesn't exist
    }
  } else {
    const envPath = path.resolve(process.cwd(), ".env");
    if (fs.existsSync(envPath)) {
      const content = fs.readFileSync(envPath, "utf-8");
      for (const line of content.split("\n")) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith("#")) continue;
        const eqIdx = trimmed.indexOf("=");
        if (eqIdx > 0) {
          const key = trimmed.slice(0, eqIdx).trim();
          let val = trimmed.slice(eqIdx + 1).trim();
          if ((val.startsWith('"') && val.endsWith('"')) || (val.startsWith("'") && val.endsWith("'"))) {
            val = val.slice(1, -1);
          }
          if (!process.env[key]) {
            process.env[key] = val;
          }
        }
      }
    }
  }
}

loadEnv();

export interface LoaderConfig {
  type: "fabric" | "forge" | "neoforge" | "liteloader" | "vanilla";
  version: string;
}

export interface ModEntry {
  name: string;
  source: string;
  fileUrl: string;
  sha1?: string;
  sha512?: string;
}

export interface ConfigSpec {
  url: string;
  hash?: string;
  version?: string;
}

export interface ServerManifest {
  serverId: string;
  name: string;
  serverAddress: string;
  minecraftVersion: string;
  loader: LoaderConfig;
  config?: ConfigSpec | null;
  manifestVersion: number;
  mods: ModEntry[];
}

export interface ConvertOptions {
  input: string;
  output?: string;
  outputConfig?: string;
  serverId?: string;
  name?: string;
  serverAddress?: string;
  configUrl?: string;
  cfApiKey?: string;
}

function sanitizeId(input: string): string {
  const sanitized = input
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return sanitized || "szz-server";
}

function printUsage() {
  console.log(`
szz Modpack Converter CLI (convert-modpack)

Usage:
  pnpm convert-modpack <input-archive> [options]
  pnpm convert-modpack --input <input-archive> [options]

Supported Formats:
  - Modrinth Modpack (.mrpack)
  - CurseForge Modpack (.zip)

Options:
  -i, --input <file>           Path to modpack (.mrpack or .zip) archive (Required)
  -o, --output <file>          Path to save the generated ServerManifest JSON
  --output-config <file>       Path to save the packaged config.zip
  --server-id <id>             Unique server ID (matches ^[a-zA-Z0-9_-]+$)
  --name <name>                Human-readable server name
  --server-address <addr>      Server multiplayer connection address (default: 127.0.0.1:25565)
  --config-url <url>           Remote URL where config.zip will be hosted
  --cf-api-key <key>           CurseForge API Key (defaults to CURSEFORGE_API_KEY from .env)
  -h, --help                   Show this help message

Examples:
  pnpm convert-modpack Cobblemon.mrpack --server-id cobblemon --name "Cobblemon Server"
  pnpm convert-modpack SkyFactory_4.zip --output fixtures/servers/skyfactory4.json
`);
}

function parseArgs(): ConvertOptions {
  const args = process.argv.slice(2);
  const options: Partial<ConvertOptions> = {};

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === "-h" || arg === "--help") {
      printUsage();
      process.exit(0);
    } else if (arg === "-i" || arg === "--input") {
      options.input = args[++i];
    } else if (arg === "-o" || arg === "--output") {
      options.output = args[++i];
    } else if (arg === "--output-config") {
      options.outputConfig = args[++i];
    } else if (arg === "--server-id") {
      options.serverId = args[++i];
    } else if (arg === "--name") {
      options.name = args[++i];
    } else if (arg === "--server-address") {
      options.serverAddress = args[++i];
    } else if (arg === "--config-url") {
      options.configUrl = args[++i];
    } else if (arg === "--cf-api-key") {
      options.cfApiKey = args[++i];
    } else if (!arg.startsWith("-") && !options.input) {
      options.input = arg;
    }
  }

  if (!options.input) {
    printUsage();
    console.error("\n[ERROR] Missing required argument: input archive file.\n");
    process.exit(1);
  }

  return options as ConvertOptions;
}

// Package overrides into config.zip with strict Zip-Slip protection
async function packageOverrides(
  zip: JSZip,
  overridesDirName: string,
  targetConfigPath: string
): Promise<boolean> {
  const prefix = overridesDirName.endsWith("/") ? overridesDirName : overridesDirName + "/";
  const configZip = new JSZip();
  let count = 0;

  for (const [entryPath, zipEntry] of Object.entries(zip.files)) {
    if (zipEntry.dir) continue;
    if (entryPath.startsWith(prefix)) {
      const relPath = entryPath.slice(prefix.length);
      const normalized = path.normalize(relPath);

      // Zip-Slip attack guard
      if (normalized.startsWith("..") || path.isAbsolute(normalized) || normalized.includes("\0")) {
        throw new Error(
          `Security Violation: Zip-Slip directory traversal detected in entry: '${entryPath}'`
        );
      }

      if (!normalized || normalized === ".") continue;

      const data = await zipEntry.async("nodebuffer");
      configZip.file(normalized, data);
      count++;
    }
  }

  if (count > 0) {
    const configBuffer = await configZip.generateAsync({
      type: "nodebuffer",
      compression: "DEFLATE",
      compressionOptions: { level: 6 },
    });
    const dir = path.dirname(targetConfigPath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }
    fs.writeFileSync(targetConfigPath, configBuffer);
    console.log(`[INFO] Packaged ${count} files from '${overridesDirName}' into '${targetConfigPath}' (${(configBuffer.length / 1024).toFixed(1)} KB).`);
    return true;
  }

  return false;
}

// Modrinth .mrpack converter
async function convertModrinth(
  zip: JSZip,
  options: ConvertOptions,
  archivePath: string
): Promise<{ manifest: ServerManifest; configCreated: boolean; configPath: string }> {
  console.log(`[INFO] Detected Modrinth modpack format (.mrpack)...`);

  const indexEntry = zip.file("modrinth.index.json");
  if (!indexEntry) {
    throw new Error("Invalid Modrinth pack: missing modrinth.index.json");
  }

  const indexJson = JSON.parse(await indexEntry.async("string"));
  const packName = options.name || indexJson.name || path.basename(archivePath, ".mrpack");
  const serverId = options.serverId || sanitizeId(packName);
  const serverAddress = options.serverAddress || "127.0.0.1:25565";

  const mcVersion = indexJson.dependencies?.minecraft;
  if (!mcVersion) {
    throw new Error("Modrinth pack does not specify minecraft version dependency");
  }

  let loaderType: LoaderConfig["type"] = "fabric";
  let loaderVersion = "0.16.10";

  if (indexJson.dependencies?.["fabric-loader"]) {
    loaderType = "fabric";
    loaderVersion = indexJson.dependencies["fabric-loader"];
  } else if (indexJson.dependencies?.["forge"]) {
    loaderType = "forge";
    loaderVersion = indexJson.dependencies["forge"];
  } else if (indexJson.dependencies?.["neoforge"]) {
    loaderType = "neoforge";
    loaderVersion = indexJson.dependencies["neoforge"];
  } else if (indexJson.dependencies?.["quilt-loader"]) {
    loaderType = "fabric";
    loaderVersion = indexJson.dependencies["quilt-loader"];
  }

  const mods: ModEntry[] = [];
  const files = indexJson.files || [];

  for (const file of files) {
    if (file.env?.client === "unsupported") {
      continue;
    }

    const downloadUrl = file.downloads?.[0];
    if (!downloadUrl) continue;

    const fileName = path.basename(file.path);
    const modName = fileName.replace(/\.jar$/i, "");
    const sha1 = file.hashes?.sha1;
    const sha512 = file.hashes?.sha512;

    if (!sha1 && !sha512) {
      console.warn(`[WARN] Skipping mod '${fileName}' (no sha1 or sha512 hash found)`);
      continue;
    }

    mods.push({
      name: modName,
      source: "modrinth",
      fileUrl: downloadUrl,
      sha1,
      sha512,
    });
  }

  // Handle overrides
  const targetConfigPath =
    options.outputConfig ||
    path.join(
      options.output ? path.dirname(options.output) : process.cwd(),
      `${serverId}-config.zip`
    );

  let configCreated = false;
  // Modrinth packs can have overrides/ or client-overrides/
  if (Object.keys(zip.files).some((k) => k.startsWith("overrides/"))) {
    configCreated = await packageOverrides(zip, "overrides", targetConfigPath);
  } else if (Object.keys(zip.files).some((k) => k.startsWith("client-overrides/"))) {
    configCreated = await packageOverrides(zip, "client-overrides", targetConfigPath);
  }

  const manifest: ServerManifest = {
    serverId,
    name: packName,
    serverAddress,
    minecraftVersion: mcVersion,
    loader: {
      type: loaderType,
      version: loaderVersion,
    },
    config: options.configUrl
      ? { url: options.configUrl }
      : configCreated
      ? { url: path.basename(targetConfigPath) }
      : null,
    manifestVersion: 1,
    mods,
  };

  return { manifest, configCreated, configPath: targetConfigPath };
}

// CurseForge modpack converter with 2-tier API key fallback
async function convertCurseForge(
  zip: JSZip,
  options: ConvertOptions,
  archivePath: string
): Promise<{ manifest: ServerManifest; configCreated: boolean; configPath: string }> {
  console.log(`[INFO] Detected CurseForge modpack format (.zip)...`);

  const manifestEntry = zip.file("manifest.json");
  if (!manifestEntry) {
    throw new Error("Invalid CurseForge pack: missing manifest.json");
  }

  const cfManifest = JSON.parse(await manifestEntry.async("string"));
  const packName = options.name || cfManifest.name || path.basename(archivePath, ".zip");
  const serverId = options.serverId || sanitizeId(packName);
  const serverAddress = options.serverAddress || "127.0.0.1:25565";

  const mcVersion = cfManifest.minecraft?.version;
  if (!mcVersion) {
    throw new Error("CurseForge manifest missing minecraft.version");
  }

  // Parse loader
  let loaderType: LoaderConfig["type"] = "forge";
  let loaderVersion = "14.23.5.2860";

  const modLoaders = cfManifest.minecraft?.modLoaders || [];
  const primaryLoader = modLoaders.find((l: { primary?: boolean }) => l.primary) || modLoaders[0];
  if (primaryLoader?.id) {
    const rawId: string = primaryLoader.id;
    if (rawId.startsWith("forge-")) {
      loaderType = "forge";
      loaderVersion = rawId.replace(/^forge-/, "");
    } else if (rawId.startsWith("fabric-")) {
      loaderType = "fabric";
      loaderVersion = rawId.replace(/^fabric-/, "");
    } else if (rawId.startsWith("neoforge-")) {
      loaderType = "neoforge";
      loaderVersion = rawId.replace(/^neoforge-/, "");
    }
  }

  const filesToFetch: Array<{ projectID: number; fileID: number }> = cfManifest.files || [];

  // Resolve API Key
  const apiKey = options.cfApiKey || process.env.CURSEFORGE_API_KEY;
  if (filesToFetch.length > 0 && !apiKey) {
    throw new Error(
      `CurseForge API Key is required to resolve ${filesToFetch.length} mod file(s).\n` +
      `Please configure CURSEFORGE_API_KEY in .env or provide it via the --cf-api-key option.\n` +
      `Obtain an API key at https://console.curseforge.com`
    );
  }

  if (apiKey) {
    const masked = apiKey.length > 8 ? `${apiKey.slice(0, 8)}...` : "***";
    console.log(`[INFO] Using CurseForge API key: ${masked}`);
  }

  console.log(`[INFO] Resolving ${filesToFetch.length} mods via CurseForge v1 API...`);

  const mods: ModEntry[] = [];
  const batchSize = 50;

  for (let i = 0; i < filesToFetch.length; i += batchSize) {
    const chunk = filesToFetch.slice(i, i + batchSize);
    const fileIds = chunk.map((f) => f.fileID);

    let res: Response;
    try {
      res = await fetch("https://api.curseforge.com/v1/mods/files", {
        method: "POST",
        headers: {
          "x-api-key": apiKey,
          "Content-Type": "application/json",
          Accept: "application/json",
        },
        body: JSON.stringify({ fileIds }),
      });
    } catch (err: unknown) {
      throw new Error(`CurseForge API network failure: ${(err as Error).message}`);
    }

    if (!res.ok) {
      if (res.status === 403 || res.status === 429) {
        throw new Error(
          `CurseForge API error (HTTP ${res.status}): Authentication failed or rate limit reached.\n` +
          `Please check that CURSEFORGE_API_KEY in .env is valid or obtain an API key at https://console.curseforge.com.`
        );
      }
      throw new Error(`CurseForge API returned HTTP ${res.status}: ${await res.text()}`);
    }

    const json = (await res.json()) as {
      data?: Array<{
        id: number;
        modId: number;
        fileName: string;
        downloadUrl?: string | null;
        hashes?: Array<{ value: string; algo: number }>;
      }>;
    };

    const returnedFiles = json.data || [];
    for (const f of returnedFiles) {
      let downloadUrl = f.downloadUrl;
      const fileName = f.fileName || `mod-${f.modId}-${f.id}.jar`;

      // Fallback to Edge CDN formula if downloadUrl is omitted by author settings
      if (!downloadUrl) {
        const p1 = Math.floor(f.id / 1000);
        const p2 = f.id % 1000;
        downloadUrl = `https://edge.forgecdn.net/files/${p1}/${p2}/${encodeURIComponent(fileName)}`;
      }

      // Find SHA-1 (algo === 1)
      const sha1Entry = f.hashes?.find((h) => h.algo === 1);
      const sha1 = sha1Entry?.value;

      mods.push({
        name: fileName.replace(/\.jar$/i, ""),
        source: "curseforge",
        fileUrl: downloadUrl,
        sha1: sha1 || undefined,
      });
    }

    process.stdout.write(`\r[INFO] Progress: ${Math.min(i + batchSize, filesToFetch.length)} / ${filesToFetch.length} files resolved`);
  }
  console.log("");

  // Handle overrides
  const overridesFolder = cfManifest.overrides || "overrides";
  const overridesPrefix = overridesFolder.endsWith("/") ? overridesFolder : overridesFolder + "/";
  const targetConfigPath =
    options.outputConfig ||
    path.join(
      options.output ? path.dirname(options.output) : process.cwd(),
      `${serverId}-config.zip`
    );

  let configCreated = false;
  if (Object.keys(zip.files).some((k) => k.startsWith(overridesPrefix))) {
    configCreated = await packageOverrides(zip, overridesFolder, targetConfigPath);
  }

  const manifest: ServerManifest = {
    serverId,
    name: packName,
    serverAddress,
    minecraftVersion: mcVersion,
    loader: {
      type: loaderType,
      version: loaderVersion,
    },
    config: options.configUrl
      ? { url: options.configUrl }
      : configCreated
      ? { url: path.basename(targetConfigPath) }
      : null,
    manifestVersion: 1,
    mods,
  };

  return { manifest, configCreated, configPath: targetConfigPath };
}

export async function convertModpack(options: ConvertOptions): Promise<ServerManifest> {
  const archivePath = path.resolve(process.cwd(), options.input);
  if (!fs.existsSync(archivePath)) {
    throw new Error(`Input file not found: ${archivePath}`);
  }

  const buffer = fs.readFileSync(archivePath);
  const zip = await JSZip.loadAsync(buffer);

  // Strict Zip-Slip validation across all archive entries
  for (const entryPath of Object.keys(zip.files)) {
    if (
      entryPath.startsWith("/") ||
      entryPath.startsWith("\\") ||
      entryPath.includes("..") ||
      path.isAbsolute(entryPath) ||
      path.normalize(entryPath).startsWith("..") ||
      entryPath.includes("\0")
    ) {
      throw new Error(
        `Security Violation: Zip-Slip directory traversal detected in entry: '${entryPath}'`
      );
    }
  }

  let result: { manifest: ServerManifest; configCreated: boolean; configPath: string };

  if (zip.file("modrinth.index.json")) {
    result = await convertModrinth(zip, options, archivePath);
  } else if (zip.file("manifest.json")) {
    result = await convertCurseForge(zip, options, archivePath);
  } else {
    throw new Error(
      "Unrecognized modpack format. Archive must contain 'modrinth.index.json' (Modrinth) or 'manifest.json' (CurseForge)."
    );
  }

  const outputPath =
    options.output ||
    path.join(process.cwd(), `${result.manifest.serverId}.json`);

  const outDir = path.dirname(outputPath);
  if (!fs.existsSync(outDir)) {
    fs.mkdirSync(outDir, { recursive: true });
  }

  fs.writeFileSync(outputPath, JSON.stringify(result.manifest, null, 2), "utf-8");
  console.log(`\n[SUCCESS] ServerManifest written to: ${outputPath}`);
  console.log(`  Server ID:         ${result.manifest.serverId}`);
  console.log(`  Server Name:       ${result.manifest.name}`);
  console.log(`  Minecraft Version: ${result.manifest.minecraftVersion}`);
  console.log(`  Loader:            ${result.manifest.loader.type} (v${result.manifest.loader.version})`);
  console.log(`  Total Mods:        ${result.manifest.mods.length}`);
  if (result.configCreated) {
    console.log(`  Config Archive:    ${result.configPath}`);
  }

  return result.manifest;
}

// Execute when invoked from CLI
if (process.argv[1] && process.argv[1].endsWith("convert-modpack.ts")) {
  const opts = parseArgs();
  convertModpack(opts).catch((err: unknown) => {
    console.error(`\n[FATAL ERROR] ${(err as Error).message}\n`);
    process.exit(1);
  });
}
