# CLI delivery through package-manager caches

This journey is operable for its staged HTTP and complete local-binding routes when the declared target and required permissions are available. An actual hosted release is a separate observation.

The adopted cache-delivery plan requires the exact current guide command to select the matching CLI while an older mutable-URL cache is retained. The consumer copies the published guide URL; a direct Node invocation, a successful exit alone or the presence of a matching cache directory does not prove that the package manager selected it.

Before running, freeze the source, stage or actual published target, guide and archive identity, manifest, input and package-manager versions and cache state. Obtain actual permissions for the owned evidence/copies, package-manager-managed cache writes, process and network. Preserve old cache state; do not purge, reset or force refresh it. A first invocation is cold only when absence for that exact URL has been demonstrated; call a later invocation warm only when actual reuse has been demonstrated.

From the monorepo root, the read-only staging preflight is:

```sh
node apps/demistify-poc/scripts/verify-cli-delivery.ts --stage <release-directory> --major <number>
```

It reports the staged guide examples and checks advertised archive bytes against the staged manifest. It deliberately exits 2 after successful preflight because it does not run the package-manager journey. It cannot establish that an advertised URL resolves over HTTP, that the selected package matches the archive, or that an actual release is published.

The driver exposes `--run <guide-or-loopback-origin> --input <collection>` after the preflight arguments; use it only after a matching target and permissions are admitted. Its identity companions inject a recorded Node preload and are separate from ordinary commands. The first-cache classification stays unknown unless independently established. The complete journey must use the exact generated guide's pnpm prepare/check with only the authorized loopback origin substitution for a staged target, then repeat it and observe actual cache reuse and selected executable/package identity against independently extracted archive bytes. Also exercise the listed npx alternative and documented local-binding archive on owned copies, and read back prepared files. Preserve commands, raw output, exit codes, identities, before/after cache state and failure evidence. On a later separately authorized release, use the literal current published guide URL and check actual hosted bytes and manifest; local staged evidence is not hosted proof.

For each unresolved identity, permission or target gap, report unknown rather than deriving a success from output alone. Keep product repairs with the implementer. Keep evidence and owned child-process cleanup separate from the producer's server and shared caches.
