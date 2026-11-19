import { spawn } from 'child_process';
import { createInterface } from 'readline';

/** A game's BepInEx installation: major version 5 or 6 and product version. */
export interface BepInExInstallation {
  readonly major: number;
  readonly version: string;
}

/** greyhack-setup's status of a game. */
export interface GameStatus {
  readonly folder: string;
  readonly bepinex: BepInExInstallation | null;
  readonly hook: string | null;
  readonly port: number | null;
  readonly launchOption: string | null;
  readonly duplicateSteamApi: string | null;
  readonly readiness: { readonly kind: 'noBepInEx' | 'noHook' | 'wrongHook' | 'ready' };
}

/** A BepInEx release greyhack-setup installs. */
export interface BepInExRelease {
  readonly major: number;
  readonly version: string;
  readonly url: string;
}

/** greyhack-setup's `status` or `done` event. */
export interface StatusReport {
  readonly status: GameStatus;
  readonly readinessText: string;
  readonly bepinexReleases: readonly BepInExRelease[];
  readonly hookRelease: string;
}

interface SetupEvent extends Partial<StatusReport> {
  readonly type: 'progress' | 'status' | 'done' | 'error';
  readonly message?: string;
}

/** Runs the extension's greyhack-setup binary with `--json`. */
export class SetupTool {
  readonly #binary: string;

  constructor(binary: string) {
    this.#binary = binary;
  }

  /** The status of the game in `folder`, or of the one greyhack-setup finds. */
  status(folder?: string): Promise<StatusReport> {
    return this.run(['status'], folder, () => {});
  }

  /**
   * Runs `args` for the game in `folder` without confirmation requests, reporting each step to `progress`; resolves
   * with the status afterwards.
   */
  run(args: readonly string[], folder: string | undefined, progress: (message: string) => void): Promise<StatusReport> {
    const child = spawn(
      this.#binary,
      [...args, '--json', '--yes', ...(folder === undefined ? [] : ['--game', folder])],
      { stdio: ['ignore', 'pipe', 'pipe'] }
    );
    let report: StatusReport | undefined;
    let failure: string | undefined;
    let errorOutput = '';

    child.stderr.on('data', (chunk: Buffer) => (errorOutput += chunk.toString()));
    createInterface({ input: child.stdout }).on('line', (line) => {
      if (!line.startsWith('{')) return;

      const event = JSON.parse(line) as SetupEvent;

      switch (event.type) {
        case 'progress':
          progress(event.message ?? '');
          break;
        case 'error':
          failure = event.message;
          break;
        default:
          report = event as StatusReport;
      }
    });

    return new Promise((resolve, reject) => {
      child.on('error', reject);
      child.on('close', (code) => {
        if (report === undefined || code !== 0) {
          reject(new Error(failure ?? (errorOutput.trim() || `greyhack-setup exited with code ${code}`)));

          return;
        }

        resolve(report);
      });
    });
  }
}
