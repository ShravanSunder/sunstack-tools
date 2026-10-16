# Cloudflare releases by major

This workflow serves the shared library through Workers Static Assets. Authors load two tags from one major path:

```html
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css" />
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script>
```

These URLs serve the initial experimental 1.0.0 release. The matching component guide is `/v1/authoring/SKILL.md`. The build includes its references, metadata, worker, fonts and lazy chunks. This does not upload authored explainers, publish npm packages, or install a framework for readers. The existing experimental Worker/pins are separate and unchanged.

## Version and release flow

1. A feature PR runs `pnpm changeset` and declares `@sunstack/demistify` patch, minor or major. The initial release is owner-approved `1.0.0`. While `/v1` is experimental, use patch/minor updates even for breaking vocabulary changes. CI rejects a planned major above 1 until the owner explicitly enables strict major-version compatibility. After that decision, incompatible API changes will use a new major and preserve the previous major directory.
2. PR CI checks formatting/lint/types, builds the complete library/demo, runs Node and pinned Chromium browser tests, validates the actual Changesets plan and compiles the Cloudflare Worker without credentials or upload.
3. After merge, Changesets opens a version/changelog PR. It uses no npm publish command and creates no Git tags. The workflow explicitly dispatches checks for the bot branch because PRs created with `GITHUB_TOKEN` do not start another PR workflow automatically.
4. Merging a supported version PR deploys the current major. `1.2.3 → 1.3.0` updates `/v1/`. After the owner enables strict major compatibility, `1.3.0 → 2.0.0` adds `/v2/` and keeps `/v1/`. There are no public minor/patch-history URLs.
5. CI downloads the catalog and every retained major, verifies manifests/files by SHA-256 and size, stages the new release, then invokes Wrangler. Missing or corrupt history aborts before upload. Published HTTP bodies, MIME, CORS and cache policy are checked afterward.

Stable entry files and guides revalidate. Content-hashed chunks cache immutably; earlier chunks are retained for already-open pages. A hash URL cannot acquire different bytes. Same-version different assets and accidental downgrades fail. Old major directories are included in every deployment, rather than relying on Cloudflare deployment-history retention.

The initial `1.0.0` package/changelog update is merged and deployed. Pre-1.0 candidates run checks without publishing `/v0`. Future public source changes and their consumed version updates stay in separate PRs. Test-only or release-tooling-only changes may use `pnpm changeset add --empty`; public source/guide changes must have a real target release declaration.

## One-time repository setup

- Allow GitHub Actions to create pull requests. Bot PR creation is enabled and the repository retains a read-only default token; the version job explicitly requests only its required write permissions. Keep the default read-only.
- Use environment `demistify-cdn`, restricted to deployment branch `main`. This environment and its exact main-only policy have been created and verified for this repository.
- Store `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` as environment secrets. A project-specific account-owned token has been created, verified and installed privately for this repository. Its policy grants Workers Scripts Write and Account Settings Read in the selected account, plus Workers Routes Write and Zone Read for `getagentstudio.dev` only. Workers script permission is account-scoped. Never copy local OAuth/refresh credentials into CI or put secret values into Git, command arguments, logs or authored HTML.
- The job derives its origin from the custom domain in the same Wrangler configuration used for deployment. Before fetching history it verifies through Cloudflare’s fixed API endpoint that this hostname is attached to the configured production Worker in the selected account. Missing, reassigned or ambiguous bindings abort. No separately synchronized CDN-origin variable is needed.

The initial product integration, version PR and explicit bootstrap are recorded in the [v1 deployment handoff](../../../docs/wip/demystify/2026-10-16-v1-deployment-handoff.md). Creating secrets and an environment does not prove a hosted deployment. No R2 bucket or persistent application server is needed.

## First supported release and local preparation

After the separate version PR records the approved initial `1.0.0`, build that exact commit and prepare a first release only for a verified new `sunstack-demistify` Worker. Its configured Custom Domain is `demistify.getagentstudio.dev`; the owner’s active zone and absence of existing DNS/domain bindings were checked. Recheck for conflicts immediately before first upload. Cloudflare manages the hostname’s DNS and certificate through the Custom Domain binding. See [Custom Domains](https://developers.cloudflare.com/workers/configuration/routing/custom-domains/).

Prepare and inspect the first release:

```sh
pnpm install --frozen-lockfile
pnpm build
pnpm release:prepare --bootstrap
pnpm exec wrangler deploy --dry-run --autoconfig=false --config packages/demistify/release/wrangler.json
```

`--bootstrap` only prepares local files; it is never an automatic fallback when historical fetch fails. Verify the target is new before an authorized first deployment. If it already serves a catalog, use the normal origin-preserving preparation instead. Do not run a bootstrap upload over existing history.

Normal preparation uses `DEMISTIFY_ASSET_ORIGIN` and refuses a nonempty output directory. Choose an empty directory with `DEMISTIFY_RELEASE_OUTPUT` for a separate local stage; pass that same directory as Wrangler's `--assets` override when needed. The script does not log in or upload.

The normal protected main deployment uses `wrangler deploy` only after checks, a matching existing Custom Domain binding and successful retention staging. First bootstrap remains a distinct authorized operator action; the normal job does not turn a missing binding/catalog into permission to overwrite or initialize a target. `pnpm release:verify` compares the staged tree against the actual origin. A failed post-deploy check is a failed release; it is not automatically called rolled back. Fix and publish a new compatible version rather than rewriting an old published version. Retain the failed-run evidence.

## Proof boundaries

The focused release tests use real filesystem and loopback HTTP for retention/error scenarios. Local workerd verifies the actual Assets binding/Worker and HTTP headers. These do not prove hosted GitHub permissions, credentials, global CDN propagation or field performance. Build artifacts expire after 14 days and are not the historical-version store; retained paths on the CDN are the history.

The separate fixes workstream owns browser tests and new component APIs. This workflow builds runtime and guide from the same checked-out commit. Do not publish a newer guide under an older experimental pin.
