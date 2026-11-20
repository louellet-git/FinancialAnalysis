# A date can have several option snapshot files, and a reader takes the last one

*Trap:* `fa options snapshot --source alpaca` and `fa options archive` write
`data/raw/<T>/options/<date>T<hhmm>.json` (time in UTC) and not `<date>.json`. A date can have
the files `<date>.json` (Cboe, closes), `<date>T0930.json`, `<date>T1542.json` and the gzip copy
`<date>T1542.json.gz`. A reader that is given a date only takes the LAST file of that date (the latest
time). A person who wants the open quotes and gives `--date D` gets the last quotes. The time of a
file is in UTC, and `snapshot_date` is the UTC date of the run. A run after 00:00 UTC has the next date.
Cboe gives the VX settlement file for a date before today (UTC) only: an archive run on the day
copies the settlement of the trade date before.

*What to do:* Give `--time HHMM` (UTC) to `fa options check` and `fa vol fit` for one file. Do not
compare two dates with `--compare` when one date has a half-day file: the check takes the last file
of each date. A snapshot with a time is never replaced by closes (#240).

*Evidence:* `rust/src/market/options.rs` (`load_snapshot_at`, `stems_of_date`),
`rust/src/market/options_archive.rs` (tests `the_time_and_the_file_name_round_trip`,
`closes_never_replace_a_timed_live_file`).
