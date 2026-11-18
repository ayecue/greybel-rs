import { execFile } from 'node:child_process';
import { createRequire } from 'node:module';
import { readFile, rm } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

const TARGET_DIR = process.env.CARGO_TARGET_DIR;

const BINARYEN = dirname(createRequire(import.meta.url).resolve('binaryen/package.json'));

// A native wasm-opt of binaryen's version runs about ten times faster than the package's JavaScript one.
const NATIVE_WASM_OPT = process.env.WASM_OPT;

const WASM_BINDGEN = process.env.WASM_BINDGEN ?? 'wasm-bindgen';

const MAX_OUTPUT = 64 * 1024 * 1024;

export type Optimization = 'speed' | 'size';

export interface WasmOptions {
  readonly crates: URL;

  readonly target: URL;

  readonly test: boolean;

  readonly optimization?: Optimization;
}

const LEVELS: Record<Optimization, { readonly rustc: string; readonly wasmOpt: string }> = {
  speed: { rustc: '3', wasmOpt: '-O3' },
  size: { rustc: '"z"', wasmOpt: '-Oz' }
};

interface BuildProfile {
  readonly cargo: 'dev' | 'release';
  readonly output: 'debug' | 'release';
  readonly settings: Readonly<Record<string, string>>;
  readonly wasmOpt?: string;
}

export interface CrateBuild {
  readonly script: string;

  readonly wasm: Buffer;
}

function buildProfile(options: WasmOptions): BuildProfile {
  if (options.test) {
    return {
      cargo: 'dev',
      output: 'debug',
      settings: { panic: '"abort"', lto: '"off"', debug: 'false' }
    };
  }

  const level = LEVELS[options.optimization ?? 'size'];

  return {
    cargo: 'release',
    output: 'release',
    settings: {
      panic: '"abort"',
      lto: 'true',
      'codegen-units': '1',
      'opt-level': level.rustc,

      strip: '"debuginfo"'
    },
    wasmOpt: level.wasmOpt
  };
}

export class CrateBuilder {
  readonly #crates: string;
  readonly #target: string;
  readonly #options: WasmOptions;
  readonly #profile: BuildProfile;
  readonly #builds = new Map<string, Promise<CrateBuild>>();
  #bindgen: Promise<string> | undefined;
  #wasmOpt: Promise<string[]> | undefined;

  constructor(options: WasmOptions) {
    this.#crates = fileURLToPath(options.crates);
    this.#target = join(TARGET_DIR ?? fileURLToPath(options.target), 'wasm');
    this.#options = options;
    this.#profile = buildProfile(options);
  }

  async build(crate: string): Promise<CrateBuild> {
    const current = this.#builds.get(crate);

    if (current !== undefined) return current;

    const build = this.#assemble(crate);

    this.#builds.set(crate, build);

    try {
      return await build;
    } finally {
      this.#builds.delete(crate);
    }
  }

  async #assemble(crate: string): Promise<CrateBuild> {
    const artifact = await this.#compile(crate);
    const outDir = join(this.#target, 'bindgen', crate);

    await rm(outDir, { recursive: true, force: true });

    const bindgen = await this.#wasmBindgen();

    await this.#bindings(bindgen, artifact, outDir);

    const wasm = await this.#optimize(join(outDir, 'index_bg.wasm'), outDir);

    return { script: join(outDir, 'index.js'), wasm: await readFile(wasm) };
  }

  async #compile(crate: string): Promise<string> {
    const { cargo, output, settings } = this.#profile;
    const args = ['rustc', '-p', crate, '--lib', '--target', 'wasm32-unknown-unknown', '--crate-type', 'cdylib'];

    if (cargo === 'release') args.push('--release');
    for (const [key, value] of Object.entries(settings)) args.push('--config', `profile.${cargo}.${key}=${value}`);
    await this.#invokeProcess('cargo', args, { CARGO_TARGET_DIR: this.#target });

    return join(this.#target, 'wasm32-unknown-unknown', output, `${crate.replaceAll('-', '_')}.wasm`);
  }

  async #bindings(command: string, artifact: string, outDir: string): Promise<void> {
    await this.#invokeProcess(command, [
      '--out-dir',
      outDir,
      '--out-name',
      'index',
      '--target',
      'web',
      '--omit-default-module-path',
      '--no-typescript',
      artifact
    ]);
  }

  async #optimize(wasm: string, outDir: string): Promise<string> {
    const level = this.#profile.wasmOpt;

    if (level === undefined) return wasm;

    const output = join(outDir, 'optimized.wasm');
    const [command, ...prefix] = await this.#wasmOptCommand();

    await this.#invokeProcess(command, [
      ...prefix,
      level,
      '--strip-producers',
      '--strip-target-features',
      wasm,
      '-o',
      output
    ]);

    return output;
  }

  /** `WASM_OPT` when it is binaryen's version, else the package's wasm-opt through Node. */
  #wasmOptCommand(): Promise<string[]> {
    this.#wasmOpt ??= (async () => {
      if (NATIVE_WASM_OPT === undefined) return [process.execPath, join(BINARYEN, 'bin', 'wasm-opt')];

      const { version } = JSON.parse(await readFile(join(BINARYEN, 'package.json'), 'utf8')) as { version: string };
      const major = version.split('.')[0];
      const native = /version (\d+)/.exec(await this.#invokeProcess(NATIVE_WASM_OPT, ['--version']))?.[1];

      if (native !== major) {
        throw new Error(
          `WASM_OPT is binaryen ${native ?? 'of an unknown version'}, but the package is binaryen ${major}`
        );
      }

      return [NATIVE_WASM_OPT];
    })();

    return this.#wasmOpt;
  }

  #wasmBindgen(): Promise<string> {
    this.#bindgen ??= (async () => {
      const lockVersion = /@([\d.]+)\s*$/.exec(
        (await this.#invokeProcess('cargo', ['pkgid', 'wasm-bindgen'])).trim()
      )?.[1];
      let cliVersion: string | undefined;

      try {
        cliVersion = /wasm-bindgen ([\d.]+)/.exec(await this.#invokeProcess(WASM_BINDGEN, ['--version']))?.[1];
      } catch {
        cliVersion = undefined;
      }

      if (cliVersion === undefined || cliVersion !== lockVersion) {
        const installation = cliVersion === undefined ? 'none is installed' : `${cliVersion} is installed`;

        throw new Error(
          `the crates use wasm-bindgen ${lockVersion}, but ${installation}: run \`cargo install wasm-bindgen-cli --version ${lockVersion} --locked\``
        );
      }

      return WASM_BINDGEN;
    })();

    return this.#bindgen;
  }

  async #invokeProcess(command: string, args: string[], env: Record<string, string> = {}): Promise<string> {
    try {
      const { stdout } = await execFileAsync(command, args, {
        cwd: this.#crates,
        env: { ...process.env, ...env },
        maxBuffer: MAX_OUTPUT
      });

      return stdout;
    } catch (error) {
      const { stderr } = error as { stderr?: string };

      throw new Error(`${command} ${args.join(' ')} failed${stderr ? `:\n${stderr}` : ''}`, { cause: error });
    }
  }
}
