export interface StorageCategoryUsage {
  totalBytes: number;
  reclaimableBytes: number;
  itemCount: number;
  reclaimableCount: number;
}

export interface StorageUsageReport {
  totalBytes: number;
  reclaimableBytes: number;
  mods: StorageCategoryUsage;
  runtimes: StorageCategoryUsage;
  assets: StorageCategoryUsage;
  libraries: StorageCategoryUsage;
  instances: StorageCategoryUsage;
}

export interface CleanOptions {
  cleanMods: boolean;
  cleanRuntimes: boolean;
  cleanAssets: boolean;
  cleanLibraries: boolean;
  dryRun: boolean;
}

export interface CleanSummary {
  reclaimedBytes: number;
  deletedModsCount: number;
  deletedRuntimesCount: number;
  deletedAssetsCount: number;
  deletedLibrariesCount: number;
  dryRun: boolean;
}

export const formatBytes = (bytes: number): string => {
  if (isNaN(bytes) || bytes <= 0) {
    return "0 B";
  }

  const kb = 1024;
  const mb = 1024 * kb;
  const gb = 1024 * mb;

  if (bytes >= gb) {
    return `${(bytes / gb).toFixed(2)} GB`;
  } else if (bytes >= mb) {
    return `${(bytes / mb).toFixed(1)} MB`;
  } else if (bytes >= kb) {
    return `${(bytes / kb).toFixed(1)} KB`;
  } else {
    return `${bytes} B`;
  }
};
