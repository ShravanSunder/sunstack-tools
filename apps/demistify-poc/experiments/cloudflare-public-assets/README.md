# Public Cloudflare experiment

The owner approved public library assets and public sample pages on 2026-10-16. This dedicated Worker uses Static Assets; no account ID, token, credential path or custom domain is stored here. Wrangler uses its authenticated local CLI session.

From the monorepo root, after dependencies and the matching library build exist:

```sh
node apps/demistify-poc/scripts/prepare-cloudflare-public.ts
wrangler deploy --dry-run --autoconfig=false --config apps/demistify-poc/experiments/cloudflare-public-assets/wrangler.json
wrangler deploy --autoconfig=false --config apps/demistify-poc/experiments/cloudflare-public-assets/wrangler.json
```

The prepare command writes only ignored tmp/demistify-cloudflare-public/public. The three manifests pin e3a92d2, 16ac23d and c96a72e. The last two also include the canonical authoring guide and six references. A changed current build is rejected; choose a new release root deliberately. Historical bytes are fetched from their exact public pin only when needed and verified against their stored SHA256. Existing staged release/sample files are reused only when byte-identical. The mutable landing page revalidates. Keep previous release roots in this staging directory when publishing another version; this small experiment does not implement a complete archival release service.

The four round 12 pages use absolute URLs for the public library. The explorer JSON remains relative to each page. No per-page Vite project is needed. The published roots and real protocol/browser evidence are in the root WIP deployment report.

This is an experimental workers.dev distribution, with preview URLs disabled. It does not establish a custom-domain contract, global cache propagation, an automated release pipeline, a standalone checker package or stable guide acceptance. Do not upload private repository content, raw CLI output or credentials with later samples.

The latest sample set is round14/v3; all previous roots remain staged. See [the inspection correction report](../../../../docs/wip/demystify/2026-10-16-inspection-fixes.md) for actual public URLs, browser proof and limits.
