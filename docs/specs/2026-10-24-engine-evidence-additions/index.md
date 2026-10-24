# Engine evidence additions

An addition to the rule engine's design set (`sunstack-tools/docs/specs/2026-10-12-rust-rule-engine/`). It lets mechanical rules run over session logs and code changes, makes Codex edits visible, and limits change rules to what a change added. Lead: 🦁 claude-local `68844339`. Status: proposed, for design review. Working home: this scratch folder. The set moves unchanged into sunstack-tools `docs/specs/` with the implementation PR.

- Requirements: [requirements.md](requirements.md) (U32–U35)
- Specification: [specification.md](specification.md) (R42–R45, V36–V40)
- Program Design: [program-design.md](program-design.md)
- Verification Design: [verification-design.md](verification-design.md)

## Reading path: why a Codex edit was invisible, and how it becomes visible

1. [Requirements: the need](requirements.md#the-need)
2. [Specification: R42](specification.md#obligations)
3. [Program Design: binding and owners](program-design.md#binding-and-owners)
4. [Verification Design: U33](verification-design.md)

## Reading path: a change rule that judges only what a commit added

1. [Requirements: U34](requirements.md#requirements)
2. [Specification: sides, R44 and R45](specification.md#entities)
3. [Program Design: sides and mechanical targets](program-design.md#binding-and-owners)
4. [Verification Design: U34](verification-design.md)
