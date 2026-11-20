---
name: ste-writing
description: Use when you write or change a document, a ticket, an agent file or a skill file of this repo. It gives the ASD-STE100 Simplified Technical English rules that matter in practice, with examples and a checklist. It is a short guide and not the full standard.
---

# Writing in Simplified Technical English

`CLAUDE.md` makes ASD-STE100 the document standard for new and changed documents, to about 85%:
most sentences follow the rules; a sentence can break a rule where the STE form is less clear.
Frozen documents and review records keep their text. This skill is a short guide. It is
not the full standard.

## Rules that matter
1. Write short sentences. Procedure: 20 words or fewer. Description: 25 words or fewer.
2. One instruction in one sentence. Put the steps in a numbered list.
3. Use the active voice. Use the present tense for facts. Use the imperative for steps.
4. Use one word for one thing. Do not change the word to avoid repetition.
5. Use simple verbs. Write "use", not "utilize". Write "get", not "obtain".
6. Use articles ("the", "a"). Do not drop them.
7. Do not stack more than three nouns. Break the group with "of" or "for".
8. Do not use idioms, jokes, or phrasal words with two meanings.
9. Say what a number is: unit, date, source.
10. Put the warning before the step that it is about.

## Examples
| Bad | Good |
|---|---|
| The free cash flow got double-counted due to the interest income being retained in the margin. | The margin has the interest income. The bridge subtracts the cash. So the model counts the interest income two times. |
| Utilize the seed to ensure reproducibility. | Use the same seed. The result is then the same. |
| IBM P50 came out around 150. | `fa dcf drivers IBM` (seed 7) gives a P50 of $151.84. |
| Data was refreshed. | `fa sync IBM` updated the price data on 2026-11-20. |
| Tests were run and look fine. | `cargo test` ran: 568 passed, 0 failed. |

## Words of this repo
Keep the same name for the same thing: "standard variable", "driver model", "cache",
"data set", "P50", "market price", "model price", "ticket", "pull request".
A code name stays in backticks. A command stays in a code block.

## Checklist
- Each sentence has one meaning and is short enough.
- No passive voice in a step. No idiom. No dropped article.
- Each number has a unit and a source.
- Each command and path exists.
- The document does not state status that the board owns.
