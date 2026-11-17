# greybel-editor-service

Stores the projects the web editor saves and greybel-vs shares, in Postgres. It replaces greybel-editor-be and
reads the projects it saved, so existing `?id=` links keep working.

It is its own Cargo workspace, outside `apps/rs`, so a deployment builds only this folder.

## API

| Request | Response |
|---|---|
| `POST /workspaces {"entry", "files": {path: content}}` | `{"id", "entry", "files", "createdAt"}`; saving the same entry and files again returns the stored workspace |
| `GET /workspaces/{id}` | the workspace, or a project greybel-editor-be saved as a workspace; `404` otherwise |
| `POST /code {"content"}` | greybel-editor-be's API, stored as a workspace: `{"id", "content", "createdAt"}` |
| `GET /code/{id}` | `{"id", "content", "createdAt"}` |
| `GET /health` | `200` while the database answers |

`content` is plain code for a single `main.src`, otherwise the web editor's project document
`{"greybel": "project", "version": 1, "entry", "files"}`. Errors are `{"message"}` with status `400`, `404`, `413`,
`422` or `429`.

A workspace has at most 500 files and 1,200,000 characters; paths are relative, use `/`, and have no empty, `.` or
`..` folders.

## Settings

| Variable | Default | |
|---|---|---|
| `DATABASE_URL` | none | Postgres URL, with `?sslmode=require` for a managed database. Without it, workspaces are stored in memory. |
| `PORT` | `8080` | |
| `ALLOWED_ORIGINS` | `https://editor.greyscript.org` | Origins browsers may call from, comma-separated, or `*`. A host may start with `*.` for every subdomain, such as `https://*.vscode-cdn.net` for VS Code for the Web. |
| `SAVES_PER_MINUTE` | `30` | Saves per client IP and minute. |
| `CLIENT_IP_HEADER` | `do-connecting-ip` | Header with the client IP set by the proxy; empty to use the connection's address. |
| `DATABASE_MAX_CONNECTIONS` | `5` | |
| `LOG_FORMAT` | `text` | `json` writes one JSON object per line. |
| `RUST_LOG` | `info` | Log filter, such as `debug` or `info,greybel_editor_service=debug`. |

## Logs

Logs go to stdout. Each request's logs carry its method, path and client IP (`CLIENT_IP_HEADER`, else the
connection's address):

| Log | Level | Fields |
|---|---|---|
| `response`, for every request except `/health` | info; error for status 5xx | `status`, `latency_ms` |
| `workspace saved` | info | `id`, `files`, `characters`, `is_new` (false when the same entry and files were stored before) |
| `workspace rejected`, `request body rejected` | info | `reason`: the message the client gets |
| `save limit reached` | warn | `retry_after_s` |
| `read a project of the legacy "Code" table` | info | `id` |
| `storage request failed` | error | `error`: the database error |
| `starting`, `connected to Postgres, migrations applied`, `listening`, `shutting down, finishing open requests`, `stopped` | info | the settings, database host and name, and the signal; never `DATABASE_URL` or its password |

File contents are never logged.

Migrations in `migrations/` run at start. They add the `workspace` and `workspace_file` tables; greybel-editor-be's
`"Code"` table is only read.

## Run and test

```sh
DATABASE_URL=postgres://... cargo run
cargo test
TEST_DATABASE_URL=postgres://... cargo test   # also runs the Postgres test
```

## Deploy

`Dockerfile` builds from this folder. `deploy/app.yaml` is a DigitalOcean App Platform spec for it: apply it to the
existing app to keep its address, and set `DATABASE_URL` as an encrypted variable.
