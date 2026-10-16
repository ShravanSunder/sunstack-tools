# Refused acquisitions consume their read budget

Documents and transcripts account for attempted content reads before semantic validation returns. A UTF-8 or adapter-output refusal consumes the bytes already read; a document also consumes its repository-file slot. A metadata-only refusal consumes no content budget. Public APIs and wire shapes are unchanged.

The four permanent public `EvidenceSession` regressions in `acquisition_limits.rs` use real files and the existing trusted transcript adapter boundary. Against parent `6d4a87acd9dbb15d9c6a26e3dbcde526ba9ffd93`, the three consumed-budget scenarios failed and the metadata-only control passed (exit 101). After the repair, the isolated milestone 1 source snapshot passed all 45 evidence tests and all 45 nextest tests (exit 0). Scoped Clippy with `--all-targets -- -D warnings` and `cargo fmt -p sunstack-evidence -- --check` passed (exit 0).

Validation ran from an archive of the exact parent plus these changes under `/private/tmp/2026-10-s4a-m1-budget/snapshot`; no milestone 2 source or dependency was present. Logs are alongside that snapshot. C commands used command-local `CFLAGS="-isysroot $(xcrun --sdk macosx --show-sdk-path)"` for the installed SDK. This establishes native library behavior and scoped quality gates; hosted CI and the milestone 2 integration remain separate.
