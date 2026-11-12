# Demistify standalone delivery: verification design

This plans how each proof obligation in the [Specification](specification.md) gets decided, against the components in the [Program Design](program-design.md). This repository's rule is the minimum meaningful checks plus direct proof through real paths. A few permanent tests guard the contracts that can regress silently. The incident files and real `pnpm dlx` runs prove the rest.

## Proof environments

| Environment | What is real | What stands in |
| --- | --- | --- |
| **Loopback release** | `stageRelease` output from this checkout's real `dist`, `skill` and `cli.tgz`, served by `wrangler dev` with the real `release/worker.ts` on `127.0.0.1`, plus real pnpm `dlx` and real hashes | the origin is loopback HTTP instead of the HTTPS CDN, which `publicOrigin` already allows for local proof |
| **Test loopback** | a Node HTTP server inside Vitest serving a staged release directory | the worker's headers (proved in the loopback release instead) |
| **Production read** | `https://demistify.getagentstudio.dev/v1/` served as it is today; the CLI only reads | nothing; this is the target. Agents do not deploy. |
| **Headless Chromium** | Google Chrome through the `chrome` channel, or Playwright Chromium 1.64.0 as CI installs it | nothing |
| **Fresh agent** | a new agent session with only the skill text under test and a task prompt | the reader's chat viewer: a passing `verify --no-network` stands in for Slack until the owner observes Slack |

## Proof cards

**V1 — `dlx` installs and runs the release's CLI** (R1, R2).

- Arrange: build the library and CLI. Stage a bootstrap release into `tmp/verification/demistify-cli/<run>/public`, then start `wrangler dev` on it.
- Act: run `pnpm --config.dlx-cache-max-age=0 --package=http://127.0.0.1:<port>/v1/cli.tgz dlx demistify --version`, then `--help`, from a directory outside the repository.
- Assert: the version line equals `packages/demistify/package.json` and the build's commit, and help lists all five commands.
- Falsifier: before the change, the same command found no archive. A `.ts` bin failed with `ERR_UNSUPPORTED_NODE_MODULES_TYPE_STRIPPING` (observed 28 October 2026).
- Evidence: manual proof, with the transcript kept in the PR body.

**V2 — the archive carries only the CLI** (R3, R4).

- Arrange: the V1 staging.
- Act and assert:
  - list the archive's contents, which must be exactly `package/package.json` and `package/bin/demistify.js`;
  - read the packed `dependencies`, which must contain no `@sunstack/` name, so installing the CLI cannot pull a runtime package (R4);
  - diff the staged `v1/` tree with and without `cliArchivePath`, which may differ only in `cli.tgz` and `release.json`;
  - build the archive twice, and require the same SHA-256 both times.
- Permanent test: `release-assets.test.ts` gains one case, in which staging without `cli.tgz` fails as an incomplete release.
- Falsifier: an archive carrying `runtime.js` or chunks, a `workspace:` dependency, or a second build with a different hash.
- Also run `check --hook` through the built CLI with a hook event naming a page that has a known error, and assert exit 2 with findings on stderr. This keeps today's hook contract.

**V3 — version guard** (R5).

- Permanent test: `release-library-reader.test.ts`, against the test loopback with a staged release whose version is one patch above the CLI's.
- Assert: the `release-mismatch` result carries both versions and the C4 rerun command, and no library file was requested.
- Falsifier: a reader that skips the version comparison returns file bytes instead.

**V4 — integrity and no partial output** (R6, R10).

- Permanent test: in the same file, replace one chunk's bytes after staging, and delete another file from the listing.
- Assert: the reader returns `library-file-integrity` and `library-file-missing` naming each path.
- Permanent test: `bundle-command.test.ts` spawns `node packages/demistify-cli/src/bin.ts bundle` at that release, with an existing output file.
- Assert: exit 1, the existing output file byte-identical, and no temporary sibling left behind.
- Falsifier: writing to the output path before the transform finishes, or trusting the download.

**V5 — the bundle renders with no network** (R7, R8). This is the incident proof.

- Arrange:
  - `/Users/shravan.sunder/Downloads/index.standalone.html`, the 28 October CDN-linked page, copied into the run directory as `incident.html`;
  - a PoC fixture with inline and file Markdown, a Mermaid island and an Explorer reading local JSON, such as `apps/demistify-poc/experiments/agent-written/navigation-markdown-fix/`, plus an Explorer example.
- Act:
  - bundle `incident.html` against the production binding with a CLI built at the version production serves (1.15.4 on 28 October; Changesets bump the version only in the later version PR). If production has moved on by then, bind a copy of `incident.html` to the loopback release instead; R5 refusing the mismatch is the designed behavior, not a failure;
  - bundle the fixture with `--library packages/demistify/dist`;
  - run `verify --no-network` on both outputs;
  - read each output's `<meta name="demistify-bundle">`.
- Assert: both outputs pass `verify`, which includes the fixture's Explorer reaching `data-ready`, and the screenshots show rendered Markdown, diagrams and the Explorer graph. The metadata reads `version=… source-commit=…` and `version=… source=local-build`.
- Falsifier: V8 shows the same verifier failing the unbundled incident file.
- Evidence: manual proof, with screenshots in the PR body.

**V6 — unbundleable inputs fail with nothing written** (R9).

- Permanent test: `bundle-command.test.ts`, one table-driven case per R9 input: a missing theme, split bases, an `http://` non-loopback base, an `<img>`, an unprepared Mermaid `src`, a remote Explorer, and a missing local Markdown file.
- Assert: exit 1, a message naming the element or path, and no output file.
- Falsifier: today's transform writes a bundle for some of these, which the test shows by missing the error.

**V7 — bundle-aware check** (R11–R13).

- Permanent test: `check-delivery.test.ts` in the library. Bundle a page that checks clean with `bundleStandaloneHtml(libraryDirectoryReader(dist))` and assert 0 errors and 0 warnings, including when prepared Mermaid islands still have `src`. A CDN-linked page checked with `pageFileName: 'x.standalone.html'` gives one `delivery` error. The bundle with a runtime script added back gives one `delivery` error. An unprepared Mermaid `src` on a file that already has bundle metadata also gives one `delivery` error.
- Falsifier: the current checker reports 1 `library` error and 121 `styling` warnings on a correct bundle (recorded finding F6), so the first case fails before the change.
- Also prove that the styling exemption covers only `<style data-demistify-theme>`: an authored `<style>` in the bundle setting `color` still warns.

**V8 — verify decides rendering, not loading** (R14–R16).

- Permanent test: `verify-delivered-file.test.ts` with real Chromium, three cases:
  - the V5 fixture bundle passes with `--no-network`;
  - a CDN-linked copy of the same fixture fails with `request-blocked` for `…/runtime.js` and `…/theme.css`, plus `library-not-loaded`;
  - a bundle whose Markdown island is changed to a missing `src` fails with `markdown-not-rendered` naming `ex-markdown#…`;
  - a bundle whose inlined Explorer JSON is made invalid after bundling (`bundle` itself validates it) fails with `explorer-not-rendered` naming `ex-explorer#…`.
- Permanent test: a case runs `verify-command` with `launch-chromium` given no usable channel or executable, and asserts exit 2 and the browser guidance. This is the only stand-in: a forced launch failure in place of a machine with no browser.
- Manual proof: `verify --no-network` on the two Downloads files. `index.standalone.html` fails, listing both blocked URLs. `address-mismatch-v2-workflow-map.standalone.html` passes with its four diagrams drawn.
- Falsifier: a verifier that checks only `load` or the exit status passes the CDN-linked copy when network is allowed.

**V9 — skills route agents correctly** (R17, R18).

- RED: already observed. Under the current skills, an agent produced `index.standalone.html` linked to the CDN.
- GREEN for the guide (after slice 4): a fresh agent with the local `packages/demistify/skill` gets this prompt: "make this page readable for a teammate who will open it in Slack", with a CDN-linked page attached.
- Assert: it runs `bundle`, then `verify --no-network`, reports the verify result, and names only the bundle `.standalone.html`.
- GREEN for `presentation-html` (slice 5, after release): the same prompt through `presentation-html`.
- The run follows `skill-review` with the same pass rule. The evidence is each run's transcript excerpt and the files it produced.

**V10 — maintainer journey** (R19).

- Run the new `verify-demistify` journey exactly as written in a clean worktree.
- Assert: it proves the bundle with `verify --no-network`, and its evidence directory holds the bundle, the result and the screenshot.
- Falsifier: a journey that serves over HTTP or omits `--no-network` would not appear in that output.

**V11 — delivery shape** (R20, R21).

- Run `gh pr view` for each PR and record its head, base, draft state, mergeable state, and that it is unmerged.
- For slice 5, run `git merge-base --is-ancestor 39957959 <head>` in `ai-tools`.

**V12 — batch bundling** (R22, R23).

- Arrange (permanent test, `bundle-command.test.ts`, against the test loopback release): `site/index.html` and `site/guide/details.html`. They link to each other, read Markdown files beside them, and carry a search index from `demistify index`.
- Act: `bundle --out-dir out site/index.html site/guide/details.html`.
- Assert:
  - it exits 0 and writes `out/index.html` and `out/guide/details.html`;
  - the test server logged each library file fetched exactly once;
  - both outputs check with 0 errors and 0 warnings.
- Act and assert: add an `<img>` to `details.html` and rerun with a new output directory. It exits 1 naming `details.html`, and the output directory is not created, even though `index.html` alone would bundle.
- Act and assert: these cases fail as `output-collides-with-source`, with the sources byte-identical:
  - `--out-dir site`;
  - the same source given twice;
  - `site/Index.html` beside `site/other/../index.html` on macOS.
- Act and assert: two pages bound to different bases fail as `bindings-differ` before any release request. The same two pages with `--library` bundle.
- Slice 3 adds: both outputs pass `verify --no-network`, and in Chromium the link in `out/index.html` opens `out/guide/details.html` from disk.
- Falsifier: a batch that writes `index.html` even though `details.html` fails, that flattens `guide/`, or that fetches the runtime once per page.

## Gates for every implementation slice

- `pnpm check` (lint, format and type checking) and `pnpm test`, with command, counts and exit code.
- `pnpm test:browser` when a slice touches the runtime-adjacent library code, or `verify`.
- `pnpm release:check`, and a Wrangler dry run for slice 2.

## Coverage

| V | R | Kind | Where it runs | Slice |
| --- | --- | --- | --- | --- |
| V1 | R1, R2 | manual, real `dlx` | loopback release | 2 |
| V2 | R3, R4 | manual and one permanent case | loopback release, `release-assets.test.ts` | 2 |
| V3 | R5 | permanent | test loopback | 2 |
| V4 | R6, R10 | permanent | test loopback, spawned CLI | 2 |
| V5 | R7, R8 | manual, incident file | production read, local build | 2 (bundle), 3 (verified) |
| V6 | R9 | permanent | spawned CLI | 2 |
| V7 | R11–R13 | permanent | library Vitest | 2 |
| V8 | R14–R16 | permanent and manual | real Chromium | 3 |
| V9 | R17, R18 | fresh-agent RED and GREEN | local guide; released guide | 4, 5 |
| V10 | R19 | journey run | clean worktree | 4 |
| V11 | R20, R21 | GitHub state | both repositories | each |
| V12 | R22, R23 | permanent | test loopback, spawned CLI, real Chromium | 2 (verify step in 3) |

## Not proven by agents

- **Slack and Grok rendering of a bundle.** This needs a share to those services, which `AGENTS.md` keeps with the owner. Verification stands in with `verify --no-network`, which is stricter than a policy that allows `blob:`. Closing it: the owner shares the V5 incident bundle in Slack and checks the console with `/slackdevtools`.
- **Production `dlx` of `cli.tgz`.** That URL exists only after the owner releases. Closing it: after release, the V1 commands rerun against `https://demistify.getagentstudio.dev/v1/cli.tgz`.
- **Windows, pnpm below 11, Node below 26.** These are out of the specified platform.
- **Source-mode type stripping through the workspace link.** The Program Design assumes the link resolves outside `node_modules`. V4 and V6 run the CLI from source, so they fail first if it does not.
