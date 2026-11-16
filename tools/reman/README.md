# reman

`reman` manages workspace discovery, versions, change files, changelogs,
GitHub releases, and repository tasks.

From the repository root:

```sh
cargo reman [--root DIR] <command>
```

`cargo reman` is the alias in `.cargo/config.toml`. The global `--root` option
accepts any folder inside the repository and defaults to the current folder.
reman searches upward for the root `reman.toml`; `init` can instead start from
a Git worktree without one.

File-oriented commands read Git history in process and edit only working-tree
files. They never stage, commit, push, or create local tags. `--dry-run` prints
file diffs without writing them. Task commands execute the configured shell
commands, whose side effects are task-defined. `release publish` uses the
GitHub API; it does not tag the local repository or push HEAD.

## Configuration

reman discovers every non-ignored `reman.toml` below the repository root. Build
output is excluded. Unknown configuration keys are errors.

| Table | Location and purpose |
|---|---|
| `[repository]` | Root only, once. Sets `github = "owner/name"`, the release `tag` template (default `{name}/v{version}`), and the `changes` folder (default `.changes`). `{name}` and `{version}` are substituted in tags. |
| `[repository.project]` | Root defaults for every project's `changelog` and `skip`. |
| `[languages.<language>]` | Root defaults for a language: package `select` strings and `[tasks]`. |
| `[workspace]` | Declares `language = "rust"`, `"typescript"`, `"dotnet"`, or `"duplex"`. `development = true` forbids projects in a fixture/test-only workspace. |
| `[workspace.project]` | Overrides project defaults for that workspace. |
| `[project]` | Marks a discovered package as versioned. `kind` is required: `app` for a program, `site` for a page deployed from main, or `library` for code or definitions. |
| `[tasks]` | Tasks of the file's most specific role: repository, workspace, or project. A file can declare both a workspace and its own package. |

Project settings:

| Key | Meaning |
|---|---|
| `release` | Apps are tagged and released on GitHub by default; `false` disables that. Sites and libraries are never GitHub releases. |
| `latest` | Marks the one released app whose GitHub release becomes the repository's latest. |
| `tag` | Overrides the repository tag template for a released app. |
| `uses` | IDs or unique names of projects this project ships or builds when manifests do not express the dependency. |
| `changelog` | Changelog path relative to the project; built-in default `CHANGELOG.md`. |
| `skip` | Glob patterns relative to the project whose changes do not require a release. The built-in default is an empty array. This repository configures `tests/**`, `test/**`, `benches/**`, `examples/**`, and `README.md` in its root `reman.toml`. |

Project defaults inherit in this order: repository, nearest workspace, then
project. A set value replaces the inherited value; in particular, `skip`
replaces the whole array rather than appending to it. Language tasks are merged
into same-key workspace tasks field by field, with `env` merged by key;
`enabled = false` disables a task. `cargo reman show <project>` prints the
resolved settings, their originating files, and dependencies.

Packages without `[project]` still participate in dependency graphs and tasks,
but are not versioned or released. IDs have the form `<language>/<name>`, such
as `rs/greybel-core` or `ts/greybel-vs`; a unique package name is accepted
wherever an ID is expected.

## Package discovery

| Language | Packages and versions | Internal dependencies |
|---|---|---|
| Rust | Members of a declared Cargo workspace. Uses `package.version`, including an inherited workspace version. | Path dependencies, plus paths named through relative `[env]` entries in the root `.cargo/config.toml`. Development dependencies and references outside shipping sources affect tests only. |
| TypeScript | Packages matched by `pnpm-workspace.yaml`; version from `package.json`. Only pnpm workspaces are discovered. | `workspace:`, `link:`, and `file:` specifiers, and local `overrides`. `dependencies`, `peerDependencies`, and `optionalDependencies` are release dependencies; `devDependencies` are test-only. |
| .NET | Each immediate child folder of the workspace containing projects below it. Uses the first `<Version>` in its projects or `Directory.Build.props`. The solution names the package, else its single project, else the folder. | `ProjectReference` and source paths expressed through inherited folder properties. Projects below `tests/` or with `<IsTestProject>true</IsTestProject>` are test-only. |
| Duplex | A folder of `<major>.<minor>.protocol` files, either the workspace itself or each child folder. | None. The newest filename supplies the read-only `major.minor` version. |

A normal local dependency of a project must resolve to a package in a declared
workspace. References to shared folders outside every package make those
folders inputs of the reading package. `target`, `node_modules`, `bin`, and
`obj` are build output and never affect package changes, even if tracked.

## Change files

Pending changes are Markdown files in `.changes/pending/` (or the configured
changes folder):

```markdown
---
rs/greybel-interpreter: minor
greybel-vs: patch
---

`wait` follows the world clock instead of the wall clock.
```

Each front-matter entry is `project:level`; levels are `major`, `minor`,
`patch`, and `none`. One file can name multiple projects. A releasing change
requires a note; `none` records a change that needs no release and may have an
empty note. For Duplex, the level must agree with movement of the newest
protocol filename since the last release.

```sh
cargo reman change rs/greybel-core:minor greybel-vs:patch -m "Description"
cargo reman change greybel-vs:none -m "" --dry-run
```

Preparation moves consumed files into `.changes/YYYY-MM-DD/`; subsequent
releases on that date use `-2`, `-3`, and so on. `.changes/release.toml` records
all project versions. A change Markdown file directly in `.changes/` is an
error because it is neither pending nor released.

## Checks and change attribution

```sh
cargo reman check [--since REV]
```

Checks cover configuration/package consistency, valid versions and change
files, release tag uniqueness, at most one `latest` app, required changelogs,
development-workspace constraints, and whether every changed project requiring
a release is named by a pending change file. Every normal dependency or `uses`
target shipped by a project must itself be a project.

The default diff base is the last commit that wrote `.changes/release.toml`,
or HEAD when no such commit exists. `--since REV` selects another base for
change-file coverage. The diff includes uncommitted and untracked files.

- A file belongs to the innermost package containing it, or to packages that
  read its folder.
- Change files, `reman.toml`, project changelogs, and build output do not
  trigger release requirements.
- `skip` excludes files from release requirements only; test changes still
  count for task selection with `--since`.
- Lockfile changes are attributed to packages whose resolved dependency
  closure changed.

## Release planning and preparation

`cargo reman plan` prints the pending release without editing files.

```sh
cargo reman release prepare [--dry-run] [--date YYYY-MM-DD] [--no-check]
```

Preparation runs `check` first unless `--no-check` is given. The changelog date
defaults to today in UTC. `--dry-run` prints the plan and all proposed file
diffs without writing them.

For all pending change files, preparation:

1. Moves each named project by its highest requested level; `none` does not
   move it. A `major` bump of a 0.x version produces 1.0.0.
2. Gives every transitive dependent through normal dependencies or `uses` at
   least a patch bump. Each project moves once.
3. Writes manifest and lockfile versions where the language supports it.
4. Adds `## <version> · <date>` changelog sections, grouped by change level.
   Notes inherited through a dependency identify that dependency.
5. Moves every consumed change file into the dated release folder and writes
   the release record.

## GitHub publication

```sh
cargo reman release publish [--dry-run] [--draft] [--offline]
```

For each released app whose current version matches the release record and
whose tag does not already exist, publication creates an annotated tag at HEAD
through the GitHub API and then creates its release from the matching changelog
section. The tag is created even for `--draft`. The configured `latest` app
becomes the repository's latest release. Existing local or remote tags are
skipped; `--offline` checks only local tags.

A real run requires HEAD to be on GitHub and a contents-write token in
`GITHUB_TOKEN` or `GH_TOKEN`. It refuses a dirty working tree because otherwise
HEAD would not include the prepared files. `--dry-run` makes no GitHub changes
and only warns about a dirty tree, then prints what it would create. Without
`--offline`, both modes query GitHub tags; unauthenticated reads of a private
repository can make existing tags appear missing. A real publication cannot be
created with `--offline`.

Both modes print a JSON array of publications with `id`, `name`, `version`,
`tag`, `path`, and `language` for downstream build pipelines.

## Tasks

| Command | Suite behavior |
|---|---|
| `cargo reman install` | Runs the `install` suite. |
| `cargo reman lint` | Runs lint `run` commands. |
| `cargo reman format` | Runs lint `fix` commands. |
| `cargo reman test [SUITE...]` | Runs `unit` and `integration` by default, including `game` when explicitly requested. |
| `cargo reman run <SUITE>...` | Runs any named suites. |

All task commands accept repeatable `--task KEY` and `--language LANGUAGE`
filters, `--dry-run` to print commands, and `--no-fail-fast` to run every
selected task before returning failure. `--since [REV]` restricts coverage to
changed packages and their dependents; omitting `REV` uses the last release.

```toml
[tasks.clippy]
suite = "lint"
run = "cargo clippy {packages} --all-targets -- -D warnings"
fix = "cargo clippy {packages} --fix --allow-dirty"
cwd = "greyhack-util"       # relative to the declaring file
env = { KEY = "value" }
select = { all = "--workspace", each = "-p {name}" }
packages = "library"       # or "all"
after = ["rustfmt"]
language = "rust"          # repository task filter
enabled = true
```

Repository tasks run once at the root, workspace tasks once per workspace after
their dependencies, and project tasks once per project. Commands run through
`sh -c` (`cmd /C` on Windows) with the configured environment.

For workspace tasks, `{packages}` expands to `select.all`, or under `--since`
to `select.each` for each covered package below the task's working directory.
`{name}` is replaced inside `select.each`. Tasks with no covered packages are
skipped. `after` puts every step of the named tasks before this task.

## Inspection and initialization commands

| Command | Purpose |
|---|---|
| `cargo reman show <project>` | Resolved settings, origins, and dependencies. |
| `cargo reman graph [--all] [--dot]` | Project dependency graph; `--all` includes development packages and test edges, `--dot` emits Graphviz. |
| `cargo reman impact [--since REV]` | IDs of packages changed since the release or `REV`, plus dependents. |
| `cargo reman init [--kind KIND=PATTERN]... [--dev PATTERN]... [--dry-run]` | Writes missing configuration, initial version files and changelogs, and the first release record for review. |

`init` infers the GitHub repository from `origin`, discovers workspaces, and
uses `app` for packages building programs and `library` otherwise. Patterns
match an ID, package name, or folder glob; the first `--kind` match wins.
Development workspaces and `--dev` matches get no `[project]`. An unpublished
manifest also gets no `[project]` unless an explicit `--kind` pattern matches
it. .NET packages are unpublished when every project is test-only (including
`IsTestProject`) or sets `<IsPackable>false</IsPackable>`. Running `init` again
fills only missing files.
