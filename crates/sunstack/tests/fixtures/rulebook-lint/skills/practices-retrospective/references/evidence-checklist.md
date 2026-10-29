# Evidence Checklist

You arrive from move 1 with the boundary and evidence that is a summary, a partial transcript, or spread across sessions. This checklist decides what is worth reading; move 1's research step collects what lies outside the conversation. Return the records to read first, the coverage they leave and whether a recurrence claim is possible.

## Read actions before accounts

Read in this order:

1. The owner's own messages in the work: the intent, each correction and when it came. A user turn can carry pasted agent text, or another agent's message delivered as user input. Treat only the owner's own words as owner intent.
2. The host's record of actions and results: tool calls, commands and their output, and the files they changed. An agent's reply about what it did is its account, not the record.
3. What the work left behind: diffs and commits, review reports, proof and CI output.
4. The source of whatever governed the behavior, at the revision the work used where the record shows it: the skill text and which revision was offered or loaded, the hook and CI config, the driver.

Read summaries, traces, handoffs and memory notes last, to find the records behind them. Until you have read the record behind one, what it says stays its claim, a "done" in a trace included.

## Name what the evidence can't carry

- **Only a summary.** Report what it claims and that no record behind it was read, and ask your caller for the bounded records that would decide the question: name the session, the run or the review. Infer no cause and no recurrence from the summary alone.
- **A partial transcript.** A compacted or truncated span, elided tool output, or a helper whose session you don't have: name the missing span and the questions it leaves open. When the gap covers the decision, you can say what happened but not whether the guidance was read.
- **An unknown revision.** When you can't show which revision of a skill or check the work ran, a finding about that text is a finding about today's text, and the account says so.

Each limit goes into the account as a coverage gap, beside the findings it limits.

## Count recurrence across sessions

Apply move 2's recurrence rule session by session:

- Count the events whose records you read, not the events a summary reports.
- Two events share a mechanism when the same condition from move 3 held in each, such as the same skill never triggering or the same check never running. A shared symptom isn't enough: two overnight stalls, one on an approval nobody could see and one on a missing credential, are two findings of one event each.
- State the count with what you read: "the same check went unrun in three of the four sessions read; the fourth's transcript ends before the commit".

## When to stop

Stop when each record on the list is read or listed as a gap with its reason, each finding carries the coverage limits that bear on it, and each recurrence claim carries its count of events read with the same mechanism, or is dropped.
