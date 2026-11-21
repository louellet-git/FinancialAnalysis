# Dashboard: staff (louellet-git)
What the next session of this head must know. Keep this file below 60 lines. Started 2026-11-21.
Rules: `chiefs/ORIENTATION.md`, sections 1, 5 and 8. Decisions of the person: `../RECORD.md`.

## Role (owner words 2026-11-21)
- The main contact of the person, above the other heads. Project management in the background: no code,
  no gate, no build, no approval for the person.
- Work autonomously; do not wait for the word of the person. Keep a backseat view.
- Three jobs: (1) the order of the large work, with the person; (2) the heads obey the head rules "within
  reason" (a note to the head, not a stop, unless safety, money or a secret); (3) the system is cohesive.
- The heads report to me rarely: only a result that is done, or done and for my review before it goes to
  the person (owner word). I send few notes too.
- Lean: no poll, no fixed report, few messages, few files. A look at the board when a message comes or
  the person talks is sufficient.
- Questions for the person: research first, then one clickable yes/no dialog with a recommendation.

## Frame for "cohesive" (lean form, owner word 2026-11-21: "a bit more lean")
1. Each large task serves a decision of the person.
2. One question has one answer, and the output of one head is the input of the next.
3. A head finishes before it starts.
4. Each result that goes to the person answers "does it make sense?" (`chiefs/ORIENTATION.md`, end of
   section 1). I stand between the person and the heads and ask it again, in a few lines, with no form.

## How I steer (owner words 2026-11-21: "this isn't meant to choke the heads")
- Steer, do not choke. A note names the goal and the gap; the head chooses the method. No new rule or form
  when a note is sufficient. Do not argue about a small point: let it go.
- Look for the two faults that the person had before: scope creep (a ticket or a story that grows past its
  decision) and a lost goal (work that no epic needs).
- What goes to the person: short, clear, ASD-STE100, high signal. The answer first, the numbers that
  matter, what is not known. If a document is long or not clear, it goes back to the head one time with
  the reason, before the person sees it.

## Main work (owner word 2026-11-21)
- Keep `PLANNING/board/EPICS.md` (epic -> story -> ticket) true: each open ticket in one story, each epic
  with the decision that it serves. Ticket `T-20261121-board-epics`. Look for: a ticket with no story, a
  story with no path to its end, old ticket text, a head that starts before it finishes.
- The order across the epics in `EPICS.md` is a proposal; the person has not agreed to it yet.

## Monitoring (owner word 2026-11-21: script only, no memory of a head, not token heavy, never a choke)
- Test before you constrict (owner rule): a hook, a script or a way to manage the heads that constricts
  them needs a few days of evidence that it is better than each alternative. Report-only tools can start.
- Story E6.S5: `T-20261121-board-check` (the board rules from git) then `T-20261121-usage-log` (a Stop hook
  counts agents, skills and tokens from the transcript file). Technology builds; I read the output.
- Until they exist: the hand checks of the board-check ticket, only when I look at the board for a
  different reason.

## Watch list
- E1 (stock trades) is on hold: no rule passed, T1 fails, and the person paused the fix for the week study.
- PR #131 (`fa algo`) waits for the live run of the person on Monday 2026-11-23.
- The week report (E2.S1) comes to me before it goes to the person. My sense check of the preliminary
  results (2026-11-21): one fact counted two times; market or model; 1.2-1.7% against 4.41%. The head
  agreed and adds a second model (`fa dcf market`: the lowest quarter of the history, not the lowest of all).
- Rule point, seen 2026-11-21: six tickets are on a branch only, not on main (arb-invariants, cboe-off,
  recon-lasting, revenue-old-tags, selftest-golden, trade-grids). "Ticket first" asks for the ticket on
  main before the work. Say it once to Quant and Technology at their next done message; no separate note.
- E6.S4 (controls of orders and of data terms) has no ticket yet (proposal of Quality and Risk).
