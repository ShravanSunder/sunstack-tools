# Cloudflare releases by major

This workflow serves the shared library through Workers Static Assets. Authors load two tags from one major path:

```html
<link rel="stylesheet" href="https://YOUR_CDN_HOST/v1/theme.css" />
<script type="module" src="https://YOUR_CDN_HOST/v1/runtime.js"></script>
```

The matching component guide is `/v1/authoring/SKILL.md`. The build includes its references, metadata, worker, fonts and lazy chunks. This does not upload authored explainers, publish npm packages, or install a framework for readers. The existing experimental Worker/pins are separate and unchanged.

## Version and release flow

1. A feature PR runs `pnpm changeset` and declares `@sunstack/demistify` patch, minor or major. Compatible fixes use patch; compatible additions use minor; removing/renaming a component/attribute or changing its meaning uses major. Reviewers judge semantic compatibility; a source diff cannot establish it automatically.
2. PR CI checks formatting/lint/types, builds the complete library/demo, runs Node tests, validates the actual Changesets plan and compiles the Cloudflare Worker without credentials or upload.
3. After merge, Changesets opens a version/changelog PR. It uses no npm publish command and creates no Git tags. The workflow explicitly dispatches checks for the bot branch because PRs created with `GITHUB_TOKEN` do not start another PR workflow automatically.
4. Merging a supported version PR deploys the current major. `1.2.3 → 1.3.0` updates `/v1/`; `1.3.0 → 2.0.0` adds `/v2/` and keeps `/v1/`. There are no public minor/patch-history URLs.
5. CI downloads the catalog and every retained major, verifies manifests/files by SHA-256 and size, stages the new release, then invokes Wrangler. Missing or corrupt history aborts before upload. Published HTTP bodies, MIME, CORS and cache policy are checked afterward.

Stable entry files and guides revalidate. Content-hashed chunks cache immutably; earlier chunks are retained for already-open pages. A hash URL cannot acquire different bytes. Same-version different assets and accidental downgrades fail. Old major directories are included in every deployment, rather than relying on Cloudflare deployment-history retention.

The package remains `0.1.0` in this workflow PR. Pre-1.0 versions run CI without publishing `/v0` or pretending they are `/v1`. Selecting the first supported 1.x version belongs to a separate release PR. Test-only or release-tooling-only changes may use `pnpm changeset add --empty`; public source/guide changes must have a real target release declaration.

## One-time repository setup

- Allow GitHub Actions to create pull requests. The repository currently uses a read-only default token and has bot PR creation disabled; the version job explicitly requests only its required write permissions. Keep the default read-only.
- Create environment `demistify-cdn` and restrict its deployment branches to `main`.
- Store `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` as environment secrets. Use a scoped token with the documented Workers deployment permissions; do not copy local OAuth/refresh credentials into CI. Never put secret values into Git, command arguments, logs or authored HTML. An existing 1Password wrapper can supply a value through stdin when configuring GitHub secrets.
- The job derives its workers.dev origin from the same configured account and Worker name used for deployment; no separately synchronized CDN-origin variable is needed. The token needs the documented account/Workers read access used by Wrangler as well as deployment access. Custom-domain delivery is outside this initial workflow.

Those settings and the first supported release are prerequisites, not established by adding YAML. No R2 bucket or persistent application server is needed.

## First supported release and local preparation

After a separate release PR selects a supported version, build that exact commit and prepare a first release only for a verified new `sunstack-demistify` Worker:

```sh
pnpm install --frozen-lockfile
pnpm build
pnpm release:prepare --bootstrap
pnpm exec wrangler deploy --dry-run --autoconfig=false --config packages/demistify/release/wrangler.json
```

`--bootstrap` only prepares local files; it is never an automatic fallback when historical fetch fails. Verify the target is new before an authorized first deployment. If it already serves a catalog, use the normal origin-preserving preparation instead. Do not run a bootstrap upload over existing history.

Normal preparation uses `DEMISTIFY_ASSET_ORIGIN` and refuses a nonempty output directory. Choose an empty directory with `DEMISTIFY_RELEASE_OUTPUT` for a separate local stage; pass that same directory as Wrangler's `--assets` override when needed. The script does not log in or upload.

The protected main deployment uses `wrangler deploy` only after checks and successful retention staging. `pnpm release:verify` compares the staged tree against the actual origin. A failed post-deploy check is a failed release; it is not automatically called rolled back. Fix and publish a new compatible version rather than rewriting an old published version. Retain the failed-run evidence.

## Proof boundaries

The focused release tests use real filesystem and loopback HTTP for retention/error scenarios. Local workerd verifies the actual Assets binding/Worker and HTTP headers. These do not prove hosted GitHub permissions, credentials, global CDN propagation or field performance. Build artifacts expire after 14 days and are not the historical-version store; retained paths on the CDN are the history.

The separate fixes workstream owns browser tests and new component APIs. This workflow builds runtime and guide from the same checked-out commit. Do not publish a newer guide under an older experimental pin.
