# The memory of a head

A new session of a head starts with no memory of the last session. The files of your
folder give it a memory. The memory is files in git only.

The ideas come from the Anthropic post "The components of agent memory" (2026-11-06,
`https://www.anthropic.com/research/components-of-agent-memory`), read on 2026-11-20
through a fetch tool. Read the post before you rely on a detail of it.

## 1. What memory is for

Memory helps the next session of the same head do its job better. This page says what
memory is for. It does not say how you must use it: you decide what is important.

Save what exists only because you worked it out or the person said it: a plan, a
tested rule, a decision, a lesson from an error. Do not save a state that you can read
again from the repo, the board or a command. A copy of a state is old tomorrow, and a
wrong note misleads each later session.

In the post, the model that made 413 memory files got one badge of the game. The model
that made 9 files and one dashboard got four. Keep few files.

## 2. Each head has a different memory

| Head | What its memory is for | Examples |
|---|---|---|
| Quant Research | To keep what was learned about models and signals | A calibration method that failed and why; a numerical trap; a result that changes the plan of the model sequence |
| Fundamental Research | To keep the judgments about value that are not in the code | Why a convention was chosen or refused; a hypothesis about the gap to the market and its test; what a company type needs |
| Technology | To keep how the environment and the sources behave | A source that refuses cloud addresses; a build trap; what makes the CI cost high |
| Quality and Risk | To keep the patterns of the reviews | A defect that came back in several reviews; a check that found nothing in many reviews; a precedent for a verdict |

## 3. The layers

| Layer | What it is | Here |
|---|---|---|
| The ground truth | What was said and done | The transcript of your session (`list_events`), git, the pull requests. `RECORD.md` is your short list of the decisions of the person |
| Notes | What you found important | Pages in `memory/`. Few. One subject for one page |
| The dashboard | What each session reads at its start | `memory/MEMORY.md`, less than 60 lines |

The dashboard is small because each session pays for it, and because an error in it
goes into each session. A good dashboard has: the goal now and the subsequent steps,
the rules that you tested, the standing words of the person, and the list of pages.
Change it while you work; do not only add to it.

## 4. The dream pass

The post calls it "dreaming": between sessions, the agent compares its notes with the
transcripts, corrects the notes that are wrong, and puts related notes together. In the
post, one wrong note about a place sent later runs the wrong way; the corrected note
did not.

Do the pass BEFORE a compaction, while the full conversation is still there:

- when `get_session` (no session id) shows the context use above about 75% of the
  compaction threshold (`context_usage.used_tokens` against `autocompact_state.threshold`):
  look at it after each large piece of work;
- when the person wants to compact: the person says "dream" first, then compacts;
- when the person says "clean up memory" or "dream".

After a compaction (automatic or asked for), do only a short check: read the dashboard,
correct a note that the summary disagrees with, then continue the work.

1. Read the ground truth from the last pass to now: your conversation, your record, the
   merged pull requests.
2. Examine the notes that can change first. Correct what is wrong. Delete what is old or
   done.
3. Put notes about one subject together. Make the dashboard short again.
4. Write the date of the pass in the dashboard. Commit and push. Tell the person in a
   few lines what you changed.

**Your agents (optional).** The pass shows what your team does well and where it loses
time. If you see a way to make an agent of your team a little better, you can change
its file or a skill, in the way that helps your goal: a clearer instruction, a lesson
from an error, a better brief form. You can also do this at any other time. It is not
mandatory. The limits are in `chiefs/ORIENTATION.md`, section 3.

## 5. Limits

- One writer for each file. A shared file that several heads write loses facts.
- A note has its source and its date when the fact can change. Read the source again
  before you use an old note for a decision.
- Memory does not replace the word of the person.
- This repo is public: no secret, no personal contact detail, no private words.
- No hook writes memory, and no clock starts the dream pass (a compaction is an event of the
  session, not a clock). The dashboard does not load
  the pages. Read a page when it is necessary.
- The agents of your team have no memory. Put what an agent needs in its brief.
