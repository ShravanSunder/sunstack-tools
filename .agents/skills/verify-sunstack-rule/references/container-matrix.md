# S2 — standalone release artifacts

Observe the exact shipped binaries in four fresh Linux cells: Alpine and Debian, each x64 and arm64, using the immutable per-platform digests in `scripts/container_matrix.py`. Expected outcome: runner R16/R18 and VD U8/U9/S2 at SKILL.md's runner oracle. No Linux interpreter or shared dependency is allowed; macOS dependencies must be OS paths under `/usr/lib` or `/System`.

**Draft until the declared release artifacts are driven.** Preparation checks only the apparatus. Inputs: both Linux musl binaries, their declared source/version/SHA256s, optional macOS artifacts, Docker and the four already cached pinned images. The driver never builds or pulls. A missing target/image/daemon is a setup gap; keep its error and do not call it a failed portable build.

## Drive

Run from the monorepo root with the main skill's clean uv environment and a new authorized evidence directory:

```sh
uv run --no-config --no-env-file --no-python-downloads --default-index https://pypi.org/simple --script \
  .agents/skills/verify-sunstack-rule/scripts/container_matrix.py \
  --prepare --evidence <new-evidence>
```

This produces a mechanical-error fixture and four matrix pins, with `product_observed: false`. For the observation omit `--prepare`; add `--linux-amd64 <declared-binary> --linux-arm64 <declared-binary> --version <version-without-v>`. Optionally add `--macos-arm64 <artifact>` and `--macos-x64 <artifact>` for linkage inspection on macOS. First compare every driver-recorded digest with the commission's declared archive extraction. Copying or inspecting a different local build is not release proof.

The inspector reads ELF64 headers from the exact mounted bytes. It checks the architecture and rejects PT_INTERP or DT_NEEDED, including libgit2/OpenSSL. Every container has a read-only root, read-only binary/fixture mounts, network `none`, pull `never`, an empty tmpfs home, no capabilities and a minimal `env -i PATH=/usr/bin:/bin HOME=/home/runner`. Each cell runs version (exit 0 and exact version), `lint --check` (0), and a real mechanical-error lint (1 with the literal finding). No key or proxy enters the product environment. The 12 commands must all finish; an empty pass set cannot close the matrix.

The driver bounds each container to 120 seconds and removes only its exact owned name if it times out. Retain a cleanup failure as an open resource gap. Image metadata, exact commands, stdout/stderr/exits and binary digests remain in evidence after container teardown. A binary hash change stops the run.

## Complete U8/U9

S2's Linux pass does not close macOS runtime or source feature policy. Join the following declared-target evidence:

- `otool -L` on each shipped Mach-O lists only OS libraries, never Homebrew libgit2. The optional driver inputs retain that output.
- Fresh GitHub-hosted macOS runtime, x64 under Rosetta, and a native arm64 local Mac run execute version/check/lint under the same minimal environment. S4 owns fresh-runner installation and runtime; retain the local native arm64 observation as a distinct cell.
- `cargo tree --locked -e features -i libgit2-sys` from the declared product source shows `vendored-libgit2`; inspect all git2 manifests for `default-features = false` and absence of Git network features. Linkage alone cannot establish that Cargo feature claim.
- S3 completes the static Linux no-system-CA hosted judgment; this offline matrix cannot.

Wrong builds rejected: dynamically linked Linux, extra macOS dylib, wrong architecture/version, ambient helper dependency, config-check failure, or clean exit on the failing fixture. Other Linux distributions, Intel Mac hardware, private CAs and Windows remain unverified.

Return every matrix cell and command exit, observed binary/source identity, independent declared-hash comparison, linkage/Cargo-feature evidence and missing macOS/hosted cells as supported/unsupported/unknown. `s2-pass.json` covers only its Linux cells plus optional linkage, not the entire U8/U9 rows.
