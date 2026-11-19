---
id: T-20261119-live-data-stream
title: Show a continuous stream of data on the screen
area: engine
status: backlog
owner: louellet-git
branch:
created: 2026-11-19
acceptance: The viewer shows a page that updates with no action of the person, with the age of each data set, and a background process keeps each data set inside its maximum age for one day with no person at the computer.
---
Owner request, 2026-11-19: a steady stream of data on the screen, with all sources
pulled in a consistent way. The owner said that this is not necessary now. Do not start
before `T-20261119-data-sync` is complete: the stream is the sync command that runs
without a stop, plus a page that shows the result.

Steps, in sequence:

1. A watch mode of the sync (`fa sync --watch`): a loop that updates each data set when
   it gets older than its maximum age. It keeps the request limit of each host. It writes
   a status file after each cycle.
2. A live page in the viewer (`fa serve`): the page asks the server for changes (or the
   server sends events). Views: prices and the last value of each model, value against
   price for a watch list, the earnings calendar, rates, and the age and the last error
   of each source.
3. Storage for a history that grows each day: one file for each ticker is correct for
   daily data. For data during the day, use an append-only store (for example SQLite).
   Decide when the volume needs it.
4. A computer that is always on: the process must run on a computer that stays on (the
   computer of the owner, or a small server). The cloud sessions of the agents stop
   after a period with no activity, so they cannot host it.

Decisions for the owner before the start:

- Speed of the data. The free sources are slow: daily prices, and option quotes that are
  about 15 minutes late (Cboe). Prices during the day need a paid data feed or the data
  API of a broker. Decide if daily data is sufficient.
- Terms of use. A process that asks a site API without a stop must respect the limits of
  the site. Yahoo and Nasdaq have no published API; a published API with a key (for
  example Tiingo) is the better base for a continuous process.
- The repo gives analysis only. A stream does not change this rule: no orders.
