import type { VersionInfo, VersionNumber } from '../../protocol/index.js';

export class VersionTable {
  constructor(
    readonly version: VersionNumber,
    readonly versions: readonly VersionInfo[]
  ) {}

  getFingerprint(minor: number): number | undefined {
    return this.versions[minor]?.fingerprint;
  }

  getVersionInfo(minor: number): VersionInfo | undefined {
    return this.versions[minor];
  }
}
