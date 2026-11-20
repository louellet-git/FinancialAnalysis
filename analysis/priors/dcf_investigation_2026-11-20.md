# Investigation of the automatic DCF (2026-11-20)

Ticket `T-20261120-dcf-investigation`. The automatic model (`fa dcf auto`, `fa dcf screen`) is a screen, not a
reviewed valuation. This file gives the parts of the difference between the model value and the market price, the
defects with their evidence, the market-implied values, the test against the later returns, and the corrections.

**Rule of the ticket.** The price is used to measure and to rank. No input is set from the price. Each correction has
a reason that does not use the price.

**Data and commands.** The cache `data/raw` of 2026-11-19 (read only). Model date `--as-of 2026-11-19`. The screen is
`fa dcf screen --tickers-file rust/mapcheck/universe/sp500.txt --cache data/raw --as-of 2026-11-19 [--rules K=V] [--implied]`
(4,000 paths, seed 7). The price is the last close of the price cache (2026-11-18). The point-in-time test is
`fa dcf pit --cache data/raw` (2,000 paths). The return test is `fa dcf returns --cache data/raw` (1,000 paths). The
scratch scripts that read the CSV files are not in the repository.

**Words.** "Ratio" is P50 / price. "Stable set" is 55 large and old companies that we chose by name before the first
run: KO PEP PG JNJ WMT COST MCD HD LOW MSFT AAPL GOOGL IBM ORCL CSCO TXN HON MMM UNP CAT DE LMT GD NOC RTX ABT MRK PFE
ABBV AMGN BMY MDT CL KMB GIS MO PM VZ T CMCSA XOM CVX DUK SO NEE UPS ADP ACN NKE SBUX TGT LIN SHW ITW ECL.

## 1. The starting fact (reproduced)

`fa dcf screen` of the 503 tickers of `sp500.txt`: 439 values, 438 with a price, 64 refused (bank 23, insurer 19,
REIT 19, new registrant 2, history too short 1).

| Measure | Value |
|---|---|
| Median ratio | 0.480 |
| Share with P50 above the price | 17% |
| Share with P50 at or below 0 | 10% (43 tickers) |
| Median P90 / P10 (tickers with P10 above 0; 237 of 439) | 10.6 |
| Median ratio, stable set | 0.525 |

The 43 tickers at or below zero: 27 are electric and gas utilities (AEE, ATO, AWK, CMS, CNP, D, DTE, DUK, EIX, ES, ETR,
EVRG, EXC, FE, LNT, NEE, NI, NRG, PCG, PEG, PNW, PPL, SO, SRE, WEC, XEL, CEG). Their free cash flow after capex is
negative or small because a rate base grows with debt. Three are REITs that passed as standard (CPT, EQIX, IRM; see
section 4.4). The others have SBC larger than the free cash flow (OKTA, RDDT, TWLO, TTWO, AXON, HOOD) or a capex peak
(AMZN, INTC, APD, BX, COHR, LVS, NCLH).

## 2. The parts of the gap (one convention at a time)

Each row changes one rule with `--rules K=V` and keeps all others. "Effect" is the change of the median ratio (all 438
tickers; stable set in brackets). "Median change" is the median over the tickers of `ln(P50 new / P50 old)`.

| Part | Convention now | Alternative (`--rules`) | Effect, all [stable] | Median change | Judgment |
|---|---|---|---|---|---|
| Risk-free rate | 10-year zero yield 5.13% (GSW 2026-11-13) | fixed 4.2% (`rf=0.042`) | +0.097 [+0.102] | +17% | convention |
| Equity premium | 4.41% | 3.5% (`erp=0.035`) | +0.060 [+0.053] | +12% | convention |
| Beta | 2/3 raw + 1/3 | raw (`beta_weight=1`) | +0.011 [+0.057] | +2% | convention |
| Beta | 2/3 raw + 1/3 | 1.0 for all (`beta_weight=0`) | -0.037 [-0.118] | -5% | convention |
| Equity weight | E / (E + debt) | 1.0 (`equity_weight_one=1`) | -0.052 [-0.040] | -7% | convention |
| Terminal growth | 2.25% | the 10-year zero yield (`terminal_rf=1`) | +0.123 [+0.115] | +20% | open (section 4.7) |
| Terminal growth | 2.25% | 3.0% (`terminal_growth=0.03`) | +0.029 [+0.031] | +6% | convention |
| Horizon | 10 years, fade to the cap, step to 2.25% | 20 years, growth stays at the cap after the fade (`years=20`) | +0.079 [+0.046] | +14% | convention |
| Horizon | as above | 20 years, linear fade to 2.25% (`years=20,long_cap=0.0225,fade_years=20`) | +0.009 [+0.015] | +2% | convention |
| Level of the fade | 10-year zero yield (5.13%) | the terminal growth (`long_cap=0.0225`) | -0.035 [-0.021] | -7% | open (section 4.7) |
| Level of the fade | as above | no fade (`fade_to_cap=0`) | +0.028 [+0.000] | +1% | convention |
| Growth shrink | weight from the table | no shrink (`growth_w=1`) | +0.057 [+0.001] | +14% | convention |
| Growth persistence | table phi 0.11 to 0.26 | 0.5 (`growth_phi=0.5`) | -0.010 [-0.026] | -3% | convention |
| Margin shrink | weight from the table | no shrink (`margin_w=1`) | +0.017 [+0.067] | +7% | convention |
| Rising margin | target = shrunk mean | target = margin of the base year (`margin_last=1`) | +0.055 [+0.042] | +13% | convention |
| SBC | a cash cost | not a cost (`sbc_cost=0`) | +0.079 [+0.035] | +9% | convention (IBM review F5) |
| Acquisitions | no cost; bought growth stays in the history | full charge (`acq_charge=1`) | -0.162 [-0.146] | -5% | open (section 4.6) |
| Bridge: investments | `long_term_investments` left out | in the bridge (`bridge_lti=1`) | +0.011 [+0.001] | 0% | defect, with the next row |
| Interest income | in the free cash flow | taken out (`no_interest_income=1`) | -0.024 [-0.016] | 0% | defect (IBM review F1a, F15) |
| Both rows above | | `bridge_lti=1,no_interest_income=1` | -0.007 [-0.015] | 0% | defect (section 4.2) |
| Operating leases | not debt | debt (`bridge_leases=1`) | -0.018 [-0.016] | -1% | convention (the lease cost is in the operating cash flow) |
| Share count | weighted average of the period | count at the end plus the dilution (`shares_latest=1`) | -0.002 [0.000] | 0% | defect, small (section 4.3) |
| Timing | year 1 is one full year away | from the model date; the part of year 1 in the balance sheet is left out (`stub=1`) | +0.012 [+0.014] | +3% | defect (section 4.1) |
| Timing | flows at the end of each year | flows in the middle of each year (`mid_year=1`) | +0.029 [+0.023] | +5% | convention |
| Base revenue | latest fiscal year | trailing four quarters | not measured | | convention; the stub row gives the time part |

Evidence: the result files `gap/<name>.csv` of the scratch folder; the table is the output of the scratch script on them.
The effects do not add: the model is not linear.

**What the table says.** No single defect explains the gap. The four defects together move the median ratio by about
+0.01 to +0.02. The large parts are conventions of the discount rate and of the long run: the risk-free rate of 5.13%
with the premium of 4.41% (cost of capital at the means: median 8.7%, stable set 7.9%), the terminal growth of 2.25%,
and the 10-year horizon. Each of these moves the median ratio by 0.06 to 0.12. To reach a median ratio of 1.0, the
market needs a combination of them that is far from the conventions (section 3).

## 3. Market-implied values (measurement)

`fa dcf screen ... --implied` searches, for each ticker, the cost of capital of year 1 that gives P50 = price. The
search moves the mean of the equity premium (the risk-free rate and the cap of the terminal growth stay; the guard
"terminal growth at most the cost of capital less 2%" stays). The long-run growth is the level of the fade that gives
the price (the existing column `implied_long_run_growth`).

| Measure (438 tickers with a price) | Model | Implied by the price |
|---|---|---|
| Cost of capital of year 1, median | 8.72% | 5.44% (P25 4.24%, P75 7.07%; 400 with a solution, 95 of them only an upper bound) |
| Implied less model, median (P10, P90) | | -3.11 points (-6.48, +1.00) |
| Cost of capital, stable set, median | 7.93% | 5.04% (51 tickers) |
| Level of the fade of the growth, median | 5.13% | 17.8% (P25 8.4%, P75 28.1%; 263 tickers with a solution from -10% to +40%) |

An implied cost of capital of 5.4% with a risk-free rate of 5.13% is an implied premium near 0.3% for a beta of 1. The
guard of the engine keeps the terminal growth 2 points below the cost of capital, so the search cannot go far below 4.3%:
95 tickers need a rate below the range. The implied values depend on all other numbers of the model.

**Consensus (the estimates cache, EPS only, 430 tickers).** The model has no EPS. We compare growth rates of different
things: the consensus EPS growth also has margin change and buybacks.

| Median | All | Stable set |
|---|---|---|
| Consensus EPS growth, next fiscal year | 10.2% | 7.2% |
| Consensus EPS growth, 2 to 3 years a year | 10.4% | 7.5% |
| Model revenue growth, year 1 | 6.0% | 3.6% |
| Consensus EPS growth (2 to 3 years) less model revenue growth (year 1) | +4.8 points | +4.7 points |
| Forward P/E of the market (price / EPS of the next fiscal year) | 17.8 | 18.5 |
| Forward P/E of the model (P50 / EPS of the next fiscal year) | 9.3 | 9.8 |

The model growth of the first years is about 4 to 5 points below the consensus EPS growth. Part of that is the shrink
to the cell mean (cell means for large companies: 2.8% to 3.7%), part is the difference between revenue and EPS.

## 4. Defects and open points of the code and rules

The inputs of `fa dcf auto T --explain` were compared with `mapcheck values T` for MSFT, AAPL, NVDA, XOM, JNJ, WMT, DUK,
KO, GOOGL and IBM. Revenue, operating cash flow, capex, SBC, interest, tax, debt and cash agree with the mapping for each
of the ten. The formula `(OCF - capex + (1 - t) interest paid) / revenue` charges SBC one time: the operating cash flow
adds SBC back, and the model subtracts the SBC ratio one time (`paths.rs`, `FCFF = revenue (margin - SBC - charge)`). No
step counts SBC two times or zero times. The terminal value is `FCFF(N) (1 + g) / (k - g)` with the cost of capital of
year N (correct Gordon form). The caps of the terminal growth (target of the growth process, the risk-free rate, the cost
of capital less 2%) work as documented.

### 4.1 Defect: the discount ignores the date of the model, and year 1 counts cash that is already in the bridge

*Category (a) and (b).* The engine discounts the cash flow of year 1 by one full year (`disc /= 1 + k` at `t = 1`). Year 1
is the fiscal year after the base year. On 2026-11-19, for a company with a December year, year 1 is 2026, which ends 42
days later. The cash flow of the first three quarters of 2026 is also already in the cash and the debt of the 10-Q of
2026-09-30 that the bridge uses. So the model counts that part two times, and it discounts all later flows about 0.9 years
too much. Correction: `[discount] stub_years` (years from the model date to the end of year 1) and `year1_share` (the share
of year 1 after the balance sheet date); the engine uses them only when a model file has them (the hand models do not).

### 4.2 Defect: interest income is in the free cash flow, and the cash that earns it is in the bridge

*Category (b).* The operating cash flow has the interest received. The bridge subtracts the cash. The IBM review (F1a and
F15, closed decisions) sets the rule: FCFF = OCF + (1 - t)(interest expense - interest income) - capex - SBC, and a stock is
in the bridge only if its flow is not in the margin. The automatic model adds back the interest expense but keeps the
interest income. KO: interest income $0.79B, cash $13.9B; IBM $0.65B; GOOGL $4.3B. The reverse error: the marketable
securities and other investments outside "cash and short-term investments" (`long_term_investments`: AAPL $80.2B, GOOGL
$68.7B, MSFT $36.3B) are in no part of the model. Correction: take the after-tax `interest_income` out of the margin and put
`long_term_investments` in the bridge. Equity-method stakes (XOM $32.7B, KO $20.2B) stay out: their dividends are in the
operating cash flow. Limit: MSFT and AAPL have no value of `interest_income` in the mapping, so for them only the second
part works.

### 4.3 Defect, small: the share count is the weighted average of the period

*Category (a).* The value per share on the model date needs the count on that date. The model uses
`shares_diluted`, a weighted average of the period. Correction: `shares_outstanding` at the end of the period plus the
dilution of the period (`shares_diluted - shares_basic`), with a guard (the new count must be within 85% to 110% of the
weighted count, else the weighted count). Effect: AAPL +0.9%, XOM +1.5%, the median 0%.

### 4.4 Defect: banks, lenders and REITs that pass as standard

*Category (a).* `filer_template` (`rust/mapcheck/src/checks.rs`) reads only the tags `Deposits`,
`RealEstateInvestmentPropertyNet` and `RealEstateInvestmentPropertyAtCost`. Evidence in the company facts: PNFP tags
`DepositsDomestic` ($47.4B of $57.7B assets); GBCI has `InterestExpenseDeposits` and loans of $20.7B of $32.0B; OMF has
`NotesReceivableNet` $21.2B of $27.4B and no current-assets total; VICI has a net investment in sales-type leases of $23.7B
of $46.7B; CPT tags `RealEstateGrossAtCarryingValue` $14.0B less depreciation $5.3B of $9.0B. CPT also has a mapping error
(revenue $13M from a small contract-revenue tag, P50 -$53). Correction: section 6.

### 4.5 Open: utilities

27 utilities have P50 at or below zero. A free-cash-flow DCF with the margin of the history is the wrong tool for a
regulated rate base that grows with debt (the same reason as for banks). No rule refuses them now. A template for
utilities (allowed return on the rate base) is the next step; we did not make one.

### 4.6 Open: acquisitions

The growth history keeps growth that cash for businesses bought (a jump below 60% stays, #185), and the free cash flow
has no cost for it. The IBM review (F2, F16) calls this a critical error and pairs the growth with a charge. The full
charge (`acq_charge=1`: the mean of `acquisitions_net_of_cash` / revenue of the margin years) lowers the median ratio by
0.16 (JNJ -67%, IBM -68%, AVGO -47%, ABBV -30%). The shrink of the growth to the cell mean already takes out much of the
bought growth, so the full charge counts too much. The data has no bought revenue, so the correct pair is not known. We
added the rule as an option and did not change the default.

### 4.7 Open: two numbers for one long-run growth

The fade of the company growth goes to the 10-year zero yield (5.13%, "a proxy for the long-run nominal growth of the
economy", #189). The terminal growth is 2.25%. So the growth falls from up to 5.13% in year 10 to 2.25% in year 11. Also,
the equity premium of 4.41% is the implied premium of Damodaran (source of the repo value), which he calculates with a
long-run growth equal to the risk-free rate. The two alternatives (`terminal_rf=1`: +0.12; `long_cap=0.0225`: -0.04)
make the model consistent in two opposite directions. Which one is correct is a decision for the owner.

### 4.8 Other checks (no defect)

- Tax: XOM 27.9%, MSFT 18.2%, AAPL 16.2%: the effective rate when stable, else 21% (NVDA). Correct by the rule.
- NVDA: the level rule of the growth shock (`level_kappa`, #188) makes the noise of the history large, so the shrink weight
  of its growth history is small (company mean 10.6% against a history of 65% to 126%). Without the level rule P50 is $31.78
  (now $17.48). The beta is 1.68 (cost of capital 12.5%). This is the design of #188, not an error of code.
- 53-week years (JNJ) pass the 350 to 380 day rule; the extra week adds about 2% to the growth of one year. Small.
- IBM capex has no capitalized software (`capitalized_software` has no value): a mapping limit, outside this ticket.

## 5. The value against the later returns (measurement)

`fa dcf returns --cache data/raw` (new command). For each company of the standard template with a price file (429
companies) and each fiscal year T: the model from the fiscal years up to T only (as `fa dcf pit`), the balance sheet of T,
the GSW curve, the beta and the price of the date D = end of T + 100 days (a 10-K is due 60 to 90 days after the year end;
the first trading day on or after D). Returns are the price change from D to D + 1 year and D + 3 years. The price source
is adjusted for splits, not for dividends. The lines "with dividends" add the yield `dividends_paid / (shares x price)` for
each year (median yield 1.2%). The standard error uses the years as clusters (the mean of the yearly rank correlations,
the standard deviation over the square root of the number of years; years with fewer than 20 observations are not used).

| Horizon | n | Pooled rank correlation | Mean of yearly correlations (SE, t) | Q1 / Q2 / Q3 / Q4 / Q5 return less the year mean | Q5 - Q1 (SE) |
|---|---|---|---|---|---|
| 1 year, price | 3,491 | +0.091 | +0.056 (0.034, 1.6), 9 years | +7.4 / -4.2 / -3.4 / -2.5 / +2.8% | -3.8% (6.2%) |
| 1 year, with dividends | 3,491 | +0.103 | +0.069 (0.034, 2.0) | +6.8 / -5.3 / -4.3 / -2.9 / +5.8% | -0.2% (6.9%) |
| 3 years, price | 2,658 | +0.143 | +0.121 (0.031, 3.9), 7 years | +9.1 / -16.4 / -12.5 / -0.4 / +20.4% | +11.1% (13.3%) |
| 3 years, with dividends | 2,658 | +0.157 | +0.134 (0.031, 4.3) | +6.8 / -20.2 / -15.3 / -2.1 / +31.2% | +24.1% (15.2%) |

By size (3 years, price, mean of yearly correlations): 1-10B +0.141 (t 3.4), 10-30B +0.165 (t 5.8), 30-100B +0.026 (t 0.3),
above 100B +0.044 (t 0.5). By year (1 year): from -0.09 (2024) to +0.19 (2020); 7 of 9 years are above zero at 3 years.

**Reading.** (1) The rank has a small positive relation with the later returns: at 3 years the correlation is about 0.12
to 0.14 in each year. The 3-year windows of near years overlap by two years, so the real standard error is about 1.7
times larger (t about 2.3 to 2.5). (2) The relation is not monotonic: the lowest quintile (P50 near or below zero; growth
companies that burn cash or have large SBC) had high returns too, so Q5 - Q1 is not significant. Q2 and Q3 had the lowest
returns. (3) The relation comes from companies below $30B of revenue; for the largest companies there is none.
(4) Survivorship: the companies are those of the cache now, mostly the S&P 500 of 2026. A company that grew into the index
is in the test with the years before it entered; such companies are growth companies with a low ratio and high later
returns. Companies that failed or left the index are not in the test. The first effect makes the lowest quintile look better
than it was; the size of the second is not known. The relation of the middle and the top quintiles is less exposed.

## 6. Corrections (Part 2)

We made each correction one at a time, in this order. For each: the reason (section 4), a test that failed before the change
and passes after it, the smallest change, then the screen (`--as-of 2026-11-19`) and `fa dcf pit` again. The scratch files are
`corr/screen_<step>.csv` and `corr/pit_<step>.txt`. Before the corrections we ran `cargo test`: the three tests of C2 to C4 failed
(538 passed, 3 failed); after them all 541 pass.

| Step | Change | Test | Values with a price | Median ratio | P50 at or below 0 | Median P90 / P10 | Median ratio, stable set |
|---|---|---|---|---|---|---|---|
| C0 | none (2026-11-19 rules) | | 438 | 0.480 | 10% | 10.64 | 0.525 |
| C1 | template detection (4.4) | `banks_lenders_and_reits_with_other_tags_are_not_standard` | 430 | 0.485 | 9% | 10.59 | 0.525 |
| C2 | discount from the model date, year-1 share (4.1) | `the_default_model_starts_the_discount_on_the_model_date`, `the_timing_of_the_discount_follows_the_model_date_and_the_balance_sheet` | 430 | 0.499 | 9% | 11.08 | 0.539 |
| C3 | interest income out, `long_term_investments` in (4.2) | `interest_income_leaves_the_margin_and_the_investments_enter_the_bridge` | 430 | 0.489 | 9% | 10.74 | 0.523 |
| C4 | share count at the end plus dilution (4.3) | `the_share_count_is_the_count_at_the_end_plus_the_dilution` | 430 | 0.489 | 9% | 10.74 | 0.523 |

C1 refuses 8 tickers of `sp500.txt` (AMT, CCI, CPT, EQIX, HST, IRM, SBAC, VICI: now REIT). In `sp500.txt` and `next500.txt`
together 18 tickers change (#264): to REIT also LAMR, LINE, MAC, OUT, RHP; to bank AFRM, CNH, GBCI, OMF, PNFP. CNH is a maker
of machines whose finance arm is most of its assets; the rule refuses it as a lender. That is a judgment that the owner can reverse.

Point-in-time bands (share of the real values in P10 to P90, target 80%; all company-years):

| Step | Company-years | Revenue T+1 / T+2 / T+3 | Free cash flow T+1 / T+2 / T+3 |
|---|---|---|---|
| C0 | 6,128 of 725 companies | 86.1 / 83.8 / 86.9 | 87.1 / 86.2 / 85.6 |
| C1 | 6,030 of 712 | 86.1 / 83.8 / 86.9 | 87.1 / 86.3 / 85.6 |
| C2 | 6,030 of 712 | no change (the test has no timing) | no change |
| C3 | 6,030 of 712 | 86.1 / 83.8 / 86.9 | 87.1 / 86.3 / 85.5 |
| C4 | 6,030 of 712 | no change (the test has no share count) | no change |

The tests of revenue and free cash flow do not move: the corrections are in the value, not in the paths.

Ten companies, P50 before and after (price 2026-11-18): MSFT $215.88 to $226.59 ($508.29); AAPL $113.68 to $121.82 ($352.45);
NVDA $17.12 to $15.92 ($150.96); XOM $152.56 to $157.81 ($160.58); IBM $138.89 to $131.48 ($228.53); JNJ $149.99 to $144.31
($272.70); WMT $32.82 to $33.47 ($127.36); KO $31.96 to $29.23 ($98.09); GOOGL $105.19 to $113.28 ($282.44); DUK -$24.14 to
-$19.87 ($130.05).

The return test after the corrections (`analysis/priors/auto_returns_2026-11-20.txt`, 3,833 observations of 421 companies) gives
the same result: 3-year rank correlation +0.123 (SE 0.027, 7 years), 1-year +0.055 (SE 0.032, 9 years).

The reviewed numbers do not change: `fa dcf drivers IBM` $83.84 / $151.84 / $258.03; JNJ P50 $112.64; ANT P50 $30.97; the ANT
path model P50 $25.01; the IBM path comparison file $79.17 / $154.03 / $277.20.

**Conventions with an alternative (no default changed; the owner decides).** All rows of the table of section 2 are options of
`--rules`. The four that a source supports better than the present rule, in our judgment:

| Option | Effect on the median ratio | Source and reason | Recommendation |
|---|---|---|---|
| `terminal_rf=1` (or `long_cap=0.0225`) | +0.12 (or -0.04) | The implied premium of Damodaran (the source of 4.41%) uses a long-run growth equal to the risk-free rate; the fade of the model already uses the yield as the long-run growth (#189) | Choose one of the two, so that the premium, the fade and the terminal growth use one long-run growth. `long_cap=0.0225` keeps the conservative side. |
| `mid_year=1` | +0.03 | Koller, Goedhart, Wessels, *Valuation* (McKinsey), the mid-year adjustment: cash comes during the year | Make it the default if the hand models also change; else keep the two the same. |
| `acq_charge=1` | -0.16 | IBM review F2 and F16 (closed decisions): bought growth needs its cost | Use a paired rule (for example half the ratio, as the IBM version 4 recurrence of 0.5) only after a review of the same weight. |
| `bridge_leases=1` | -0.02 | none better: the lease cost is in the operating cash flow, so the stock-and-flow rule keeps leases out | Keep the default. |

## 7. Conclusion

1. The model is conservative by its conventions. The median value is about half the price before and after the corrections
   (0.480 to 0.489).
2. The four defects (discount timing, interest income and investments, share count, template detection) are real but small:
   together +0.01 on the median ratio and 8 REITs out of the screen.
3. The largest parts are conventions of the discount and the long run: the rate of 5.13% with the premium of 4.41% (cost of
   capital 8.7%), the terminal growth of 2.25%, the 10-year horizon, and SBC as a cost. Each moves the median ratio by 0.06 to 0.12.
4. The prices need a cost of capital of about 5.4% (median), 3.1 points below the model, or a long-run growth of about 18%
   at the model rate. The consensus EPS growth is 4 to 5 points above the revenue growth of the model in the first years.
5. The model has two numbers for one long-run growth (5.13% and 2.25%). This is the one internal conflict; the owner must
   choose (section 6).
6. Acquisitions are free in the model (IBM review F2). A full charge lowers the median ratio by 0.16; the correct charge is
   not known from the data. It is an option.
7. Utilities (27 of 43 values at or below zero) need their own template.
8. The ranking has a little information: the rank correlation with the 3-year return is about +0.12 (t about 2.4 when we
   correct for the overlap), only for companies below $30B of revenue, and the order is not monotonic.
9. Do not use the automatic value as a price target. Use it, with care, to rank smaller companies.
10. The bands (median P90 / P10 near 11) stay wide; the point-in-time test shows them a little too wide (86% against 80%).
