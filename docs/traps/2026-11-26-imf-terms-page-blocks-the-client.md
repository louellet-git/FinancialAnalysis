# The IMF terms page answers HTTP 403 to the program client

*Trap:* `imf.org` and `data.imf.org` pages (terms, data, commodity prices) answered "Access Denied"
(HTTP 403, edge service) to the honest client name on 2026-11-25. The terms for a stored copy are
not read, so the IMF uranium series (PURAN) is not loaded. The World Bank Pink Sheet has no
uranium column. The FRED copy is not permitted in this repo.

*What to do:* Do not change the client name to get through. A person reads the IMF terms in a browser
and writes the answer in the ticket; then the loader can add a series to `SERIES` in
`rust/src/market/commodity.rs` (the IMF data API is a different file form: a new reader is needed).

*Evidence:* `fa commodity get uranium` (refusal text); `curl` with the client name on five IMF
addresses, 2026-11-25, all HTTP 403.
