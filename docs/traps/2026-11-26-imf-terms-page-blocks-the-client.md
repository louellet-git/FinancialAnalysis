# The IMF web pages block the client; the IMF data API does not

*Trap:* `imf.org` and `data.imf.org` web pages (terms, data, commodity prices) answered "Access
Denied" (HTTP 403, edge service) to the honest client name on 2026-11-25. The terms were then read
by the Head of Staff with another reader (2026-11-26): they permit a stored copy with attribution
to the IMF, and they forbid bulk download by automated technology. The data API host
`api.imf.org` answers HTTP 200 to the honest client name. An SDMX data call with a key that does not
match the dimensions of the dataflow gives HTTP 200 with an empty data set, not an error. The
dataflow call `.../dataflow/IMF.RES/PCPS/+?references=datastructure` gave HTTP 204 (no content).
The PCPS keys `M.W00.PURAN.USD` and `..PURAN.` gave an empty data set. The right key is
`G001.PURAN.USD.M` (dimensions COUNTRY, INDICATOR, DATA_TRANSFORMATION, FREQUENCY; the world is `G001`,
not `W00`). The SDMX 3.0 structure call (`/external/sdmx/3.0/structure/dataflow/IMF.RES/PCPS/+?references=all`)
gives this order. The full-series answer without `lastNObservations` did not parse (2026-11-26); check
the answer text that the error message prints.

*What to do:* Do not change the client name and do not use a mirror. Use the key above and make one
small request for the one series (`lastNObservations` or `startPeriod` limits it). Never ask for the
whole dataflow. Show the source line "Source: International Monetary Fund, Primary Commodity Prices,
link" with each shown value.

*Evidence:* `curl` with the client name on `api.imf.org/external/sdmx/2.1/data/PCPS/...`,
2026-11-26 (3 requests); `fa commodity get uranium` (refusal text).
