# Generalization

## Standards

- A rule a skill teaches states a reusable judgment, with any product, library, API or incident only as an example, unless the rule names the scope the owner adopted it for.<!-- K001 -->
- A skill encodes judgment and house style, never task history.
- A rule passes the transfer test both ways: it holds on a materially different repository where it should apply, and it rejects a bad choice that uses something the rule never names.

## Why rules overfit

A rule usually arrives with an incident: one session, in one repository, on one stack, went wrong. The quickest fix writes the incident's nouns into the rule, because they feel concrete and checkable. The rule then encodes one repository's answer instead of the judgment that produced it, and it misleads every repository whose answer differs.

## Running the transfer test

1. Write the rule as the judgment, in words that name no product, library, API or incident. Where a named thing stays, label it: an example, or the owner's adopted scope.
2. Case one, where it should hold. Pick a repository that differs in what matters (another language, database, framework or team convention) where the judgment should still apply. Read the rule there: does it still tell the agent the right thing?
3. Case two, a bad choice it never names. Build a bad choice from something the rule doesn't mention. Read the rule again: does it still reject that choice, for the right reason?
4. Give the verdict. Passing both cases keeps the rule. Failing case one means it is overfitted: rewrite it as the judgment, or narrow it to the scope the owner adopted. Failing case two means it lists instances instead of stating the judgment: find the judgment the instances share.

The two cases below are worked examples, not rules; neither storage choice is a standard of this topic.

## Worked case: storage rules that fail

A closed pull request in this plugin's repository proposed persistence rules for two language standards skills: TypeScript persistence through SQLite with Drizzle and checked-in migrations, and Rust persistence through SQLite with SQLx. Both were written as rules for every repository.

- Case one: a TypeScript service already built on Postgres, with its own migration tool. The rule tells the agent to bring in SQLite and Drizzle, so it rejects a valid repository. It fails.
- Case two: an agent adds a second database to a repository that uses one. The rule rejects it, but only because the addition isn't SQLite; in a SQLite repository it would wave through a second SQLite store nobody asked for.
- Verdict: fails. The incident's stack became the rule, with no scope the owner adopted.

## Worked case: the owner's rule that passes

The owner named the reusable rule that change should have taught: "use teh repo techonolgoy or ask the suker dont assume tech choices", meaning use the repository's own technology, or ask the user, and don't assume technology choices.

- Case one: the Postgres service. The rule says to use Postgres and its migration tool, which is right. In a SQLite repository it says SQLite, so the original incident survives as one example of the rule.
- Case two: an agent adds a Redis cache, or a new ORM, to a repository that uses neither, without asking. The rule never names Redis or ORMs, and it rejects both.
- Verdict: passes both.

## A scope the owner adopted

What an adopted scope looks like in a rule: "A principle or domain skill carries both hide settings, by the owner's decision" names its scope (principle and domain skills) and its basis (the owner's decision); it claims nothing for other skills. Without the scope and the basis, the same words would read as a universal rule.

## Task history

Task history is what happened on one run, kept as if it were judgment. Bad, an illustration built on the worst case named by the pull request in the storage case above: "Every commission cites `packages/oauth-broker` as the persistence example." In a repository without that package, the commission points at nothing. Good, the form that pull request's own description used: "A commission that touches persistence names the repository's own example of the pattern, or says none." The incident stays in the change's evidence, where it explains why the rule exists.

## When to stop

Complete when each rule the skill states has its judgment sentence, its two transfer cases and a verdict, and each failing rule is rewritten as the judgment or given the scope the owner adopted, then tested again. Return each rule with its two cases and its verdict.
