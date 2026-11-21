# A-002: acquisition charge of the automatic model
Updated: 2026-11-21.
## Choice
The model charges `w` times the expected spend / revenue, 0.00% + 0.5226 x past (`acq_persist`; past = the mean of the last 5 fiscal years, capped at 25%; the fit through the origin of half 0, see Limits). A company with no past purchases gets no charge. Spend = `acquisitions_net_of_cash` plus `acquisitions_net_of_cash_second`, the second line only when its value differs from the first line or the first line has no value (`acq_second`; a tag renamed between 10-Ks gives the same value in both lines). `w` is the weight of the history in the company mean of the growth. A year with no value counts as 0. The growth history stays. The charge fades with the growth that it pays for (`acq_fade`): if the company mean is above the fade level L, the credited growth above L falls linearly to zero along the drift ramp, and the charge falls to zero in the same years. If the mean is at or below L, the charge is constant. `acq_charge` and `acq_f` are never faded. Defaults: `acq_w` on (`--rules acq_w=1`), `acq_fade` on (`--rules acq_fade=1`), `acq_second` on (`--rules acq_second=1`), `acq_keep` off (`--rules acq_keep=0`) (on: the charge keeps the share L / mean).
`--rules acq_persist=off` gives the past spend as the base (the rule of 2026-11-20); `acq_persist_a`, `acq_persist_b` set a + b x min(past, 25%); after `off`, one of them takes the default of the other (0.0 or 0.5226).
The fit applies to the `acq_w` charge only. `acq_charge`, `acq_f` and the `chain` cut of the growth history use the actual past spend.
## Where used
`docs/models/auto-path-model.md`; `rust/src/dcf/auto.rs` (`acq_w`, `acq_fade`, `acq_second`, `acq_persist`, `acq_keep`); `rust/src/dcf/auto_pit.rs` (`fcff_of_rules`: the same spend).
## Alternatives considered
| Alternative | Command | Why not chosen |
|---|---|---|
| No charge (old model) | `--rules acq_w=0` | Overstates the free cash flow after acquisitions: -1.7% of revenue in year 1 |
| Constant charge | `--rules acq_fade=0` | The charge does not fade with the credited growth |
| First line only | `--rules acq_second=0` | Loses real cash paid (NVDA FY2026: $1.5B of $14.5B) |
| Past spend as the base | `--rules acq_persist=off` | Over-charges heavy buyers: error +1.1 to +1.8% of revenue (half 1) |
| Intercept rule 1.75% + 0.3925 x past | `--rules acq_persist_a=0.0175,acq_persist_b=0.3925` | Charges about 1.5% of revenue to a company with no purchases and credits no growth for it (KR, AZO, COST) |
## Why this choice
None of the reasons uses the price. Study 2.6: a consistent pair by construction. The charge pays for the credited growth: the model credits the share `w` of the bought growth, so it charges the share `w` of the spend. A company with no past purchases has no bought growth in its history, so it gets no charge (pairing). The base is the expected next spend, not the past spend: a heavy buyer spends less later (study 2.1: the next 3 years are 0.55 of the past at the mean). The slope is the half-0 fit through the origin (`fa dcf pit --half 0 --diag acq`: 0.5226). The fade keeps the pair true in each year. The second line is real cash paid for businesses. Choice order: cycle 1 measured the past spend, the intercept rule and the rule through the origin (table below); the rule was chosen after those scores were seen. Its reason is the pairing. The rule gives up about 0.8 points of
error (% of revenue) against the intercept rule below 5% of spend, and 0.3 to 0.9 points against the past spend at
0.5% to 5% (tables below).
## Measurements (store 1804f48, seed 7, 2000 paths each, 5,097 company-years of 636 companies)
Commands: `fa dcf pit --cache data/raw --half 1 [--cut 2015] --rules R`; R = none (default),
`acq_persist=off` (past), `acq_persist_a=0.0175,acq_persist_b=0.3925` (intercept). Free cash flow error T+1 / T+2 / T+3 in % of revenue, half 1; inside P10 to P90 at T+1; real below P10 / above P90 at T+1.
| Class (n) | Default | Past | Intercept |
|---|---|---|---|
| spend < 0.5% (2,120) | -1.1 / -1.1 / -0.2 | -1.1 / -1.1 / -0.2 | -0.3 / -0.2 / +0.7 |
| 0.5% to 2% (853) | -1.5 / -1.3 / -0.6 | -1.2 / -1.0 / -0.3 | -0.7 / -0.4 / +0.2 |
| 2% to 5% (787) | -1.7 / -1.6 / -1.3 | -0.8 / -0.8 / -0.5 | -0.9 / -1.0 / -0.7 |
| above 5% (1,321) | -1.4 / -1.3 / -0.9 | +1.1 / +1.2 / +1.8 | -1.2 / -1.1 / -0.8 |
| above 5%: inside, below, above | 70.1, 23.3, 6.6 | 66.3, 19.6, 14.1 | 70.2, 23.2, 6.7 |
| cut 2015, above 5% (1,084) | -0.8 / -0.4 / +0.1 | +1.2 / +1.7 / +2.5 | -0.7 / -0.2 / +0.2 |
| cut 2015 above 5%: inside, below, above | 64.1, 26.3, 9.6 | 58.9, 22.9, 18.2 | 64.6, 25.8, 9.6 |
| cut 2015, spend < 0.5% (1,657) | -0.3 / +0.6 / +1.6 | -0.3 / +0.6 / +1.6 | +0.4 / +1.2 / +2.3 |
Screen (`fa dcf screen --tickers-file rust/mapcheck/universe/sp500.txt --cache data/raw --as-of 2026-11-19`,
4,000 paths, 431 values). Median P50 change of the default, by margin target (4 classes), against the
intercept: +34.6% (n 15) / +7.6% (59) / +3.1% (155) / +1.3% (163); against the past: +1.4 / +0.6 / +0.7 / +1.7%.
P50 default (intercept): KR 67.39 (3.99); AZO 2630 (2539); COST 413 (335); MCK 2109 (1567); IBM 123.03
(116.09); ROP 494 (501); ILMN -0.85 (-1.10). Median P50 / price 0.5205 (past 0.4897): a measurement only.
## Limits (ticket `T-20261120-dcf-acq-charge-limits`)
- **F2.** Coverage of "above 5%" improves because the upper tail shrinks (above P90 14.1% to 6.6%); the
  share below P10 rises (19.6% to 23.3%). The test is a median test; the charge is a mean.
- **F3.** The hold-out splits by company only. The half-0 fit uses years after 2015: the cut at 2015 is
  not out of time for the charge. The rule form was chosen after a look at the half-1 slopes. Swapped halves (fit on half 1: 0.5297 x past, or 0.0206 + 0.3670 x past; score on `--half 0`, "above 5%"): inside 71.0 (origin) and 71.5 (intercept); error -0.9 / -1.5 / -1.0 and -0.6 / -1.3 / -1.0. The score is the same there; the intercept is better by about 1 point in the classes below 5%.
- **F4.** The fit applies to the `acq_w` charge only; `acq_charge` and `acq_f` use the past spend, and the
  `chain` cut uses the actual past spend. **F5.** Each figure names its command and paths (Measurements).
  **F6.** "Why this choice" has no P50 / price figure; the screen figures above are a measurement only.
- Step at L: `acq_keep` removes it (ROP 357 to 186); a pit of 3 years cannot test it. 8 of 503 tickers change
  with the second line (BIIB, CBRE, J, MRK, NVDA, TRGP, ZBH, ZTS).
- The panel (`rust/src/dcf/panel.rs`, about line 295) reads the first acquisition line only. Known limit; not changed.

## Who decided and when
The Head of Fundamental Research, on the word of the owner of 2026-11-20 (record: `chiefs/louellet-git/fundamental/RECORD.md`). 2026-11-21, after the measurement: `acq_second` and `acq_persist` on; `acq_keep` off. Cycle 2 of review, 2026-11-21: the rule through the origin is the default (pairing). Open question (owner: the same head): is the growth at L paid by organic reinvestment (then the charge goes to 0 for each mean) or in part by acquisitions (then `acq_keep`)? Review (source rule 3): not done.
## When to examine it again
When sector data exists.

