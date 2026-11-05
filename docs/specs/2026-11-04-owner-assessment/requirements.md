# Requirements: filters

4 Nov 2026. Author: 🦁 Lead (`claude-local/affa24ce`). Status: draft. The earlier owner-assessment draft was reviewed to zero open findings at `9116442`; this revision moves it to the owner's final config vocabulary (4 Nov) and adds U5. One concern, so no capability level.

Normalized from the accepted skill-rules design in ai-tools (`docs/specs/2026-11-03-skill-rules/` on `skills/rules-as-nudges`: Requirements U11, U13, U14; Specification R5, R13–R15; Program Design "The engine feature: filters"), and from the owner's words quoted there. This file states the engine's part in generic terms; the skill wording stays in that set.

**Vocabulary, settled by the owner on 4 Nov.** A **filter** groups checked files by folder (`per`), reads one piece of one file in that folder (`reads`), and asks one choice question about it (`question`); the question's criteria labels are the filter's **answers**. A rule names the answers it applies to with `only_when`. Owner: "filter+folters sure"; "we keep jevs flexibliyt for inptu filter mehcina, , output and evencie. and also deicion tree?"; "yes stay on json ok inline or yaml referenc eok?". The earlier names (owners, units, assessments, `requires`, `accept`) are retired.

## Who it serves

- **A caller's rule writer:** declares a filter (which folders, which piece to read, one question) and which rules apply to which answers.
- **The agent reading a run:** sees each folder's answer, how sure it was, and which results it excluded or skipped, never a silent pass.
- **This repository's maintainers:** keep the engine generic (owner, 28 Oct, standing: "fix the engine and release it; never bend skills, runner or rules around an engine gap"; owner, 31 Oct: "our system was to be generic enokugh that any evidence and rules shoudl have been able to go in").

## Problem

A rule that should judge only some kinds of files judges all of them. In the skills repository on 1 Nov, a placement rule gave 9 wrong nudges of 11, four on product skills' own text; `rule.generalization` gave 45, all wrong. The engine at 0.6.0 can't ask one question per folder and give the answer to every rule: `select` is a yes/no question per piece, `as_kind` refuses selected kinds, and a reference file never reaches its folder's description.

## Needs

### U1: One answer per folder, asked once

`kind: requirement · status: authorized · source: skills set U13 ("A model reads each skill's description once and answers what the work is for")`

A caller declares a filter: a folder pattern, the one piece it reads in that folder, and one choice question about that piece. Each run asks the question once per folder, and every rule that needs it reads that one answer, including rules over the folder's other files.

### U2: A rule judges only the answers it names

`kind: requirement · status: authorized · source: skills set U13 ("A product's own skill is never judged by a rule about general skills")`

A rule can name which answers it applies to (`only_when`). On a folder with another answer it gives no judgment, and the report says why.

### U3: An unsure answer skips and is listed

`kind: requirement · status: authorized · source: skills set U14; owner, 3 Nov: "skip, and list it"; "this shokudlo be caught in evals. we can log it too"`

When the answer isn't sure enough, rules that need it skip the folder's files, and the run lists the skip with the answers. A failed question makes the run incomplete.

### U4: The engine stays generic

`kind: requirement · status: authorized · source: skills set U11, R5; owner, 4 Nov: "skill kind isnt that over fittkign to skills? is this only for skill rules or the general litner?"`

Filters, their folders, questions, answers and rule gates are declared by the caller. This change adds no skill-specific word, kind or path to the engine; the same filter can group code packages.

### U5: A one-off match can be written inline

`kind: requirement · status: authorized · source: owner, 4 Nov: "lastly shodu ast grep be seperate yaml or if one off inline?"; "we shokud provide both options in ast grep no?"; "yes stay on json ok inline or yaml referenc eok?"`

An evidence kind's ast-grep rule can be a named rule in the evidence project, as today, or a one-off rule written inline in the config as a one-line YAML string. The config stays JSON.

## Boundary

**In:** a `filters` config section; an `only_when` rule-entry gate; the filter stage in a lint run; an inline ast-grep rule on an evidence kind; report and schema additions; admission refusals; tests and verification journeys for them; a release.

**Out:** caching answers across runs; evidence-listing mode (it exits before rules run, and asks no filter questions); inline rules inside field sources (they keep naming a rule or a kind); a config format other than JSON.

**Settled by the owner** (each supersedes part of the accepted skills design at `2a94295a`):

- **One model for the run.** Filters use the run's `decision_model`; no per-filter model. Authority: owner, 4 Nov (review 00facfd0 of the skills design page), decision 8, "One model for the run".
- **The existing skill-aware target resolver is later work.** This release adds nothing skill-specific; the existing `Skill`/`PathOrSkill` resolver and its `SKILL.md` lookup stay until a later change, logged in ai-tools `docs/wip/skills-authoring/2026-11-03-skill-rules-task-log/out-of-scope.md`. Authority: owner, 4 Nov (review 00facfd0 of the skills design page), decision 9, "Later work, logged".
