# Distribution and authoring experiments

Historical checkpoint: experiment design and first authoring runs. Later explorer, matched-model and local Cloudflare results are in [the results report](2026-10-15-explorer-results.md). PR #1 merged the existing library at 49686d9d30cf3290d7a396fb2ae4b4b3a065fcc8. Work branch: feat/demistify-distribution-authoring-experiments. No Cloudflare resources or deployments.

## Decisions already supplied

Cloudflare is the target. Merge the experimental library first, then use a new branch for deployment and validation experiments. Use fresh Luna agents as HTML authors. Keep the consumer install minimal and the reusable library separate from each page.

## Authoring experiment

Use the same fixed brief for a baseline (current small guide) and a treatment (same guide plus a minimal working starter and explicit validation completion command). Fresh gpt-6-luna high runs receive no previous conversation, identical content, matching runtime version, exact allowed inputs/output paths, and at most two correction rounds. Preserve first outputs and repaired outputs. Prevent library/source browsing from becoming hidden context; record all extra reads.

Measure the actual supplied guide/reference bytes and tokens, authored page and referenced-source tokens, invented APIs, static findings, browser failures, required facts and relationships preserved, repair rounds, and whether the page remains understandable. Two runs reveal failure modes, not a statistical winner. Do not equate file tokens with billed tokens.

Validate independently: parse/vocabulary/real IDs and relative assets; render/layout/lazy diagrams/tours at desktop and narrow width; brief-specific facts and relationships; human readability. Seed a small negative set (unknown attribute, duplicate ID, broken diagram/label-as-ID, omitted fact, reversed relationship, missing asset) plus valid controls before relying on the checker. Factual meaning remains a bounded review task.

Observed baseline gap: checkHtml accepts an ex-step whose target is only a Mermaid label word, not a node ID. An invented attribute is correctly rejected; a valid control has no findings. Do not repair the baseline before capturing its authoring results.

## Delivery experiment

Compare Workers Static Assets and Worker plus R2 with the same complete release artifact, same page and same version rules. Keep version roots immutable, aliases revalidated/short-lived, and explicit CORS on script/CSS/font/worker responses including redirects. Test exact pin, alias change, rollback, missing release/file, conditional request, cold/warm browser requests, worker and main-thread fallback.

Two independently moving CSS/script aliases do not pin a page atomically. Force an alias move between those fetches as a negative control. Compare exact-version tags with one bootstrap resolving a release once. Every lazy import and worker must stay within that chosen root.

Static Assets must retain old version folders in later deployments. R2 can retain them independently of the Worker deployment but adds object publication and alias-update ownership. Validate upload-before-alias and refusal to overwrite release files. An R2 GET is not automatically an edge-cache hit; local runtime timings do not prove edge cache, purge timing, geographic latency or production cost.

## Minimum consumer modes

1. Online HTML loads one versioned shared library with browser tags; no npm, Vite or app server required by the library.
2. A copied asset folder works from any static HTTP host with relative URLs. Direct local file behavior is a separate scenario because ES modules/fonts have CORS rules.
3. An optional packaging command creates an offline artifact. Verify its lazy chunks, fonts and diagrams offline before calling it self-contained.

Agent tools are optional and separate: a version-matched CLI can validate before browser review; the browser consumer does not inherit Node/pnpm. The current CLI requires Node26/source dependencies, so a one-shot standalone checker is an experiment, not an existing install promise.

## Decision rule

Reject any delivery strategy that loses old pinned pages, mixes release assets, or silently serves missing JS as HTML. Prefer the smallest operational setup among passing variants. Reject a validation strategy that passes seeded critical negatives or makes valid pages fail. No domain, account binding, stable URL/version contract, production cache duration or deployment is selected here.

## First acceptance-source freeze (historical)

The owner selected the current planning-dag.html, titled Sunstack · Skill composition explorer. Frozen SHA-256: 03734753690e789ccb57a9e8cd7d60dc94f6d382a03568cf1818f564ea7785cd; 586,306 UTF-8 bytes, 698 physical lines. Its embedded data names 48 skills, 256 declared edges, 2 proposals, 18 changes, 6 extractions and 2 candidates; it cites 175 inspected source files. This is the current interactive explorer, not the lab's older 25-node planning snapshot.

The final test must account for the prose and diagram/reference data, not only its HTML shell. Measure exact context actually supplied/read (guide, requested references, content, examples and validator feedback), complete authored output across all referenced files, and repairs separately. A reference does not make its payload free. Existing shared input, generated renderer output and reusable library code have distinct ledgers. Source-token counts use o200k_base and are not billed model usage.

The reference has a searchable skill browser, selection/history, current/proposed views, routed graph, hover trace, arrow-to-table links, incoming-use tables, source drawer, changes and remaining ideas. Existing islands cover many presentation pieces but do not establish a reusable search/selection/synchronized-table API. The owner is being asked whether full interaction parity is required or unsupported interactions should be reported as gaps in an existing-islands conversion. Do not build custom per-page application JavaScript or claim parity while silently dropping these features.

Browser control rejected the file URL on protocol policy. The source was inspected through the filesystem; no alternate browser surface or HTTP mirror was used to bypass that restriction. Direct visual comparison remains pending an allowed source view.

## Initial validator observations

The real checker at the merged implementation reports:

| Input                                         | Errors | Warnings | Meaning                      |
| --------------------------------------------- | -----: | -------: | ---------------------------- |
| Valid shell and explanatory content           |      0 |        0 | Valid control                |
| Invented card attribute                       |      1 |        0 | Caught                       |
| Duplicate document ID                         |      1 |        0 | Caught                       |
| Tour target that is only a diagram label word |      0 |        0 | Incorrect reference accepted |
| Missing image path                            |      0 |        0 | Asset not checked            |
| Empty content inside a valid shell            |      0 |        0 | Content coverage not checked |
| Claim that an in-memory Map survives restart  |      0 |        0 | Domain truth not checked     |

These are cheap diagnostic probes, not an exhaustive quality score. A factual validator needs a supplied brief or expected facts; a generic HTML checker cannot infer domain truth. Keep the baseline unchanged while measuring the authoring treatments.

## Cloudflare source checks

Primary docs were read through the official cloudflare/cloudflare-docs repository because the website reader returned 404 and the documentation host returned 403 in this environment.

- [Static Assets](https://developers.cloudflare.com/workers/static-assets/): asset binding and automatic edge caching; a credible alternative to R2, so R2 is not yet the chosen storage.
- [Headers](https://developers.cloudflare.com/workers/static-assets/headers/): default browser caching revalidates with ETags; custom _headers overrides and worker-generated responses have separate rules. Redirects run before _headers, so alias CORS must be explicitly checked.
- [Bindings](https://developers.cloudflare.com/workers/static-assets/binding/): ASSETS.fetch can serve assets; run_worker_first chooses whether dynamic code precedes asset delivery.
- [R2 bindings](https://developers.cloudflare.com/r2/api/workers/workers-api-reference/): conditional get/put and HTTP metadata are available; adding an R2 bucket does not by itself establish an edge-cache strategy.

DeepWiki supplied pointers for wrangler dev and Miniflare; no local workers-sdk checkout was available. Local experiments will establish protocol/runtime behavior, while production caching, rollback propagation and cost remain unverified until an explicitly authorized deployed trial.

## First Luna probe: observed results

Fresh `gpt-6-luna`, high effort, no inherited conversation. Same code-review brief and checker; only the starter differs. Exact input files and initial/final HTML are preserved under [round-7-luna](../../../apps/demistify-poc/experiments/agent-written/round-7-luna/README.md).

| Arm             | Reported input-file tokens | First HTML tokens | Final HTML tokens | Checker calls | Final static result  | Browser observation                                            |
| --------------- | -------------------------: | ----------------: | ----------------: | ------------: | -------------------- | -------------------------------------------------------------- |
| Current guide   |                      2,248 |             1,571 |             1,558 |             2 | 0 errors, 0 warnings | No captured warning/error at the observed page                 |
| Guide + starter |                      2,251 |             1,590 |             1,556 |             3 | 0 errors, 0 warnings | ex-tree warning: HTML list supplied where Markdown is expected |

Input-file totals include the brief, core guide and references each worker reported reading; the starter adds its shell, while the baseline also read the tours reference. These are source-token counts with o200k_base. Prompts, tool framing, validator feedback, hidden system context and provider billed usage are not in this table. The final acceptance run must instrument the visible prompt/reference/feedback transcript from the start; do not call these lower-bound input-file totals total model context.

Both first checker invocations reported errors but lost their numeric exit codes because the workers used zsh's reserved `status` variable in a wrapper. Later calls captured exit 0. The starter made a third checker call after already reaching zero; this is observed agent overhead within the assigned cap. These limitations are preserved in checker-results.json.

Parent source inspection found the required F1–F5 findings, three options and the complete evidence values in both pages. A real browser run proved why static success is insufficient: the starter's HTML ul remained ordinary list content and the ex-tree component reported a source warning; it did not become the intended themed tree. The initial and final outputs remain verbatim, including this failure. No library fix was added to make the experimental evidence look clean.

Next validation comparison: the same authoring task with an explicit tree-source example and one completion command that combines static checks, runtime issues and brief-specific coverage. Require a valid control and seeded invalid cases before treating the completion command as an oracle. Human readability remains owner-reviewed. A one-shot starter alone did not establish improved reliability in this probe.

## First delivery probe: alias consistency

Run from the monorepo root after building:

```sh
node apps/demistify-poc/scripts/probe-delivery-aliases.ts
```

This uses real loopback HTTP redirects and two named release roots containing identical frozen library bytes. Payload size is held constant. It forces an alias update between the CSS and runtime requests.

Observed: independently requested aliases returned CSS from 0.1.0 and runtime from 0.1.1. Exact-version tags stayed together. Resolving a release once then loading both exact URLs stayed together even when the alias changed. The selected worker shared that root; rollback selected the old root; a missing asset returned 404/no-store; the redirect carried CORS. Seven protocol conditions passed, exit 0.

This does not execute browser modules or emulate Workers Static Assets, R2, cache expiry or global rollback propagation. It establishes a host-independent consistency defect and candidate fixes. The Cloudflare backend comparison is designed above and has not run; choosing a backend remains open.

The frozen final reference measures 174,303 source tokens, including 151,645 in its embedded data. These are full-file sizes, not context actually consumed by an authoring agent. Reusing the data is valid; reading or generating it must appear in the corresponding ledger.

## Current proof and delivery boundary

At the experimental branch, pnpm check passed (lint, formatting and strict TypeScript). The complete lab built successfully; the later extra input/receipt files were packaged with the existing runtime. The protocol probe passed all seven named conditions. No new automated suite was added; the existing 29 tests passed at the merged baseline. Both Luna pages were opened through the real runtime over loopback HTTP. The starter's runtime warning is preserved as an experimental result, not presented as a finished explainer.

No domain, Cloudflare resource, credentials, paid API request, public deployment or stable distribution URL was created. Browser access to the owner-supplied file URL remains blocked by host protocol policy. The full final conversion and controlled Workers Static Assets/R2 trials remain future experiments; this document does not claim them complete.
