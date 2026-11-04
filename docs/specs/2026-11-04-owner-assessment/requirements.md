# Requirements: owner assessment

4 Nov 2026. Author: 🦁 Lead (`claude-local/affa24ce`). Status: draft, not reviewed. One concern, so no capability level.

Normalized from the accepted skill-rules design in ai-tools (`docs/specs/2026-11-03-skill-rules/` on `skills/rules-as-nudges` at `2a94295a`: Requirements U11, U13, U14; Specification R5, R13–R15; Program Design "The engine feature: owner assessment"), which was reviewed to zero open findings, and from the owner's words quoted there. This file states the engine's part in generic terms; the skill wording stays in that set.

## Who it serves

- **A caller's rule writer:** declares how checked files group into units, one question to ask about each unit, and which rules apply to which answers.
- **The agent reading a run:** sees each unit's answer, how sure it was, and which results it excluded or skipped, never a silent pass.
- **This repository's maintainers:** keep the engine generic (owner, 28 Oct, standing: "fix the engine and release it; never bend skills, runner or rules around an engine gap"; owner, 31 Oct: "our system was to be generic enokugh that any evidence and rules shoudl have been able to go in").

## Problem

A rule that should judge only some kinds of files judges all of them. In the skills repository on 1 Nov, a placement rule gave 9 wrong nudges of 11, four on product skills' own text; `rule.generalization` gave 45, all wrong. The engine at 0.6.0 can't ask one question per unit and give the answer to every rule: `select` is a yes/no question per piece, `as_kind` refuses selected kinds, and a reference file never reaches its unit's description.

## Needs

### U1: One answer per unit, asked once

`kind: requirement · status: authorized · source: skills set U13 ("A model reads each skill's description once and answers what the work is for")`

A caller declares a unit (a folder with one describing file and its members) and one choice question about the unit's description. Each run asks it once per unit and every rule that needs it reads that one answer, including rules over the unit's other files.

### U2: A rule judges only the answers it accepts

`kind: requirement · status: authorized · source: skills set U13 ("A product's own skill is never judged by a rule about general skills")`

A rule can declare which answers it applies to. On a unit with another answer it gives no judgment, and the report says why.

### U3: An unsure answer skips and is listed

`kind: requirement · status: authorized · source: skills set U14; owner, 3 Nov: "skip, and list it"; "this shokudlo be caught in evals. we can log it too"`

When the answer isn't sure enough, rules that need it skip the unit's files, and the run lists the skip with the answers. A failed question makes the run incomplete.

### U4: The engine stays generic

`kind: requirement · status: authorized · source: skills set U11, R5`

Units, questions, options and rule gates are declared by the caller. This change adds no skill-specific word, kind or path to the engine.

## Boundary

**In:** config sections for units and questions; a rule-entry gate; the assessment stage in a lint run; report and schema additions; admission refusals; tests and verification journeys for them; a release.

**Out:** caching answers across runs; evidence-listing mode (it exits before rules run, and asks no unit questions).

**Open, for the owner** (the accepted skills design at `2a94295a` includes both; narrowing either needs the owner's supersession):

- **A model per assessment.** The accepted contract lets each assessment name its decision model. This set keeps it until the owner says otherwise.
- **The existing skill-aware target resolver.** The accepted R5 says the engine release carries nothing skill-specific; the engine already has `Skill`/`PathOrSkill` and `SKILL.md` lookup in its target loader. Either this release makes that resolver caller-declared, or the owner accepts it as later work.
