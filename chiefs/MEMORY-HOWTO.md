# How the memory of a head works

A new session of a head starts with no memory of the last session. These files give it
a memory. The memory is files in git only. It has no database, no hook and no clock.
The rules that bind you are in `chiefs/ORIENTATION.md`, section 8. This page has the
forms and the clean-up pass.

## 1. The four layers

| Layer | What it is | Where |
|---|---|---|
| 1. The record | The exact words of the person, with the clock time, and what was done | `<your folder>/RECORD.md`; add at the end only |
| 2. The pages | One page for each thing that has exact terms: a data provider, an account, a model decision | `<your folder>/memory/<thing>.md` |
| 3. The index | What the next session must know, and the list of pages | `<your folder>/memory/MEMORY.md`, less than 60 lines |
| 4. What loads at each start | The pointer line and the orientation page | `CLAUDE.md`, `chiefs/ORIENTATION.md` |

A higher layer is short and points down. The orientation page says where the index is.
The index says where the pages are. A page says where its source is.

## 2. What goes in memory

| Put it in memory | Do not put it in memory |
|---|---|
| A standing word or a decision of the person, with its date | What the code, a ticket or a document says already: point to it |
| A trap of the environment and the way around it | A status that is old tomorrow (the board has the status) |
| The exact terms of a provider, an account or a source | A summary of finished work: git has it |
| An open item and where its work is | A secret or a personal contact detail |
| A fact with its source and the date it was read | A number with no source |

Memory is for the next session of the same head. Write what that session cannot find
quickly in a different place.

## 3. The index form

```markdown
# Memory index: <head> (<login>)
What the next session of this head must know and what is not written in another place.
Keep this file below 60 lines. Started <date>.

## Who and how
- <a standing word of the person, with its date>

## What stops work
- <a trap that cost time, and the way around it that breaks no rule>

## Pages
- `<thing>.md`: <one line>

## Clean-up pass
On the word of the person ("clean up memory"). Last pass: none.

## Open on <date>
- <an item that is not done, and where its work is>
```

## 4. The page form

One page for one thing.

```markdown
# <Name> (<what it is>)
One page for this thing: what we know, with the source of each fact. Written <date>.
Read the source before you use a fact.

State: <chosen / not chosen / active / closed>, with the words of the person and the date.

## <Topic, for example Limits>
Source: `<path or address>`, section <n>. Read <date>.
- "<the exact words of the term>"
- Not shown: <what the source does not say>

## Open
- <what is not known, and who can answer>
```

1. Quote a term word for word. Do not write it again in your own words.
2. Give the source of each fact and the date that you read it.
3. Write "not shown" when the source does not say. Do not fill the gap.
4. The source stays correct. The page collects; it does not replace.

## 5. The record form

```markdown
## <date> <time> UTC (<chat or dialog>)
Words: "<the exact words of the person>"
Done: <one short line>
```

- Get the time from the system clock (`date -u`). Write the entry in the same turn.
- Add at the end only. Do not change an old entry. A correction is a new entry.
- Record the words that decide or direct the work. Do not record small talk.
- **This repo is public.** Before you write an entry, remove each secret and each
  personal contact detail from the words, and write `[removed]` in its place. If the
  words are private and are not about the work, do not record them.

## 6. The clean-up pass

Do it when the person says "clean up memory". Do not do it on a clock.

1. Read your record from the last pass to today.
2. Compare each line of the index and each page with the record and the sources.
3. Correct what is wrong. Delete what is old or done.
4. Move a group of facts about one thing to its own page.
5. Repair each link to a file that moved.
6. Make the index short again.
7. Write the date of the pass in the index. Commit and push.
8. Tell the person in a few lines what you changed and what you deleted.

## 7. Do not build

| Do not build | Why |
|---|---|
| One memory file that all heads write | Two sessions that write one file make merge conflicts and mixed facts |
| An index that loads all pages at each start | It fills the context. Read a page when it is necessary |
| A hook that writes memory | It adds notes that no person and no head examined |
| A clean-up on a clock | The word of the person is sufficient at this size |

## 8. Faults

| Symptom | Possible cause | Remedy |
|---|---|---|
| A new session does not know a decision | The decision was in the chat only | Record it, then add one line to the index |
| The index is long | Facts about one thing were not moved to a page | Do the clean-up pass |
| A note is wrong | The source changed, or the note was not written from the source | Correct it from the source; write the date |
| Notes were lost | They were not pushed | Push in the same turn as the work |
