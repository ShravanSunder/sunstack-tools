# S4 — rehearse every install channel

Install two declared private releases, then observe what each real caller receives. Artifact integrity, installation and invocation are different claims: verify all three. The adopted runner R7/R16/R19–R23 and VD U2/U8/U10/U11/S4 govern this route; the current all-channel/stream/stop oracle is `8cf9c511a932cd41a5e8120b922bdd3a5aebeaee`. A later commission may repin it. Historical receipts retain their original source and oracle.

## Admit the source and private effects

The commission supplies the exact source, expected package `@sunstack/cli`, two normal SemVer tags, and authorized effects. The bin stays `sunstack`; `node_modules/.bin/sunstack` resolves to `../@sunstack/cli/run-sunstack.js`. The deprecated unscoped signpost has no release or alias journey.

Use only `ShravanSunder/sunstack-release-rehearsal` and `ShravanSunder/homebrew-sunstack-rehearsal`. Immediately before **each push or workflow dispatch**, use one checked sequence that aborts on any failed gate:

1. Read `gh repo view <repository> --json visibility,nameWithOwner` for both exact repositories; require PRIVATE and exact identities.
2. Read `gh variable list --repo ShravanSunder/sunstack-release-rehearsal --json name`; require `NPM_TRUSTED_PUBLISHING` absent. Read only names with `gh secret list --repo ShravanSunder/sunstack-release-rehearsal --json name`; require `HOMEBREW_TAP_DEPLOY_KEY`.
3. Verify original source/private-copy diff, exact local/remote refs and unused intended tag. Preserve the product deploy-key checkout, launcher replacement, checksums and **complete** npm variable/publication/non-prerelease predicate.
4. Perform only the commissioned mutation once, retain its result and read back its exact effect. Reconcile an uncertain result before any replay.

Private copies change only declared versions/destinations and separate verification jobs. Keep source-to-copy diffs and all workspace package versions together. Put source checkouts and normal debug output in their repository homes; task temp holds disposable downloads, installations and evidence, never copied source trees, Cargo target folders or Cargo homes. Follow the commission's unsigned/noninteractive and no-merge limits. Preparation alone grants no external mutation.

Run two sequential **non-prerelease** tags from the same original source, with older and newer versions. Confirm the plan says `announcement_is_prerelease: false`. Watch each with `gh run watch <id> --repo ShravanSunder/sunstack-release-rehearsal --exit-status --interval 240`. Require every normal release job to succeed, including Homebrew publication, announce and both fresh-install-smoke jobs, with **publish-npm skipped** because the variable is absent. Never publish to the real npm registry or production tap. Enabled OIDC publication stays outside this private rehearsal.

## Capture the pushed formulas and exact assets

After the older run, capture its actual CI-pushed private-tap commit and `Formula/sunstack.rb` **before starting the newer tag**. Repeat for newer. Retain commit/parent, version, run/job, formula bytes/hash and the successful deploy-key push evidence. Generated asset formulas alone cannot establish this route. Product brew-style formatting may change the pushed formula; retain that diff.

An authenticated host read may bridge those exact formula/provenance bytes into a separate private verification job. Check their hash again there. The bridge carries no credential. Retain every release job conclusion separately from verification jobs.

Download each tag with `gh release download <tag> --repo ShravanSunder/sunstack-release-rehearsal --dir <new-directory>`. Verify server digests and sizes, unified checksums and manifest, and run the existing `.agents/skills/verify-sunstack-rule/scripts/release_assets.py` from the monorepo root with the main skill's clean uv environment and explicit older/newer directories, versions and evidence. It must admit four macOS/Linux arm64/x64 archives and sidecar checksums, shell installer, formula, scoped npm package and schema. It requires exact name/version, bounded regular archive members and static Linux linkage. Its `s4-artifacts.json` is integrity proof, not installation. Older/newer native hashes must differ. Any mismatch stops installation; never regenerate a checksum to match an artifact.

## Serve a private registry and archive mirror

Run a loopback-only archive mirror with immutable older/newer paths and owned PID/port. Verify bytes **served over HTTP** against authenticated asset hashes. A container or fresh runner uses its own declared mirror, with no download credential inherited by the product.

All six npm invocation forms resolve `@sunstack/cli@<declared-version>` by name through a local private registry, including local and global installs. A local-file install or a cache-only shortcut cannot prove registry acquisition. Both versions remain available, with latest pointing at newer so an older pin can fail. Registry requests and served tarball digests must witness actual resolution/fetch.

The only permitted npm adaptation is a repack changing `package.json.artifactDownloadUrls` to the archive mirror. Require original/adapted tar digests, every member's before/after hash, exactly one changed member `package.json`, and equality after restoring that one URL field. Name/version/bin, launcher, postinstall, shrinkwrap and every other member remain identical. The registry serves that explicitly adapted tar, not an invented launcher. The archive location is a **stand-in**; real npmjs global/npx/dlx installs after release close it under the Lead's separate release grant.

A minimal registry or Verdaccio may serve the real client's metadata/tarball protocol. Bind only loopback, refuse unknown packages/paths, and permit no upstream forwarding during observations. Retain pinned dependency tarballs and their integrity checks, separate from the Sunstack artifact. Never expose a token, credential file or arbitrary directory. Install tools separately and record their source/version/hash; use pnpm `12.14.0` when installed.

## Install the real channels

Use isolated owned homes/configs/caches/prefixes/stores. An existing installation is a stop, never permission to overwrite it. A fresh client must use the registry, rather than an ambient project or global package. Record cold acquisition and all manager notices.

- **Homebrew:** start from the exact CI-pushed formula. Change only archive URL/mirror bases, preserving version/target/checksums. On disposable fresh runners, use `brew tap-new --no-git verifier/<owned-tap>`, locate it with `brew --repo`, copy the adapted formula, then `brew trust --formula verifier/<owned-tap>/sunstack` and install that formula. Every command uses owned empty `XDG_CONFIG_HOME`. Retain trust/install/uninstall outcomes and URL-only diff. Do not change the local Mac's Homebrew state.
- **Shell:** execute the tag's unchanged installer into an owned empty unmanaged directory using seams read from that actual installer. Keep original bytes, requests and unchanged HOME/profile hashes. Its embedded `APP_VERSION` pins the older installer even while newer bytes exist; invent no version flag.
- **Local bin and exec npm script:** install `@sunstack/cli@<version>` by name from the private registry in an empty consumer. Drive the declared installed bin and a script `"lint": "exec sunstack lint"` via `npm run --silent lint -- …`.
- **Global:** run a real `npm i -g @sunstack/cli@<version>` with an isolated owned prefix, then invoke its installed `sunstack`.
- **npx:** execute `npx @sunstack/cli@<version> …` against the private registry with a fresh owned cache; prove it was not satisfied by an ambient local/global package.
- **Exec-form npx:** execute `npx --yes --package=@sunstack/cli@<version> -c 'exec sunstack …'` through the real inner shell and private registry. Quote each literal CLI argument; preserve bare-npx observations as their own cells. An outer `exec npx` does not replace this invocation.
- **pnpm dlx:** execute `pnpm dlx @sunstack/cli@<version> …` against that registry with owned store/config/cache. Record any explicit package-specific build approval. Keep acquisition logging on stderr where the client supports it; do not discard it or repair stdout after capture.

For every installation, verify installed native SHA256 against the platform archive, launcher identity and exact `sunstack <tag-without-v>` output. An older request installing newer bytes fails.

## Observe every ordinary and stop cell

Fresh GitHub macOS and Linux runners must cover Homebrew, shell and all six npm invocation forms. All six npm invocation forms also run in the digest-pinned Node container; additional native Node-container and local-Mac runs are not required by the current commission. Record Node/npm/pnpm/shell versions and image digest. Native execution uses an absolute installed binary, empty HOME and `env -i PATH=/usr/bin:/bin`, without DYLD overrides. Package-manager routes need their explicit tool/local-bin PATH; record that exception. Fresh-hosted claims require actual hosted job evidence. Preserve Rosetta x64 and local macOS arm64/native applicability and S2 exact-archive joins.

For each version × route × platform, keep separate cells for version, clean exit0, finding exit1, **invalid config** exit2 and incomplete exit3. Compare stdout **byte for byte** with native on identical fixture paths/state. Compare complete parsed reports too, including findings, config/version, evidence and usage. Native stderr lines must be present; preserve and attribute extra package-manager stderr rather than suppressing it. Use `npm run --silent` so npm's own header does not alter stdout. Join installed-bin → native equality to a real CLI → library test only after inspecting its real boundary/assertions and current source/dependency/fixture applicability. Name reuse rather than claiming a tag-specific library rerun.

For Homebrew and shell, group Ctrl-C must give exit4 with the full cancelled report. Also retain the caller-only INT/TERM/HUP observations required by the commission. For npm use this adopted R22 matrix; “4” requires the full cancelled JSON and no surviving owned run/socket before cleanup.

| Route | Group SIGINT | Caller-only SIGINT | Caller-only SIGTERM | Caller-only SIGHUP |
| --- | --- | --- | --- | --- |
| Local installed bin | 4 | 4 | 4 | native ends; caller ends by SIGHUP |
| Global installed bin | 4 | 4 | 4 | native ends; caller ends by SIGHUP |
| Exec npm script | 4 | 4 | 4 | not passed on |
| npx macOS | 4 | 4 | 4 | not passed on |
| npx Linux | full cancelled report; npx SIGINT/130 | not passed on | not passed on | not passed on |
| Exec-form npx, macOS/Linux | 4 | 4 | 4 | not passed on |
| pnpm dlx | 4 | 4 | 4 | native ends; record caller end |

“Not passed on” means **no cancelled report**. Observe after a declared bound and retain whether the native run/caller remains alive or ended, when, and its actual exit/signal; their fate is recorded, not graded. A limit row is not a passing cancellation claim. The Linux plain npm script without exec remains a separate control/limit, not another required npm invocation. Preserve historical controls with their original targets.

Each stop starts only after a real admitted synthetic held backend request establishes readiness. Record actual request state, root/launcher/native PIDs and group, exact signal destination and time, complete stdout/stderr, actual exit **and signal**, native survivors and sockets **before cleanup**. Compare the full cancelled JSON and evidence state with the actual admitted request where a cancelled report is required. A sleep or a live PID alone is not readiness; a cleanup kill never turns a survivor into a pass. Every group Ctrl-C must end its native run.

Run `node scripts/verify/sunstack-rule/release/release-channels.ts --help` for the lasting channel driver. Its arguments name the declared full product `--commit`, downloaded `--assets`, extracted preflight `--native-root`, permanent `--fixtures scripts/verify/sunstack-rule/release/fixtures`, a new `--evidence` directory, the locked `--dependency-tar`, and `--channel npm|shell|brew`. The npm route also requires the exact `--pnpm` executable. Copy the unmodified preflight `s4-artifacts.json` to the assets root; its extracted older/newer/target binaries stay under native-root. The assets root's older/newer directories also carry `ci-pushed-sunstack.rb` and `tap-provenance.json` for Homebrew. Supply an original dependency tarball matching the package shrinkwrap's integrity, not repacked dependency files. The driver owns the registry lifecycle; separate registry, process and Homebrew modules own those mechanics. The recipe supplies the fixed expected matrix; helpers do not learn it from the observed product. Exercise each helper alone before composing the hosted/container workflow, and retain deterministic rejection/sensitivity controls.

## Observe actual GitHub Actions cancellation

Use the verification-only `scripts/verify/sunstack-rule/release/release-actions-workflow.yml` in the private repository. Pin its newer tag/commit to the declared release, keep the template/controller reviewed, and run actionlint before dispatch. Run it twice on the same verifier head, once in `exec` mode and once in `plain` mode. Recheck both repositories and the unset npm variable immediately before each workflow push or dispatch. The product Release workflow is a different workflow and must never be cancelled by this observation.

The contents-read preparation job authenticates the declared release/tag and every downloaded asset before bundling only those bytes, the small controller and the literal fixture. The cancellation job has **actions:write only**. An earlier setup step starts a loopback archive/held-request endpoint and shell-installs the actual release build into an empty owned home. Its controller receives the job token; the installer and native process receive only their explicit minimal environments. Never persist a token or pass it to product arguments, config or environment.

The work step records its actual shell PID/group and ends with `exec env -i … sunstack lint --all --format json`, so the shell becomes the native program. The plain control uses the identical command without that exec prefix. Real request admission, exact installed-byte identity and observed native ownership trigger one `gh run cancel <own-run> --repo <exact-private-repository>`. The controller validates the actual private repository, workflow name/event, head and run immediately before that call. Do not cancel on an in-progress flag, PID alone or elapsed sleep, and never replay an uncertain cancellation.

Retain the command and actual API result, admitted request/state, actual shell/native topology, process/socket timeline through the declared 20 second observation bound and before-cleanup snapshot. A later evidence-only `if: always()` step waits for the frozen observation, cleans only recorded owned resources, and uploads safe artifacts. Missing admission, uncertain cancellation, missing observation/log/upload or a controller failure remains unknown/failed setup. Local fake-gh/HTTP/process controls prove apparatus only.

Download **both complete raw job logs**, including the held-work step. Parse the full exec cancelled JSON after removing only runner timestamps/framing; retain the original log. Compare every field with `cancel.expected.json`, changing only declared version/config, and compare its evidence state to the actual admitted request. A cancelled workflow conclusion alone proves no report or native behavior. The plain control is ungraded: record its actual report/ending/timing. If the exec and plain work/report/process outcomes match, tell the Lead with both records **before any Actions claim**. Matching conclusions alone are expected. Keep upload failures and independent log readback rather than manufacturing evidence during cleanup.

## Return and teardown

Return exact product/oracle/verifier/private-copy commits, two normal tags, run IDs and **every job conclusion**, exact CI-pushed tap commits/formula hashes, asset/repack/installed hashes, registry/mirror request evidence, both Actions run IDs/raw logs/admission/cancel/timeline/control records, tool versions, and a channel × cell matrix linked to each raw transcript. A transcript contains command, cwd, exact credential-free environment, stdout, stderr, exit/status or signal, readiness, processes and sockets before cleanup, then a separately recorded cleanup result. Missing or empty cells stay unknown.

Report each U claim as supported/unsupported/unknown and observed/reused/failed/limit/unreachable with its reason. No product fix or expected-outcome change follows from an observation; send those to their owners. Keep apparatus failures and original attempts, including remote uncertainty and protocol-order deviations.

Stop only owned process/container/mirror/registry IDs after consumers settle, confirm absence and reopen retained evidence. Keep private tags/releases intact unless cleanup is separately authorized. Public URLs, real registry/OIDC publication, Intel Mac hardware, Windows/other distributions/private CAs and fresh-agent skill behavior remain unverified unless separately observed. The Lead owns acceptance.
