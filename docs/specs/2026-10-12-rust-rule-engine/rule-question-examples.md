# RuleSet JSON and reusable JSONL rules

A **RuleSet** is the named definition selected by a caller. It refers to **rule files**, which store one reusable **rule definition** per JSONL line. Each rule contains named noul/choice/score **questions**. [Schema inventory](schema-inventory.md) explains every field and its domain owner.

![RuleSet, JSONL questions, captured evidence and validated native answers](assets/rule-question-flow.png)

[Diagram source](assets/rule-question-flow.mmd)

## The RuleSet file

The complete [skill-quality.ruleset.json](examples/skill-quality.ruleset.json) is:

```json
{
  "schema_version": 1,
  "name": "skill-quality",
  "description": "Check skill activation, output specificity and verification requirements.",
  "rule_paths": [
    "skill-rules.jsonl"
  ],
  "instruction_prefix": null
}
```

`name` and `description` identify its purpose to the person/agent selecting it. They do not cause execution or enter the model prompt. `rule_paths` resolves relative to this file. `instruction_prefix: null` means every question supplies its own instructions. A string, object or array can supply common instructions when reuse is useful.

The caller explicitly runs this RuleSet against chosen input. There is no event message, watcher, trigger subscription or engine-maintained scheduling history. The RuleSet groups the selected definitions and returns their native named answers together, as specified in [Specification C6](specification.md#c6--native-judgments-and-result-composition). It does not invent an overall verdict or require a separate aggregation setting.

## A rule line and its questions

The complete [skill-rules.jsonl](examples/skill-rules.jsonl) contains three lines and four questions. This is its first line, expanded for reading:

```json
{
  "rule_id": "skill.activation",
  "target_id": "U001",
  "applicability": {
    "kind": "always"
  },
  "context_selectors": [],
  "questions": {
    "activation": {
      "type": "choice",
      "instructions": "Does this paragraph clearly state when to use the skill?",
      "criteria": {
        "pass": {
          "description": "A concrete activating situation is clear.",
          "examples": [
            "Use when creating or updating a skill."
          ]
        },
        "fail": "No activating situation is stated.",
        "review": "An activating situation is suggested but ambiguous."
      }
    },
    "task_kind": {
      "type": "choice",
      "instructions": "Which kind of task does this paragraph describe?",
      "criteria": {
        "creation": "Creating a skill.",
        "updating": "Updating a skill.",
        "both": "Both creating and updating a skill.",
        "unclear": "No concrete task can be identified."
      }
    }
  }
}
```

The physical JSONL file has one complete object per line, with no array, header or wrapper. `rule_id` identifies the check. `target_id` locates the subject paragraph in the bound skill. `questions` holds independent model questions; the first asks for a pass/fail/review classification, while the second classifies task kind. They are not different schema representations of one verdict. The remaining lines demonstrate score and noul questions.

The skill binding associates this RuleSet with the complete teaching subject [SKILL.md](examples/SKILL.md). [skill-binding.json](examples/skill-binding.json) uses repository-relative skill paths and invocation-cwd-relative RuleSet paths; no subject text is repeated in the rule file. The source paragraph and declared context are captured into model `state` at runtime.

## What reaches Jev

For a null prefix, `question.instructions` and `question.criteria` are preserved exactly. With a prefix, instructions become `{ "prefix": rule_set.instruction_prefix, "question": question.instructions }`. Both values remain structured; there is no concatenation, merge, override or interpreted path. The two outer labels are the request builder's explicit structure, not extra Jev API fields. Question names correlate responses, while instructions/criteria carry their meaning. [TypeSafe Choice](https://docs.typesafe.ai/primitives/choice)

A model request contains multiple named questions and shared `state`. Runtime model selection and credentials are separate. Applicability and context selectors run locally; they are not model questions. Rule IDs, subject IDs and source spans provide correlation/provenance. Name and description are metadata, not automatic prompt text.

## Structured question content

Instructions and all option/level/noul descriptions accept string, object, array or null. Objects and arrays preserve recursive JSON, including nested finite numbers, booleans and null. Root numbers and booleans are outside the documented EntryType. Descriptive keys inside JSON content are author-chosen labels, not instructions to the engine. [TypeSafe advanced structure](https://docs.typesafe.ai/primitives/advanced)

Noul criteria may be omitted, or contain both `true` and `false`; either description may be null. Choice criteria is a nonempty label map. Score criteria is a nonempty ordered rubric. Their tagged question variants enforce these different shapes; no unrelated optional-field bag is used.

## Native results and useful failures

Choice returns the selected label plus confidence/distribution, so criteria can directly define the desired labels. Noul retains a yes probability. Score retains its fractional rubric position and distribution. A score is not a probability or a percentage of text that is correct. The [answer schema](schemas/decision-answer.schema.json) is owned once by decisions; rule results reference it through correlated execution outcomes.

| Case | Observable result |
| --- | --- |
| The activation question returns admitted `review` | That native label and its distribution remain available with rule/subject attribution |
| A consumer wants a thresholded noul result | The consumer deliberately defines that interpretation; no hidden threshold is supplied by transport |
| U001 is missing, duplicated or deleted | Source diagnostic; no judgment |
| Required context or the complete request cannot fit | Evidence/budget diagnostic; no silent truncation or pass |
| Provider returns an unknown label or wrong question kind | Invalid response; no partial completed answer map |
| Provider fails or cancellation is observed | Typed execution/cancellation outcome, never a pass |

These are teaching examples, not observed model answers, independent evaluation expectations or runtime proof. The schema files and example documents can be validated locally. The Rust engine, source resolver, HTTP adapter, hosted model and production consumers are unimplemented and unverified.
