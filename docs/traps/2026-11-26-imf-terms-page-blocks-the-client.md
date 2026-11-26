# The IMF web pages block the client; the IMF data API does not

*Trap:* `imf.org` and `data.imf.org` web pages (terms, data, commodity prices) answered "Access
Denied" (HTTP 403, edge service) to the honest client name on 2026-11-25. The terms were read by the
Head of Staff with another reader (2026-11-26): they permit use with attribution to the IMF and forbid
bulk download by automated technology. The API terms (`portal.api.imf.org`) are behind a login. The
data API `api.imf.org` answers HTTP 200 to the honest client name.

Four traps of the API:
- A key that does not match the dimensions of the dataflow gives HTTP 200 with an empty data set, not
  an error. The key of uranium is `G001.PURAN.USD.M` (COUNTRY, INDICATOR, DATA_TRANSFORMATION,
  FREQUENCY; the world is `G001`, not `W00`). The SDMX 3.0 structure call gives this order.
- The form depends on the `Accept` header. The program client (`mapcheck::http`) always sends
  `Accept: application/json, ...`, so the server answers SDMX-JSON, where `curl -H "Accept:
  application/xml"` gives XML. A reader for one form fails on the other (the first run did).
- In SDMX-JSON the value is a string, and the month of an observation is found by its index in
  `structure.dimensions.observation[0].values`.
- The source of the series is a third party (NUEXCO / TradeTech). Owner decision, 2026-11-26: read at each
  run, keep no copy, name the IMF.

*What to do:* Do not change the client name and do not use a mirror. Make one request for the one
series, never for the whole dataflow, and do not loop. Keep the answer in memory. The test fixture has
invented values.

*Evidence:* `rust/src/market/imf_pcps.rs` (`parse`, `read`, `log_line`); `api.imf.org` calls with the
client name, 2026-11-26 (6 requests in the first task, 2 in the second).
