# Demistify — owner requests and acceptance

Updated 16 October 2026. This is the delivery checklist requested by the owner; checked items require evidence. Existing experiment results remain in their dated reports.

## Current work

- [x] Sidebar inspection leaves the original island visible and usable in the main document, for every component using the shared inspector. Verify code, diagrams and an interactive composition.
- [x] Code title, change counts and inspection controls remain readable in the main view and narrow sidebar.
- [x] Review comments use a neutral gray background with a severity accent in main, sidebar and fullscreen views. Added/removed code retains its separate diff styling.
- [x] Rebuild and publish corrected library assets under a new immutable Cloudflare path, preserving existing published paths.
- [x] Refresh several public Sol/Luna sample pages to the corrected assets; capture actual Chrome screenshots and inspect their interactions.
- [x] Create Sunstack `presentation-html` using `skill-authoring:skill-creation`: explain the public runtime/theme URLs, plain HTML composition, component reference retrieval, local viewing and static/browser validation. Complete independent spec/file reviews and representative authoring proof.
- [x] Update the handoff with actual URLs, commands, navigation behavior, source branches, proof and remaining limitations.
- [x] Commit and push scoped changes; verify remote heads and tell the owner which requests are complete.

Current UI/public-delivery evidence: [inspection fixes](2026-10-16-inspection-fixes.md). Skill source is reviewed and pushed at `f1898058` on `feat/presentation-html-cloudflare` in [Sunstack](https://github.com/ShravanSunder/sunstack/tree/feat/presentation-html-cloudflare). All nine independent implementation checks are complete, verdict great, no open accepted findings. Fresh Sol/Luna examples and measurements are pushed in `502fc82` on the Sunstack tools experiment branch. The skill is available in source; default-branch merge and installed-plugin refresh are not included.

## Broader experiment obligations — retain until separately proved

- [ ] Full planning-explorer acceptance against the original: search, selection/history, graph filters, synchronized tables and presentation parity. Existing source/static checks do not close missing browser comparisons.
- [ ] Extend Sol/Luna DX/context comparisons across repeated and complex cases. [Round 15](../../../apps/demistify-poc/experiments/agent-written/round-15-presentation-skill/index.html) completes the first bounded skill/reference, correction and screenshot cases: Luna 1,099 / Sol 1,416 authored source tokens, with guidance/data/library costs separate. Automatic installed routing and general reliability remain unverified. Round 16 additionally checks source/prepared/explorer layouts and copied inspection in real390×844 child contexts.
- [x] Run bounded repeatable first-transfer/reload and interaction measurements for the current pin: round 16 has 24 loading observations, eight real selections, actual transfer ledgers and separate hosted HTTP requests. [Results and limits](2026-10-16-loading-preparation.md). Cold browser process, WAN render, field INP and tail reliability remain unverified.
- [ ] Compare minimum-install delivery strategies and explain local/offline limitations. Public Cloudflare trial works; production release/retention/alias strategy and standalone checker distribution remain open.

## Already delivered

- [x] Public Cloudflare library and four Sol/Luna-derived sample pages, verified and pushed in `ffc8f3d`. See [deployment and evidence](2026-10-16-public-assets-deployment.md).
- [x] Separate sample pages and source/content counts; library implementation and referenced graph content counted separately.

No credentials, account identifiers or private operator logs belong in this checklist or public source.
