# Descriptions and Invocation

## Standards

- A description's job is to help the agent decide whether to load the skill.
- A description starts with `Use when`, `Trigger when`, `Always use when` or `Always trigger when`.<!-- K001 -->
- A description uses an `Always` opener only when skipping the skill would break the work whenever its condition holds.<!-- K006 -->
- A description names the situation, symptom, user request, repository event or failure that should load the skill.<!-- K004 -->
- A description uses the searchable words the user, codebase, docs or logs actually use.
- A description gives a short why only when it sharpens routing.
- A description names an adjacent skill's boundary only when that skill is easy to confuse with this one.
- A description carries no workflow steps, tool lists, implementation detail or generic virtue.<!-- K005 -->
- A description aims for 100 to 200 characters.
- A description of more than 225 characters warns.<!-- K002 -->
- A description of more than 300 characters fails.<!-- K003 -->
- A description loads on its true prompt and stays quiet on the near miss; one that loads for everything is not a trigger.
- A changed description is tried on one prompt that should load its skill and one near miss that shouldn't, each in a fresh session, before it ships.
- A description routes on a positive loading condition; a boundary alone is not a trigger.
- A description never carries a rule marker or any other token.
- A description matches the skill's invocation capabilities.
- A skill is model-invocable when the agent must find it unprompted, or when another skill must reach it.
- A principle, which other skills only cite, and a domain skill, which they load, stay hidden from model invocation.
- When user-invocable skills outgrow what a person can remember, a router skill indexes them.

## Facts

- A skill can be model-invocable, user-invocable, or both. A description adds the agent's reach without removing the person's.
- A model-invocable skill keeps a description the agent reads to load it on its own. It pays context load, because the description sits in the window every turn.
- A user-invocable skill is named directly by a person. It pays cognitive load, because the person is the index that has to remember the skill exists.
- A skill's `name` is at most 64 characters of lowercase letters, digits and hyphens, and matches its folder name exactly.
- A skill's `description` is at most 1,024 characters.

## Decide the capability first

Settle who must reach the skill before writing a word of its description, and state that decision in portable terms: the agent unprompted, another skill's call, a person who names it, or several of these. The description is then written for that decision, and judged against it.

Questions that settle it, each answered by a standard above:

- Will the agent meet the situation without anyone naming the skill?
- Does another skill invoke it, or does another skill read it by path, as it reads a principle or a domain skill?
- Is it reached only by a person who knows it exists, and can that person still remember it among the skills they already hold?

IF the decision needs a client-specific encoding, load `platform.md` and return the encoding for each host the plugin supports.

## Choose the opener

`Use when` and `Trigger when` promise the same thing: what follows is the moment that should load the skill. Take the one that reads naturally with that moment, such as `Use when writing a migration` for an activity, or `Trigger when a migration file changes` for an event.

`Always` promises more: whenever the condition holds, the agent loads the skill without weighing whether this run needs it. Breakage earns that promise, and nothing else does. Ask what happens to the work, every time the condition holds, when the skill is skipped.

- Earned: a Rust crate's skill for `unsafe` code, "Always use when writing or changing `unsafe` blocks". Skipping it lets unchecked invariants through every time, because the work can't be done correctly without the checks the skill carries.
- Not earned: a release-notes skill. Skipping it makes the notes plainer, not wrong, so it opens with `Use when`.
- The near miss: a skill that is important, or used often, or that breaks the work in some runs and not others ("Always use when editing CI config", where most edits are safe). None of these is breakage every time. Narrow the condition to the runs that break, or open with `Use when`.

The tempting reason is "`Always` makes sure it loads". An `Always` the agent learns it can skip once teaches it to weigh every other `Always` too, so an unearned one weakens the earned ones.

## Keep it short

Aim for 100 to 200 characters. The description sits in the agent's context every turn, beside every other skill's, and the agent reads it to make one decision. Past about 200 characters the extra words are almost always workflow, implementation detail or a stack of boundaries, which the standards above keep out anyway. Over 225 characters warns. Over 300 fails, and the step that checks the description fixes it before that step completes. The platform's 1,024-character maximum is a ceiling, never a target.

Shorten in this order: move workflow steps and implementation detail into `SKILL.md`; drop every boundary except the neighbour that is actually confused with this skill; then tighten the moment's own words. The loading condition is the one part never cut to save characters.

```text
before (325 characters): Use when writing or reviewing a database migration, especially
  schema changes on live tables, backfills, or rollback paths. Reads the
  migration, checks it against the project's migration checklist, runs a
  dry run, and reports each risk with its fix. Not for ORM model edits with
  no migration file, seed data, or query tuning.
after (171 characters):  Use when writing or reviewing a database migration,
  especially schema changes on live tables, backfills, or rollback paths.
  Not for ORM model edits with no migration file.
```

The cut sentence was the workflow, and two of the three boundaries named neighbours nobody confuses with a migration skill. The loading condition and its symptoms are untouched.

## Test it on two prompts

Before judging a description, write two prompts in the words a real user would type: one that should load the skill, and a near miss that sits next door and should not. Read the description against both.

- It loads on the near miss: the trigger is over-broad.
- It misses the true prompt: the trigger is under-triggering.

Reading them is the first test, and it is enough while you are drafting. Before a changed description ships, run them: each prompt in a fresh session that sees the skill's description beside its neighbours' and nothing written about the change, so the session can't route from the design. Take which skill loaded from the host's own record of the load, never from the session's account of itself. The true prompt loads the skill and the near miss doesn't; a session that misses the true prompt, or loads on the near miss, is a failure to fix before the description ships.

For a database-migration skill, "write a migration that adds a non-null column to orders" should load it, and "rename this field on the ORM model, no migration needed" shouldn't. A skill hidden from model invocation, such as a domain skill other skills load by path, has no implicit trigger to try, so its trial records `not applicable`.

The same reading answers "why did this skill load?" after the fact. Put the prompt that loaded it beside the description and name the words that matched, or the words that were missing when it failed to load. The repair follows from those words.

## The description pattern

```text
Use when <observable trigger>, especially <specific symptoms or decisions>,
so the agent <succinct payoff>. Not for <adjacent boundary>.
```

Any of the four openers can start it, `Always` only where it is earned. The payoff and the boundary clauses are optional. A short description with a clear trigger beats a complete sentence that summarizes the workflow.

## Bad shapes

- Workflow summary: "Reads files, checks X, updates Y, and reports Z."
- Generic virtue: "Helps write better code."
- Tool list: "Uses grep, tests, and GitHub."
- Ownerless umbrella: "Use for releases" when several release skills exist.
- Negative-only routing: "Do not use when..." without a strong positive load condition.
- Unearned `Always`: "Always use when writing docs."

## Repair moves

- Over-broad trigger: add concrete symptoms, or a boundary with the neighbour it is confused with.
- Under-triggering: add the user's words and the repository or code signals that mark the situation.
- Workflow summary: move the process detail into `SKILL.md`.
- Over length: shorten in the order under Keep it short.
- Unearned `Always`: open with `Use when`, or narrow the condition to the runs that break without the skill.
- Hidden payoff: add a short "so that..." phrase.
- Wrong invocation: IF a run showed the skill loading in the wrong place or not loading, load `steering.md` and return its repair for wrong invocation.

One repair, worked:

```text
before: Reviews database migration scripts for common problems and
        suggests fixes using the project's migration checklist.
after:  Use when writing or reviewing a database migration,
        especially schema changes on live tables, backfills, or
        rollback paths. Not for ORM model edits with no migration
        file.
```

The before leaves the loading condition and the near-miss boundary implicit. The after makes both explicit without summarizing the workflow.

## When to stop

Complete when the description opens with one of the four openers, with `Always` only where skipping the skill breaks the work; names a real loading condition; is 300 characters or fewer, aiming for 100 to 200, with any length over 225 either shortened or its warning explained; survives both the true prompt and the near miss; and states a boundary with any confusable neighbour. A changed description of a model-invocable skill also has its two-prompt trial result, and a hidden skill's trial reads `not applicable`. Return each standard above that applies as kept or broken, with the words that show it.
