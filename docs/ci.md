# CI and releases

GitHub Actions runs reman's tasks, deploys the sites and the editor service, and releases the apps. The workflows
are in `.github/workflows`; `.github/actions/setup` installs the toolchains, caches and packages each job needs.

## CI (`ci.yml`)

Runs on pull requests, on pushes to `main` and manually.

| Job | Runs |
|---|---|
| `changes` | `cargo reman check --since <base>` and `cargo reman impact --since <base>` |
| `lint` | `cargo reman lint --task rustfmt --task clippy --task format --task csharpier`; op-lint is not part of CI |
| `rust` | `cargo reman test unit integration --language rust` |
| `dotnet` | `cargo reman test unit --language dotnet` |
| `typescript` | `cargo reman run build --language typescript`, the TypeScript unit tests, greybel-vs in VS Code (desktop and web, under xvfb), and the production builds of the sites as the `sites` artifact |
| `deploy` | `deploy.yml`, once the jobs above passed |

`<base>` is a pull request's base commit, or a push's previous commit. Pull requests run only the jobs and tasks of
the projects their changes reach; pushes to `main` run everything. Tests against the installed game (`cargo reman
test game`) are not part of CI.

## Deploy (`deploy.yml`)

Deploys what changed since the previous commit of `main`, or every site and the service when CI is run manually.

- **The `.wasm` files** of greybel-editor-web and greybel-api-reference-site go to DigitalOcean Spaces:
  `https://greybel-assets.fra1.digitaloceanspaces.com/wasm/<name>-<hash>.wasm`. CI builds the sites with
  `WASM_BASE_URL` set, which points their `.wasm` URLs there (each `vite.config.ts`; test builds are not affected).
  `.github/scripts/upload-wasm.sh` uploads them brotli-compressed (`Content-Encoding: br`, `Content-Type:
  application/wasm`, cached for a year), skips files already uploaded and never deletes any, so pages built before
  still load theirs.
- **The sites** go to Netlify with `netlify deploy`, without the `.wasm` files. Pushes to `main` deploy to
  production; pull requests from this repository get a preview (`pr-<number>`), linked in a pull request comment.
  Previews call the production editor service, whose `ALLOWED_ORIGINS` doesn't list them, so saving fails there.
- **greybel-editor-service** is deployed by `doctl apps create-deployment`, which makes App Platform build `main`
  (`apps/rs/greybel-editor-service/deploy/app.yaml`, `deploy_on_push: false`). Pull requests don't deploy it.

## Releases (`release.yml`)

Runs when a push to `main` changes `.changes/release.toml`, which `cargo reman release prepare` writes
([Development](development.md#releases)), and manually.

1. **publish:** `cargo reman release publish --draft` tags each app whose version is not released yet and creates
   a draft release from its changelog section.
2. **native:** builds `greybel`, `greybel-lsp`, `greybel-runtime`, `greyhack-world-server` and `greybel-builder`
   for each platform. Linux uses `cargo zigbuild` against glibc 2.28 (VS Code's minimum) or musl.
3. **The apps' files:**

   | Release | Files |
   |---|---|
   | greybel-vs | `greybel-vs-<platform>-<version>.vsix` for `web`, `win32-x64`, `win32-arm64`, `linux-x64`, `linux-arm64`, `linux-armhf`, `alpine-x64`, `alpine-arm64`, `darwin-x64`, `darwin-arm64`; also published to the Visual Studio Marketplace and Open VSX |
   | greybel-cli | `greybel-<version>-<rust target>.tar.gz`, `.zip` for Windows |
   | greybel-lsp-stdio | `greybel-lsp-<version>-<rust target>.tar.gz`, `.zip` for Windows |
   | greyhack-setup-wizard | `greyhack-setup-wizard-<version>-<runtime>.tar.gz` (`.zip` for Windows) with the window and the command-line tool, self-contained and trimmed, for `win-x64`, `win-arm64`, `linux-x64`, `linux-arm64`, `osx-x64`, `osx-arm64`; on macOS the window is `Grey Hack Setup Wizard.app` |
   | greyhack-message-hook | `GreyHackMessageHook5.dll` (BepInEx 5) and `GreyHackMessageHook.dll` (BepInEx 6) |

4. **finish:** adds `SHA256SUMS` to each release, publishes the drafts and marks greyhack-message-hook's release as
   the latest (its `reman.toml` sets `latest = true`; the setup wizard reads the latest release).

A release stays a draft while any job fails. Re-run the failed jobs, or the workflow at the same commit: publish then
lists the apps tagged at that commit again (`is_new: false`) and the jobs rebuild them; attaching uses `--clobber`,
and the marketplaces skip a version they already have. Nothing is signed: Windows shows a SmartScreen warning and
macOS asks to allow the programs in System Settings.
