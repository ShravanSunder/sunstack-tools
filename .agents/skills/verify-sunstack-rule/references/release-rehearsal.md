# S4 — install the exact private release

Observe two real generated releases through Homebrew, shell and npm. Expected outcome: runner R7/R16/R19–R23 and VD U2/U8/U10/U11/S4 at SKILL.md's oracle. The release workflow is product-owned; this recipe observes its outputs and owns only verification fixtures/mirror/inspection. Run only against separately commissioned releases. An artifact preflight is not installation proof.

Use only the authorized private repositories `ShravanSunder/sunstack-release-rehearsal` and `ShravanSunder/homebrew-sunstack-rehearsal`. Check `gh repo view <repository> --json visibility,nameWithOwner` for each before any later remote mutation. Stop if either is public or differs from the declared destination. This preparation commission does not create repositories, push, tag, publish or spend on hosted work. A later rehearsal commission must name those effects. Never publish to the real tap or npm registry.

## Declare and download both releases

The Implementer supplies two tags (`v0.0.1-rehearsal.1` and `v0.0.1-rehearsal.2` in the plan), exact source/workflow revisions, successful workflow IDs and release metadata. The older and newer workspace/package versions must equal their tags. Run the real generated workflow twice; a hand-built archive does not satisfy this seam. Watch each declared run with `gh run watch <id> --repo ShravanSunder/sunstack-release-rehearsal --exit-status --interval 240`, retain its raw jobs/artifact evidence, and distinguish workflow completion from all-channel proof.

Download each tag's files with authenticated `gh release download <tag> --repo ShravanSunder/sunstack-release-rehearsal --dir <new-tag-directory>`. The mirror must never receive a token or auth header. Require, per tag:

- `sunstack-<target>.tar.gz` and matching `.sha256` for macOS arm64/x64 and Linux musl arm64/x64;
- `sunstack-installer.sh`, `sunstack.rb`, `sunstack-npm-package.tar.gz` and `sunstack.lint.schema.json`.

From the monorepo root, use the main skill's clean uv environment:

```sh
uv run --no-config --no-env-file --no-python-downloads --default-index https://pypi.org/simple --script \
  .agents/skills/verify-sunstack-rule/scripts/release_assets.py \
  --older <older-download> --older-version 0.0.1-rehearsal.1 \
  --newer <newer-download> --newer-version 0.0.1-rehearsal.2 --evidence <new-evidence>
```

The helper checks every archive checksum, finds exactly one regular bounded `sunstack` member without extracting arbitrary archive paths, retains its exact bytes/hashes, checks static Linux linkage, and checks formula/npm/installer versions and formula checksums. Both versions must have different binary bytes, so an older/newer comparison can fail. `s4-artifacts.json` explicitly leaves installations unobserved. Missing files or a checksum mismatch stops before installs; never rewrite a checksum to match.

## Mirror without replacing bytes

Serve only the authenticated download directories from a loopback HTTP server whose PID/port you record, with one immutable path per version and both versions simultaneously available. Keep archive bytes unchanged. Verify every served archive by SHA256 against the preflight receipt. On GitHub runners the mirror lives on that runner; a container uses its own authorized connection to that local mirror. Do not expose it publicly.

Adapt only download locations in a retained copy of the generated formula/npm package:

- Formula: change only `url`/`mirror` bases to that tag's loopback directory. Keep `version`, target names and every real checksum unchanged. Retain original, adapted copy and diff. Current Homebrew requires formulae to belong to a tap and requires explicit trust for non-official formulae. On the disposable fresh runner only, create a local tap with `brew tap-new --no-git verifier/<owned-tap>`, locate it with `brew --repo verifier/<owned-tap>`, and copy the adapted bytes unchanged to its `Formula/sunstack.rb`. Set `XDG_CONFIG_HOME` to an owned empty directory for every Homebrew command, run `brew trust --formula verifier/<owned-tap>/sunstack`, then `brew install --formula <tap-path>/Formula/sunstack.rb`. This grants trust to that formula only; checksum validation remains required. Keep the installed version/hash/report/signal proof and uninstall that owned formula before the next version. Do not replace an existing user installation or modify the local Mac's Homebrew state.
- Shell: use the **older tag's own** generated installer while the newer release is available. cargo-dist 0.32.0 embeds `APP_VERSION` in each installer; version pinning selects the older release URL/installer, rather than inventing a `VERSION` flag. `INSTALLER_DOWNLOAD_URL=<older-mirror-directory>` redirects the archive base. Read that generated installer's supported unmanaged/install-directory variable before setting it to an owned empty install directory; never modify the real home or shell profile. Keep the unmodified installer and archive request log, and compare installed bytes with the older receipt.
- Npm: unpack the generated npm artifact in scratch, change only `package.json`'s `artifactDownloadUrls` to the tag's mirror base, and `npm pack <adapted-package-directory> --pack-destination <scratch>`. Keep package name/version, platform map, launcher and postinstall code unchanged. Retain the diff, then `npm i -D <packed-tarball>` and `npx --no-install sunstack …` in an empty consumer directory. No registry publication or substitute binary is allowed. Install-time GitHub access/write-into-package/pnpm approval limits remain product README obligations.

This is a mirror stand-in: it preserves the downloaded release bytes while avoiding authentication requirements the generated installers cannot meet. It proves those channels against those bytes; public URLs remain unverified until the first public release. If generated artifacts lack a supported mirror seam, return that concrete gap to the Lead instead of adding installer behavior.

## Drive all installation cells

Run Homebrew on fresh GitHub-hosted macOS and Linux runners; shell installs on macOS and local Linux containers; npm in a clean digest-pinned Node container and the fresh runners. Retain Node image/platform/digest and tool versions. Package-manager setup may use runner tools; product execution must use absolute installed paths under `env -i PATH=/usr/bin:/bin HOME=<new-empty-home>`, with no `DYLD_*` variables. Do not add Homebrew/Node paths to the product's minimal environment. Npm's launcher needs Node; invoke its declared absolute Node/launcher paths while retaining the same minimal environment, then check the native installed binary independently under that environment too.

Each cell for each version records:

1. Installed binary hash equals the matching extracted release bytes and `--version` is exactly `sunstack <tag-without-v>`. Formula/package versions equal the same tag. A channel that installs newer bytes for the older request fails.
2. The S1 valid fixture's `lint --check` exits 0; its mechanical error lint exits 1 with the exact finding; a clean fixture exits 0. Run the same fixture through the native binary and npm launcher with `--format json`, parse both and compare every field, including config/version/findings/evidence/usage/exit. Compare on the same directory/state, not separately rooted copies whose paths differ.
3. Ctrl-C reaches running work and every owned child settles, including npm. Use the S1 controlled server to hold an admitted request before signaling the owned foreground process group; capture the launcher and native child PIDs, exit, complete cancelled JSON and socket settlement. Readiness comes from the held request, not a sleep or a process list. Require the prescribed exit 4 and absence of surviving children; signal acknowledgement alone is insufficient. Do not call the real provider for this control.
4. Fresh hosted macOS/Linux jobs retain all evidence above after teardown. x64 macOS executes under Rosetta on the fresh Apple silicon runner, and arm64 also runs natively on the local Mac with a new empty HOME. S2 owns Linux Alpine/Debian and linkage; join its exact-archive proof, never a different dev build.

Build the verification job in the private rehearsal checkout under the later execution grant; it downloads with its workflow token, stops using that token before serving the mirror, installs the exact assets, runs the commands above under the minimal environment and uploads only safe verification evidence to that private run. Do not copy any key or raw credential location into workflow YAML or logs. Require actual successful fresh macOS and Linux jobs; a local equivalent cannot close that claim. Reuse the generated product release workflow unchanged except declared private destination/version inputs; keep any verification job separate.

## Completion and cleanup

Keep two tag/source/workflow IDs, original asset hashes, mirror request logs, URL-only adaptation diffs, installed hashes/versions, actual command exits, native/npm JSON comparisons, cancellation/child evidence and fresh-runner logs. Report a matrix with each version × channel × platform observed/reused/unreachable and its reason; empty or missing cells remain unknown. An artifact-only receipt does not close S4.

Stop only owned mirror/container/process IDs after their consumers settle, leave the private repos/tags/releases intact unless cleanup is separately authorized, and reopen retained evidence. Return supported/unsupported/unknown per U row and all gaps. Public release addresses, public publishing, Intel Mac hardware, other distributions, Windows and private CAs stay unverified. The Lead owns acceptance.
