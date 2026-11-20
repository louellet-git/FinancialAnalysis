---
name: data-source-onboarding
description: Use before and while you add a new data source or a new request pattern: read the official documentation and the terms, set an honest client name, request limits, cache rules, fixture tests, the first real test, the row in docs/DATA-SOURCES.md and the trap entry.
---

# Data source onboarding

Owner rule (`docs/DATA-SOURCES.md`): a source must give the past on request, because no
program runs all the time. A source that gives only the present is weak: say so.

## Steps
1. **Read the documentation and the terms of use** of the official site. Note the
   address and the date. If the terms forbid the use, or you cannot read them, stop and
   ask. Write "terms not read" in the table if so.
2. **Honest client.** The request names the program (`FinancialAnalysis fa (personal
   research)`). It does not imitate a browser. No personal data in the name. The SEC
   contact comes from the SEC contact file or the environment, not from a public file.
3. **Keys.** A key comes from the environment or `.env` (names in `.env.example`). Never
   print it (`rust/src/market/keys.rs`). A missing key is not an error: use the next
   source and say which key adds which source.
4. **Limits.** Set the request rate below the published limit. A block (HTTP 429 or 403)
   stops the host for the run. Two connection failures in a row stop it too
   (`docs/DATA-SOURCES.md`, rules for `fa sync`).
5. **Cache.** Write the answer to `data/raw/...` with a record: source, date, request.
   The update rule says when to ask again. Keep the filing date of each fact.
6. **Fixture tests** from a real answer or from the documentation (say which). No
   network in tests.
7. **First real test.** Run the real command once. Record the date, the count of rows or
   requests, and the result. Cross-check with a second source if one exists.
8. **Documents.** Add the row to `docs/DATA-SOURCES.md` (data set, sources in order, key,
   API or site, history on request, live test, command, `fa sync` name and age). Add
   the traps to `docs/HIDDEN-ASSUMPTIONS.md` with an evidence pointer.
9. If `fa sync` must update it, add the data set to `rust/src/sync/`.

## Check
- Terms read or marked "not read". Honest client name.
- Limits, block rule and cache rule are in the code.
- The live test has a date. The table and the trap list are updated.
