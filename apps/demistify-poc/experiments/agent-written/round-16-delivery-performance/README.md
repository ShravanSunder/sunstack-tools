# Delivery performance — round 16

This is a fixed experiment, not a production packaging feature. Source library remains c96a72e. Each page opens independently; the generated prepared page uses the same shared public runtime/theme as the source. No runtime, API, style default, installed skill or deployment changed.

Run from the monorepo root:

```sh
node apps/demistify-poc/scripts/prepare-delivery-performance.ts /absolute/path/to/ignored-output
pnpm build
pnpm preview --port 4174
```

Open `/experiments/agent-written/round-16-delivery-performance/index.html`. The preparation CLI reads the frozen round-15 Luna source and emits baseline.html, explicit-outline.html, prepared.html and a manifest. It uses the existing Markdown renderer and diagram preparation function; a small Node-only transformation matches bare-Markdown adoption for this review's document/card/callout owners. It does not claim to handle arbitrary assets, explorer packaging or every island.

The generated prepared body is byte-identical to the prepared body used in the private measurements. The measurement fixture changed only asset roots to relative private copies and added a timing observer. Here, the copies point at the public shared assets for easy viewing. Source hashes and generated sizes are in preparation.json. Its internal preparation timing excludes Node startup/imports and is not a compiler benchmark.

## Repeating the measurement

Use three fresh URL roots per page, each containing the complete library verified against interactive-release-manifest.json, and the same source/diagram/graph content. Add observers before the runtime for paint, largest-contentful-paint, layout-shift, longtask, resource and measure entries; wait for actual prepared SVG / explorer data-ready, Markdown readiness and document.fonts.ready, then two frames. Capture readiness separately from fonts and paints. Paired reloads use exactly the same URL. A loopback-only static host supplies immutable asset caching and records actual response entity bytes. Verify fresh library/data bytes versus zero warm bytes; do not label a cache-disable flag cold without that evidence. Instrumentation body bytes are separate in summary.json.

Recorded samples: 24 navigations, three paired runs for source review, explicit navigation, prepared review and full explorer. Effective normal CSS viewport 1,398×747. No throttle or fresh browser process; one existing Chrome process, unknown background load. No production latency, field Core Web Vitals, standardized INP, tail reliability or general performance winner is established. Public curl timings are anonymous sequential GETs with compression and independent connections, not forced edge-cold or browser rendering. FCP can occur before the actual diagram becomes ready; treating those as equivalent would hide the measured delay.

## Validation and actual limits

The authoring checker accepts the baseline and explicit-outline input. It rejects the generated prepared page with 29 errors / 3 warnings: internal prepared attributes, rendered Markdown/SVG and library-generated SVG styles are outside its authoring vocabulary. This is a stage boundary, not a reason to add those internal attributes to agent instructions or weaken source checking. Generated output was checked in the real browser: six headings/outline links, one actual SVG, nine annotated diff rows, preserved main/sidebar rows and no outer overflow.

The first scratch preparation attempt passed already-prepared Markdown back into a helper that expects inert author source. It failed explicitly; preparing diagram sources before bare-text normalization fixed the experiment. The retained successful CLI encodes that order. The original first timing read also failed because the browser read-only evaluation scope does not expose the Performance API; the fixture exported a JSON DOM record instead. Neither failure counts as a successful load sample.

A real 390×844 child browsing context supplies responsive proof without relying on the ineffective global viewport override. Its scrollable client area was 376px; all three documents stayed within it. The narrow inspector uses a dialog sheet and kept nine original + nine copied rows. An automation locator auto-scrolled the tall parent iframe, moving its Close toolbar out of view; keyboard close and a real screenshot-based pointer click after restoring parent scroll both closed it, leaving nine original rows. This was a test-driver positioning issue, not a demonstrated component close-handler failure. It is not a complete mobile-device or accessibility audit.

The largest diagnostic source-page shift happened after Markdown readiness and before the SVG was inserted. Declaring the outline alone did not remove the dominant shift. Generated preparation substantially reduced it in these samples while increasing delivered HTML and adding a preparation step. Runtime-only input remains available and no default was selected.

The separate native Worker wrapper forwards construction to the real browser Worker and records ready/rendered replies; the observed render returned 8,570 SVG characters. It ran only in a diagnostic document, outside the timing series. Network entries alone were not treated as proof of worker execution. All measured pages reported no authoring/runtime issues; no main-thread long tasks were observed in the captured loading/selection windows, which is not a universal absence claim.

Current content/parity receipts use the retained 792,811-byte reference SHA619a6b0a and current 776,437-byte graph SHA8c556fa2. The external original is now a different 1,274,902-byte file; these results do not claim parity against that newer input. No original app script/styles were copied, served or republished. Its pure composition function was reused in the existing source oracle without browser/IO.

No credentials, account metadata or raw private provider/action logs belong in this round. Referenced graph and shared runtime are existing separate inputs; generated HTML, library and instrumentation costs are not agent-authored token usage.

Exact measurement observer sources are retained in observer-source.js.txt and observer-prepared.js.txt. The loopback-server.py.txt recipe is the recorded server with only its task-root path replaced by a command argument; it listens on127.0.0.1:4190. These are experiment artifacts, not browser-consumer or runtime-library dependencies.
