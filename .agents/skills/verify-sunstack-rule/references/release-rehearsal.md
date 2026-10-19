# S4 — install the exact private release

Observe two real generated releases through Homebrew, shell and npm. Exact bytes establish what was installed; the invocation determines which process receives a signal. Observe both separately. Expected outcome: runner R7/R16/R19–R23 and VD U2/U8/U10/U11/S4, with adopted B′ signal/install revision `e010a45cf19c376b7bad820fb761c54d9eec025b`. The release workflow is product-owned; this recipe owns verification fixtures, mirrors and inspection. Artifact preflight never proves installation.

## Admit the source and private effects

The commission names the adopted package/alias contract, exact product source, two tags, expected npm package identity and authorized effects. Keep the `sunstack` bin distinct from its package name. An unsettled package/alias decision or missing successor SHA holds all tag pushes; continue independent preparation. Never infer expected identity from the artifact under test or choose alias behavior. Historical observations retain their original target and oracle.

Use only `ShravanSunder/sunstack-release-rehearsal` and `ShravanSunder/homebrew-sunstack-rehearsal`. Before **each remote mutation**, check both with `gh repo view <repository> --json visibility,nameWithOwner`; stop on another destination or non-PRIVATE result. Preparation authorizes no push, tag, publication or paid call; the execution commission must grant them. Never write the production tap or real npm registry.

Before an authorized tag push, independently confirm every source correction, including a post-merge head when requested. Check **names only** with `gh secret list --repo ShravanSunder/sunstack-release-rehearsal --json name`: require `HOMEBREW_TAP_DEPLOY_KEY`, never its value. Preserve product deploy-key checkout, launcher replacement, checksum gates and npm prerelease guard. Private copies may change only declared destination/version inputs, `publish-prereleases = true`, and separate verification jobs. Retain each source-to-private-copy diff and commit.

Run two sequential prerelease tags from the same original source, with distinct matching workspace/package versions. Watch each with `gh run watch <id> --repo ShravanSunder/sunstack-release-rehearsal --exit-status --interval 240`. Require successful `publish-homebrew-formula` and its actual private-tap commit per tag. `publish-npm` must be **skipped**, even though `publish-prereleases` enables Homebrew. A skipped tap job leaves publication unknown; an executed public npm job violates the rehearsal boundary. Retain every job conclusion; keep verification jobs separate from product release jobs.

## Capture both tap commits and downloads

After the older run, independently read back the exact private-tap commit and `Formula/sunstack.rb` **before** the newer tag can replace it. Repeat for the newer run. Keep commit/parent, version, run/job identity, pushed formula bytes/SHA256 and its link to the deploy-key publish job. Product brew-style formatting may change the release formula; retain the diff and test the pushed bytes.

Access the private formula through the authorized CI deploy-key checkout or an authenticated host read of that exact commit. A host read may supply retained formula/provenance bytes to the private verification job; require that job to verify their hash before installing. This bridge carries no credentials. A release asset or hand-authored formula alone does not prove installation from the CI push.

Download each tag with authenticated `gh release download <tag> --repo ShravanSunder/sunstack-release-rehearsal --dir <new-tag-directory>`. The mirror receives no token or auth header. Require, per tag:

- `sunstack-<target>.tar.gz` plus matching `.sha256`, for macOS arm64/x64 and Linux musl arm64/x64;
- `sunstack-installer.sh`, `sunstack.rb`, `sunstack-npm-package.tar.gz` and `sunstack.lint.schema.json`.

From the monorepo root, use the main skill's clean uv environment:

```sh
uv run --no-config --no-env-file --no-python-downloads --default-index https://pypi.org/simple --script \
  .agents/skills/verify-sunstack-rule/scripts/release_assets.py \
  --older <older-download> --older-version <older-version-without-v> \
  --newer <newer-download> --newer-version <newer-version-without-v> --evidence <new-evidence>
```

The helper requires npm identity `@sunstack/cli` exactly and the caller-declared tag version. The artifact remains `sunstack-npm-package.tar.gz`; the bin remains `sunstack`, with `node_modules/.bin/sunstack` pointing to `../@sunstack/cli/run-sunstack.js`. The unscoped npm name is an owner-managed deprecated signpost outside release ownership, with no alias journey. Never accept arbitrary actual names or derive the expected name from the package itself.

The helper checks archive checksums, reads exactly one bounded regular `sunstack` member without extracting arbitrary paths, retains native bytes/hashes, checks static Linux linkage and formula/npm/installer metadata. Older/newer native hashes must differ so version pinning can fail. `s4-artifacts.json` leaves installation unobserved. Missing assets or checksum/metadata mismatch stops before installs; never rewrite checksums.

## Mirror without replacing product behavior

Serve only authenticated download directories on loopback, with owned PID/port and immutable paths for both versions. Verify served archive hashes against preflight. A hosted runner uses its own mirror; a container uses its declared connection to that mirror. No public exposure.

Adapt download locations only, retaining originals and diffs:

- **Homebrew:** start with the exact CI-pushed formula. Change only `url`/`mirror` bases to that tag's loopback directory; retain version, targets and real checksums. On the disposable fresh runner, create `brew tap-new --no-git verifier/<owned-tap>`, locate it with `brew --repo verifier/<owned-tap>`, and copy adapted bytes unchanged to `Formula/sunstack.rb`. Use an owned empty `XDG_CONFIG_HOME` for every Homebrew command. Run `brew trust --formula verifier/<owned-tap>/sunstack`, then `brew install --formula <tap-path>/Formula/sunstack.rb`. Trust only that formula and retain checksum validation. Uninstall the owned installation before the next version. Never overwrite another installation or mutate the local Mac's Homebrew state.
- **Shell:** use each tag's unmodified installer; test the **older** installer while newer release bytes are available. cargo-dist 0.32.0 embeds `APP_VERSION`; selecting that older installer/URL pins its version. `INSTALLER_DOWNLOAD_URL=<older-mirror-directory>` redirects archive downloads. Read its supported unmanaged/install-directory variable before choosing an owned empty directory. Keep archive requests, installed hash and unchanged home/profile evidence; invent no version flag.
- **Npm:** unpack the generated artifact in scratch, change only `package.json.artifactDownloadUrls`, then `npm pack <adapted-package-directory> --pack-destination <scratch>`. Keep file hashes/diff proving name/version/bin map, launcher and postinstall code unchanged. Run `npm i -D <packed-tarball>` in an empty consumer. Resolve the package from its **declared expected identity** and `node_modules/.bin/sunstack` from its bin map; prove the link reaches the unchanged launcher and the native hash matches preflight. Drive that installed bin directly and an npm script `"lint": "exec sunstack lint"`. Installation-time GitHub access, package writes and pnpm approval remain README obligations.

The mirror stands in for authenticated download locations while preserving release native bytes and installer behavior. Public URLs remain unverified. If a supported mirror seam is missing, return the gap instead of modifying product installers or substituting a binary/launcher/package.

## Drive fresh platforms and the npm matrix

Run Homebrew and shell on fresh GitHub-hosted macOS/Linux, and npm on both plus a clean digest-pinned Node container. Retain platform, image/digest and Node/npm/shell versions. Native commands use absolute installed paths under `env -i PATH=/usr/bin:/bin HOME=<new-empty-home>`, without `DYLD_*`. Invoke the installed bin with absolute Node and bin paths under that environment. Npm scripts additionally need the declared Node/npm directory in PATH; record that exception instead of claiming OS-only PATH. Setup may use runner tools; product execution never inherits download credentials.

For each version × channel × platform:

1. Installed native SHA256 equals the matching extracted release; `--version` equals `sunstack <tag-without-v>`. Formula/npm versions match the tag. An older request installing newer bytes fails.
2. `lint --check` exits 0; mechanical error lint exits 1 with the exact finding; clean lint exits 0. Direct-bin output/exits 0–3 match native, including invalid input and a real incomplete case. Compare parsed native/bin JSON on **identical fixture paths/state**, every field including config/version/findings/evidence/usage. Join installed-bin → CLI equality to the real Rust CLI → library equality only after checking the actual test boundary/assertions and source/dependency/config applicability; a producer claim alone is insufficient.
3. For every Homebrew and shell installation, run its installed native binary under the minimal environment above. After a held synthetic HTTP request establishes readiness, send SIGINT to its owned foreground process group. Require exit 4, the full cancelled JSON report, a closed request socket and no surviving owned processes **before cleanup**. No genuine provider call is needed.

For npm, drive every additional supported cell below with the same held-request readiness and pre-cleanup settlement checks.

| Invocation and signal destination | Required result |
| --- | --- |
| Installed bin, SIGINT to whole foreground group | exit 4, full cancelled JSON, no owned survivors |
| Installed bin PID alone, SIGINT | exit 4, full cancelled JSON, no owned survivors |
| Installed bin PID alone, SIGTERM | exit 4, full cancelled JSON, no owned survivors |
| Installed bin PID alone, SIGHUP | native ends; launcher ends **by SIGHUP**, no owned survivors; no exit-4/cancelled-JSON requirement |
| `exec sunstack lint` npm script, SIGINT to whole group | exit 4, full cancelled JSON, no owned survivors |
| `exec sunstack lint` npm script, SIGINT to npm PID alone | exit 4, full cancelled JSON, no owned survivors |
| `exec sunstack lint` npm script, SIGTERM to npm PID alone | exit 4, full cancelled JSON, no owned survivors |

Capture root/launcher/native PIDs and group, admitted request, signal destination, command/environment, actual exit **and signal**, full report where promised, socket closure and owned survivors **before cleanup**. A sleep/process list is not readiness; acknowledgement is not settlement. A cleanup kill never turns a survivor into a pass. Afterwards stop only remaining owned processes and retain both observations.

Record these **known limits** apart from supported cells: Linux plain `"sunstack lint"` script with SIGTERM to npm alone; SIGHUP to npm alone through the `exec` script; and npx. Retain bounded limit observations or applicable prior evidence with environment/cleanup, without applying the supported contract to those routes. **npx is unsupported; do not execute it as a promised route.** Preserve old npx failures at their original contract.

Fresh macOS/Linux jobs retain evidence after teardown. x64 macOS executes under Rosetta on fresh Apple silicon; arm64 also runs natively on the local Mac with a new empty HOME. Join S2's exact-archive Linux/linkage proof when applicable, never a different dev build. A local equivalent cannot close a fresh-hosted-job claim.

## Return and clean up

Return source SHA, adopted oracle/package contract, both tags/private-copy commits, release run IDs, **every job conclusion** including skipped `publish-npm`, each CI-pushed tap commit/formula hash, original/downloaded/installed hashes, URL-only diffs, mirror requests, report/exit comparisons, per-signal process/report/socket evidence and equality-test applicability. Mark each cell observed/reused/failed/limit/unknown with its reason; missing/empty cells remain unknown. Unsupported limits are never passing support claims.

Stop only owned mirror/container/process IDs after consumers settle; verify no owned survivors and reopen evidence. Leave private repos/tags/releases intact unless cleanup is separately authorized. Public URLs/publishing, Intel Mac hardware, other distributions, Windows and private CAs remain unverified. Return supported/unsupported/unknown per commissioned U row; the Lead owns acceptance.
