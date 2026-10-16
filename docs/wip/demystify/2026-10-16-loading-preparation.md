# Loading, layout movement and optional preparation

16 October 2026. Current hosted library pin: c96a72e. Experiment sources/results: [round 16](../../../apps/demistify-poc/experiments/agent-written/round-16-delivery-performance/index.html). This continues the owner’s load-time, jank, minimal-install and understandable-layout questions. No runtime, style default, public authoring API, installed skill or Cloudflare deployment changed.

## What the measurement found

The source review displays substantial content movement while Markdown islands render. Its largest diagnostic layout shift happened after Markdown readiness and before the SVG was inserted. The outline’s late gutter also moves the document, but declaring that existing component early did not remove the dominant shift. Optional preparation is therefore a useful separate delivery experiment; a new default is not selected here.

| Page / delivery       | Fresh URL ready (ms) | Same URL reload ready (ms) | Fresh shift sum      | Library body bytes, first transfer |
| --------------------- | -------------------- | -------------------------- | -------------------- | ---------------------------------: |
| Full explorer         | 87.5,79.3,89.8       | 39.9,37.2,32.3             | 0,0.0257,0.0257      |       358,656 + 776,437 graph data |
| Source review         | 341,465.5,259.8      | 273.3,448.9,237.9          | 0.515,0.676,0.676    |                          2,528,730 |
| Explicit outline      | 332.2,255.2,264.3    | 276.6,238.4,237.6          | 0.648,0.669,0.670    |                          2,528,730 |
| Prepared Markdown/SVG | 43.3,26.2,29.4       | 19.1,18.7,20.3             | 0.0350,0.0414,0.0348 |                            275,401 |

Each row has three fresh/reload pairs in one existing unthrottled Chrome process at 1,398×747 CSS pixels. Unique local URL roots forced real transfers of the exact immutable published library bytes; paired reloads transferred zero library/data bytes. The server body ledger excludes headers/compression and separates probe/HTML/data/library costs. No main-thread long tasks were observed in these windows, which is not a universal absence claim. These are bounded loopback observations, not a cold process, WAN rendering benchmark, field Core Web Vitals or statistical model winner. Readiness is distinct from FCP, fonts and later paints.

A separate native Worker diagnostic received actual ready and rendered messages, with an 8,570-character SVG reply. Resource entries were not treated as worker-execution proof. The source starts a fresh diagram worker for a new document; cached network bytes do not remove every initialization/render step. The timing series does not isolate worker CPU phases.

## What preparation pays for

The unchanged Luna-authored input is 4,924 bytes / 1,099 o200k_base source tokens. The explicit outline input is 4,975 bytes. The generated prepared page is 13,922 bytes, with Markdown/SVG already present and the shared runtime still owning interactions, theme and inspection. Agent-authored usage, generated output and shared renderer implementation remain separate costs.

Run from the monorepo root:

```sh
node apps/demistify-poc/scripts/prepare-delivery-performance.ts OUTPUT_DIRECTORY
```

This is a fixed PoC preparation command using the existing library renderer and diagram preparer. A small Node-only normalization matches bare-text adoption for this review’s document/card/callout owners; it is not a general compiler or new authoring vocabulary. parse5 7.3.0 is declared directly as a PoC dev dependency and was already installed through the library. No new browser dependency or framework is introduced. The prepared body is byte-identical to the measured fixture body; openable copies differ only in using the actual public asset URLs and omitting measurement instrumentation.

The authoring checker accepts the original/outline inputs. It rejects the generated prepared output with 29 errors and 3 warnings because internal prepared attributes and renderer-produced HTML/SVG/styles are outside authoring syntax. Keep source validation before generation, then validate generated output through its actual browser path. Do not teach those internal attributes as agent API or weaken the source checker to manufacture a pass.

## Actual browser and narrow proof

The generated page has six headings/outline links, one real SVG and nine annotated diff rows. Sidebar inspection retains nine original rows and shows nine copied rows. A real 390×844 child browsing context provided narrow-layout proof without changing the unavailable global viewport override: the source review, prepared review and full explorer each stayed inside a 376px scrollable client area. The narrow inspector uses a dialog sheet while retaining the original content. Keyboard close and a screenshot-based visible pointer click left the original nine rows.

A locator’s automatic scrolling moved the tall iframe toolbar out of the parent viewport, so its initial close click did not activate the control. The actual rects showed the iframe at y=-48 and its close button at child y=13. Restoring outer scroll and clicking the visible icon directly succeeded. This is a diagnosed driver-positioning limit, not a demonstrated broken library handler. This proof is not a real-device or complete accessibility audit.

Eight real explorer selections through its user controls measured synchronous work of 3.7–9.9ms and two-frame settlement of 13.7–32.2ms, with no observed main-thread long tasks in that window. The latter is the library’s existing two-requestAnimationFrame measure, not standardized INP.

## Current hosted transport, measured separately

Three anonymous sequential compressed GETs per URL from this machine:

| Asset                   | Decoded bytes | gzip body bytes | Observed total request time (ms) |
| ----------------------- | ------------: | --------------: | -------------------------------- |
| theme.css               |        61,699 |          12,514 | 219.7,209.1,133.7                |
| runtime.js              |        84,294 |          23,945 | 126.7,126.3,234.9                |
| optional diagram engine |     2,195,817 |         506,562 | 501.7,454.1,526.8                |
| full planning graph     |       776,437 |          96,500 | 185.7,260.7,252.7                |

All returned 200 with the intended MIME, immutable cache policy, CORS and Timing-Allow-Origin. Independent connections and existing edge caches were not controlled as edge-cold. These numbers are not the browser render measurements above, multi-region results or production cost evidence.

## Full-reference acceptance remains bounded

The frozen 792,811-byte reference SHA619a6b0a is retained privately; the current graph SHA8c556fa2 still matches 100 node/mode composition cases with zero mismatches. All 1,870 long text values, 714 exact source/quotation anchors and 350 corresponding relationship identities remain preserved. These probes reuse the existing source oracle without browsing, mirroring or executing the original application.

The external original file is now a different 1,274,902-byte revision. It was opened in Chrome through the app-open route explicitly authorized by the owner. Browser control nevertheless refused to inspect the file protocol even after that authorization, explicitly prohibiting alternate-surface workarounds. No bypass was attempted. Visual comparison against an allowed original view remains open; the 100-case result is not claimed against the newer file.

## Validation and delivery

The optional preparation command ran through its real CLI and emitted exact hashed files. Its first scratch ordering error (prepared Markdown fed back into an author-source helper) was corrected before preserving the typed command. The first timing extraction failed because the read-only browser scope omitted the Performance API; all recorded measurements use an explicit JSON DOM export. Failed attempts are not counted as successful samples.

Required checks pass: pnpm check (lint/format/strict types); pnpm test (34 tests/9 files); three authored index/baseline/outline pages, checker0 errors/0 warnings. Generated output’s separate author-checker rejection is retained above. No decorative test suite was added and no gate was weakened. The full lab build also passes. Source delivery uses the existing feat/demistify-distribution-authoring-experiments branch.

Production defaults, a standalone checker distribution, broad repeated-model reliability and original visual parity remain separate work. The immediate result is inspectable delivery choices and an observed cause of jank, with measured costs and limits.
