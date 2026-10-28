# Descriptions and Invocation

## Standards

- A description's job is to help the agent decide whether to load the skill.
- A description starts with `Use when`, unless the platform requires another shape.<!-- K001 -->
- A description names the situation, symptom, user request, repository event or failure that should load the skill.
- A description uses the searchable words the user, codebase, docs or logs actually use.
- A description gives a short why only when it sharpens routing.
- A description names an adjacent skill's boundary only when that skill is easy to confuse with this one.
- A description stays compact, because it is in the context every turn.
- A description carries no workflow steps, tool lists, implementation detail or generic virtue.
- A description loads on its true prompt and stays quiet on the near miss; one that loads for everything is not a trigger.
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
- A skill's `description` is at most 1,024 characters.<!-- K002 -->

## Decide the capability first

Settle who must reach the skill before writing a word of its description, and state that decision in portable terms: the agent unprompted, another skill's call, a person who names it, or several of these. The description is then written for that decision, and judged against it.

Questions that settle it, each answered by a standard above:

- Will the agent meet the situation without anyone naming the skill?
- Does another skill invoke it, or does another skill read it by path, as it reads a principle or a domain skill?
- Is it reached only by a person who knows it exists, and can that person still remember it among the skills they already hold?

IF the decision needs a client-specific encoding, load `platform.md` and return the encoding for each host the plugin supports.

## Test it on two prompts

Before judging a description, write two prompts in the words a real user would type: one that should load the skill, and a near miss that sits next door and should not. Read the description against both.

- It loads on the near miss: the trigger is over-broad.
- It misses the true prompt: the trigger is under-triggering.

The same reading answers "why did this skill load?" after the fact. Put the prompt that loaded it beside the description and name the words that matched, or the words that were missing when it failed to load. The repair follows from those words.

## The description pattern

```text
Use when <observable trigger>, especially <specific symptoms or decisions>,
so the agent <succinct payoff>. Not for <adjacent boundary>.
```

The payoff and the boundary clauses are optional. A short description with a clear trigger beats a complete sentence that summarizes the workflow.

## Bad shapes

- Workflow summary: "Reads files, checks X, updates Y, and reports Z."
- Generic virtue: "Helps write better code."
- Tool list: "Uses grep, tests, and GitHub."
- Ownerless umbrella: "Use for releases" when several release skills exist.
- Negative-only routing: "Do not use when..." without a strong positive load condition.

## Repair moves

- Over-broad trigger: add concrete symptoms, or a boundary with the neighbour it is confused with.
- Under-triggering: add the user's words and the repository or code signals that mark the situation.
- Workflow summary: move the process detail into `SKILL.md`.
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

Complete when the description names a real loading condition, survives both the true prompt and the near miss, and any confusable neighbour has a stated boundary. Return each standard above that applies as kept or broken, with the words that show it.
