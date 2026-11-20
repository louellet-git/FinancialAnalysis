# Study of the value models for banks and for utilities (2026-11-20)

Ticket `T-20261120-bank-utility-study`. This is a study. It adds no model to `fa dcf auto`. It gives the data that
we have, the model form, a first calculation, and a design for the two tickets of the models
(`T-20261119-evaluator-bank`, `T-20261120-evaluator-utility`).

**Rule of the study.** The price is for comparison only. No input is set from the price.

**Data and tools.** The cache `data/raw` of 2026-11-19 (read only). Template detection:
`mapcheck check --list` of the branch (it includes the changes of the investigation, #264). The scratch scripts are in
the scratchpad folder of the session; they are not in the repository. Each table names its script and its output
file. All outputs are in `out/` of the scratchpad folder
(`/tmp/claude-0/-home-user-FinancialAnalysis/93b82429-52bd-5ac2-b515-0ec814ae8eea/scratchpad/templ/`). The scripts read
the raw company facts (10-K, fiscal-year facts, the latest filing for each end date) and the output of
`mapcheck check --emit` (the standard variables). The AR(1) estimators are a port of `fit_ar1` of `calibrate.rs`
(pooled, within, within with the Nickell correction, cluster-robust standard errors; script `ar1.py`).

**Words.** "ROE" is net income to common holders in year t divided by common equity at the end of year t-1. Common
equity is `equity` less preferred stock. "ROTE" is the same income divided by tangible common equity (common equity
less goodwill less other intangibles) at the end of year t-1. "Payout" is (common dividends + buybacks) / net income to
common. "Clean-surplus gap" is the change of common equity less (income - dividends - buybacks), divided by the equity of
the year before. It is the part of the equity growth that retained earnings do not explain (acquisitions that use shares,
other comprehensive income, share issues).

**Published source.** The excess-return model (also "residual income" or "equity excess return" model) is described in
A. Damodaran, "Valuation Approaches and Metrics: A Survey of the Theory and Evidence" (2005), section 2.4
(Now Publishers, FIN-013, <https://www.nowpublishers.com/article/DownloadEBook/FIN-013>). The web search found the
section but not the equation text. The formulas below are the identity that we derive in section A3; they use no number
from memory.

---

# Part A: banks

## A1. The set

Command: `mapcheck check --list --cache data/raw --tickers-file <sp500 + next500>` (output `template_list_universe.txt`).
Result: the two universe lists have 1,003 tickers. The template `bank` has 53 of them (23 in `sp500.txt`, 30 in
`next500.txt`). The earlier panel had 48 banks. The five new ones (AFRM, CNH, GBCI, OMF, PNFP) come from the change of
the detection in the investigation. The whole cache has the same 53 banks (`list_cache.txt`: no bank outside the two
lists).

The detection finds more than deposit banks. We sorted the 53 with the data (`classify2.py`, `out/bank_classify.txt`:
deposits / assets, loans / assets, net interest income / assets, share of noninterest income in revenue) and with the
names. The assignment of a ticker to a kind is a judgment; the table gives the data behind it.

| Kind | Tickers | Number | Data that shows the kind |
|---|---|---|---|
| Deposit banks (loans funded by deposits): large and regional | BAC C JPM PNC TFC USB WFC; BOKF BPOP CBC CBSH CFG CFR COLB EWBC FCNCA FHN FITB FNB GBCI HBAN HOMB KEY MTB ONB PB PNFP RF SSB UBSI UMBF VLY WAL WTFC ZION | 35 | deposits 53% to 88% of assets; net interest income / assets 1.8% to 4%; noninterest share 13% to 48% |
| Card and consumer lenders | AXP COF SYF ALLY OMF; AFRM SOFI (young, loss history) | 7 | net interest income / assets 5.8% to 15.5% for AXP COF OMF SYF; AFRM and SOFI have less than 5 years of profit |
| Custody and trust banks | BNY STT NTRS | 3 | noninterest share 70% to 79%; loans 13% to 24% of assets |
| Brokers and investment banks | GS MS SCHW RJF SF FRHC | 6 | loans below 20% of assets or noninterest share above 77%; deposits are a funding line, not the business |
| Not a bank (false members) | CNH (a maker of machines with a finance arm), FAF (a title insurer with a trust bank) | 2 | CNH: revenue is machine sales; FAF: no net interest income, 33% deposits only because of its escrow bank |

Asset managers (BLK is an insurer by the template, TROW IVZ BEN KKR BX APO ARES CG are `standard`) and exchanges (ICE CME
NDAQ CBOE are `standard`) are not in the bank template. They are not a bank problem. Mortgage REITs (NLY, AGNC) are
`standard` with status FAIL (`list.txt`); they belong to the REIT ticket.

**Proposed split.** One engine (excess return on book equity) with four cells of parameters:
(1) deposit banks, 35; (2) card and consumer lenders, 5 (AXP COF SYF ALLY OMF; AFRM and SOFI wait for more history);
(3) custody banks, 3; (4) brokers and investment banks, 6. The cell 4 has no persistence of ROE (section A4), so it must
not get a value in the first version. CNH and FAF go to `standard` and `insurer`: this is a defect of the detection to
fix in the model ticket (the rule "deposits at least half of assets" catches FAF through a tag that is not a deposit
line of a bank; for CNH the rule for deposit-taking lenders).

## A2. The variables

Scripts: `build_bank.py` (the values), `coverage.py` (`out/bank_coverage.txt`), `tagcount.py` (`out/bank_tagcount.txt`).
"All 10" is the number of banks with a value in each of the last 10 fiscal years (only 48 of the 53 have 10 fiscal
years). ">= 8" is the number with at least 8 of the last 10. "Latest" is the number with a value in the latest fiscal year.
The mean is the mean number of years with a value, out of 10.

| Variable | In the registry now? | All 10 | >= 8 | Latest | Mean years |
|---|---|---|---|---|---|
| net_interest_income | yes (`InterestIncomeExpenseNet`) | 44 | 44 | 48 | 8.7 |
| noninterest_income | yes (`NoninterestIncome`) | 43 | 43 | 47 | 8.5 |
| noninterest expense | **no** | 46 | 46 | 49 | 8.9 |
| provision for credit losses | **no** | 44 | 46 | 50 | 9.0 |
| net_income | yes | 48 | 49 | 53 | 9.5 |
| net_income_to_common | yes | 37 | 39 | 46 | 8.3 |
| preferred dividends | **no** | 29 | 32 | 36 | 6.6 |
| dividends_paid | yes (all classes, includes preferred) | 44 | 46 | 48 | 8.8 |
| repurchases_common_stock | yes | 33 | 44 | 42 | 8.2 |
| total_assets | yes | 48 | 48 | 53 | 9.5 |
| loans (gross, else net; tags change in 2020) | **no** | 31 | | 39 | |
| allowance for credit losses | **no** | 43 | 44 | 49 | 8.9 |
| deposits | **no** (only in `raw_extras`) | 44 | 44 | 49 | 8.7 |
| equity (parent, includes preferred) | yes | 48 | 49 | 53 | 9.5 |
| preferred equity | **no** | 25 | 30 | 37 | 6.2 |
| goodwill | yes | 46 | 48 | 52 | 9.5 |
| intangibles_net (other than goodwill) | yes | 37 | 40 | 47 | 8.4 |
| shares_outstanding | yes | 29 | 31 | 35 | 6.2 |
| shares_diluted (weighted) | yes | 47 | 49 | 53 | 9.5 |
| dividend per share | **no** | 38 | 41 | 47 | 8.3 |
| CET1 ratio | **no** | 4 | 7 | 10 | 1.6 |
| CET1 capital | **no** | 5 | 9 | 11 | 2.0 |
| Tier 1 ratio | **no** | 11 | 13 | 15 | 2.8 |
| risk-weighted assets | **no** | 2 | 2 | 2 | 0.4 |
| write-offs of the allowance | **no** | 10 | 17 | 13 | 5.8 |
| recoveries | **no** | 4 | 6 | 12 | 3.9 |
| nonaccrual loans | **no** | 15 | 16 | 23 | 5.0 |

Banks with no value in the latest year (`out` of the same script): net interest income AFRM ALLY FAF FRHC SF;
noninterest income AFRM CNH FAF FRHC SCHW SF; `shares_outstanding` 18 banks (AFRM C CBC CBSH CFR EWBC FCNCA FHN FNB KEY
MTB PNC RF SCHW SF UBSI VLY WAL). Most of these are in the groups that the first version does not value.

**Findings on the data.**
1. The first version needs only: net income to common, preferred equity, equity, goodwill, intangibles, dividends,
   buybacks, shares, and for the drivers of the second version the items of the income statement. The registry has the
   inputs for ROE and for book value, with two gaps: preferred equity and the shares.
2. Preferred equity must come from several tags. We take the largest of four tags (`PreferredStockValue` 31 banks,
   `PreferredStockIncludingAdditionalPaidInCapitalNetOfDiscount` 4, `PreferredStockValueOutstanding` 4,
   `PreferredStockIncludingAdditionalPaidInCapital` 1). JPM has $20.0B of preferred stock under the second tag only.
   A bank with no tag has no preferred stock.
3. `shares_outstanding` is missing for 18 banks. The balance-sheet tag `CommonStockSharesOutstanding` is missing; the cover
   page `dei/EntityCommonStockSharesOutstanding` exists for 51 banks (it is dated some days after the year end, and a
   bank with two classes has no total value). The first calculation (A5) used the balance-sheet count at the date of the
   equity where it exists, else the cover count.
4. CET1 and the regulatory ratios are not usable: 10 to 15 banks have them for the whole company, because the
   facts are often tagged with a dimension (company or bank) that the company-facts file does not carry. The first
   version does not need them.

### Tags to add to the registry (a list; the registry is not changed in this study)

"Banks, latest" is the number of the 53 banks with the tag in the latest fiscal year. "Banks, >= 8 y" is the number with at
least 8 of the last 10 years (`out/bank_tagcount.txt`). A tag with few years but many banks in the latest year is the
new tag after the CECL change (2020): the model needs the chain.

| New variable | Tags (in this order; the first found for each year) | Banks, latest | Banks, >= 8 y |
|---|---|---|---|
| `noninterest_expense` | `us-gaap/NoninterestExpense` | 49 | 46 |
| `provision_credit_losses` | `ProvisionForLoanLeaseAndOtherLosses` (24/21), `ProvisionForLoanAndLeaseLosses` (0/5), `ProvisionForLoanLossesExpensed` (16/10), `FinancingReceivableExcludingAccruedInterestCreditLossExpenseReversal` (31/0) | 50 for the chain | 46 for the chain |
| `loans_gross` | `LoansAndLeasesReceivableGrossCarryingAmount` (1/7), `FinancingReceivableExcludingAccruedInterestBeforeAllowanceForCreditLoss` (36/0), `LoansAndLeasesReceivableNetOfDeferredIncome` (0/8); else `loans_net` | 37 | 38 |
| `loans_net` | `LoansAndLeasesReceivableNetReportedAmount` (1/11), `FinancingReceivableExcludingAccruedInterestAfterAllowanceForCreditLoss` (35/0) | 36 | 34 |
| `allowance_credit_losses` | `FinancingReceivableAllowanceForCreditLossExcludingAccruedInterest` (39/9), `FinancingReceivableAllowanceForCreditLosses` (11/6), `LoansAndLeasesReceivableAllowance` (0/8) | 49 | 44 |
| `deposits` | `Deposits` (48/44), `DepositsDomestic` (4/2) | 49 | 44 |
| `preferred_equity` | largest of `PreferredStockValue`, `PreferredStockIncludingAdditionalPaidInCapitalNetOfDiscount`, `PreferredStockValueOutstanding`, `PreferredStockIncludingAdditionalPaidInCapital` | 37 | 30 |
| `preferred_dividends` | `DividendsPreferredStockCash`, `PreferredStockDividendsIncomeStatementImpact`, `DividendsPreferredStock`, `PreferredStockDividendsAndOtherAdjustments` | 36 | 32 |
| `intangibles_ex_goodwill` (already `intangibles_net`; check the chain) | `IntangibleAssetsNetExcludingGoodwill` (32/24), `FiniteLivedIntangibleAssetsNet` (30/26) | 47 | 40 |
| `dividends_per_share` | `CommonStockDividendsPerShareDeclared` (37/30), `CommonStockDividendsPerShareCashPaid` (13/12); unit USD/shares | 47 | 41 |
| `shares_outstanding_cover` | `dei/EntityCommonStockSharesOutstanding` (unit shares) | 51 | |
| `charge_offs`, `recoveries` (second version) | `FinancingReceivableAllowanceForCreditLossesWriteOffs` (13/9), `AllowanceForLoanAndLeaseLossesWriteOffs` (0/3); `FinancingReceivableAllowanceForCreditLossesRecovery` (12/6) | 13 / 12 | 17 / 6 |
| `nonaccrual_loans` (second version) | `FinancingReceivableRecordedInvestmentNonaccrualStatus` | 23 | 16 |
| `cet1_ratio`, `cet1_capital`, `rwa` (do not add yet) | `CommonEquityTierOneCapitalRatio`, `CommonEquityTierOneCapital`, `RiskWeightedAssets` | 10 / 11 / 2 | 7 / 9 / 2 |
| `interest_income_loans`, `interest_expense_deposits` (second version) | `InterestAndFeeIncomeLoansAndLeases`, `InterestExpenseDeposits` | 37 / 45 | 32 / 40 |

The variable `dividends_paid` includes preferred dividends (precedence `PaymentsOfDividends`). The model must subtract
`preferred_dividends`. The scope of each new variable is the template `bank` (the rule of `CLAUDE.md`, point 5).

## A3. The model form

### The identity

Let `B(t)` be common equity at the end of year t, `NI(t)` net income to common, `D(t)` net payout to common holders
(dividends + buybacks - new share issues), `k` the cost of equity. Clean surplus means `B(t) = B(t-1) + NI(t) - D(t)`.
Let `r(t) = NI(t) / B(t-1)` be the ROE. Then the value of the dividends is

```
V(0) = sum_{t=1..inf} D(t) / prod(1+k_s)
     = B(0) + sum_{t=1..inf} (r(t) - k_t) B(t-1) / prod_{s<=t}(1+k_s)
```

(proof: replace `D(t) = NI(t) - B(t) + B(t-1)` and add the telescoping sum of `B`; the terminal term goes to zero if
`B` grows slower than `k`). This is the excess-return model of the source above.

### The path model (the form that fits `paths.rs`)

For path p, year t = 1..N (N = 10, as the automatic model):

```
r(t)  = mu + phi (r(t-1) - mu) + e(t),        e ~ N(0, sigma^2)        the ROE process (the same OU form as the growth process)
pi(t) = net payout ratio: constant = own history shrunk to the cell
B(t)  = B(t-1) (1 + r(t) (1 - pi) + x(t))    x(t): clean-surplus gap, drawn from its history (A4, A6)
k(t)  = rf(t) + beta ERP                      rf(t) from the rate path (pathrate.rs); beta as in auto.rs; ERP 4.41%
V     = B(0) + sum_{t=1..N} (r(t) - k(t)) B(t-1) / prod_{s<=t}(1+k_s)  +  TV / prod_{s<=N}(1+k_s)
```

Parameters drawn once for each path: `mu` (the long-run ROE of the bank: mean `mu_i` and standard deviation from the
estimate of the bank mean), beta, ERP, terminal growth `g`. `mu_i = w m_i + (1-w) m_cell` with `w = tau^2 / (tau^2 + sigma^2 mvf)`
(the rule that `auto.rs` uses for the growth mean).

**Terminal value.** After year N the bank earns `r_inf` and grows at `g` (cap 2.25%, as the automatic model). Then the
payout is `1 - g / r_inf` and

```
TV = B(N) (r_inf - k) / (k - g)
```

Two choices for `r_inf`:
- (a) persistent: `r_inf = mu_i`. Excess return stays for ever. This is the choice of the first calculation (A5).
- (b) competitive fade: `r(t)` moves linearly to `k` between year 1 and year N, `TV = 0`. The value of the excess return
  then ends.
The truth is between (a) and (b). A5 shows both. The model ticket must decide the default (open decision 1). The data
cannot choose: ROE of banks has a half-life below one year around a bank mean (A4), and a 10-year history cannot see a
fade of the mean.

**Tangible form.** The same equations with `B` = tangible common equity and `r` = ROTE. Value of equity is then
`TCE + PV(excess ROTE)`. It treats goodwill as an asset that earns the cost of equity. A5 shows that the two forms
differ by 5% to 20%; the difference is a measure of model risk.

### The alternatives and what each needs

| Alternative | Formula | Needs | Has the mapping? |
|---|---|---|---|
| Dividend discount model | `V = sum D(t) / prod(1+k)`; `D = pi NI`; `NI = r B(t-1)` | the same as above | It is the same model: with clean surplus it gives the same value. It adds nothing but a second view of the payout. |
| Split of ROE into parts | `NI = (NII + fee - cost - provision)(1 - tax)`; `ROE = NI / B`; each part is a yearly process for each unit of assets: `NII/A` (net interest margin on assets), `fee/A`, `cost/(NII+fee)` (efficiency, mean 60%), `provision/loans`, assets / equity | noninterest expense, provision, loans (A2 table) | Not yet. 44 to 50 banks have the new tags for the last 10 years. The rate path would drive `NII/A`. This is the second version. |
| Price to tangible book from ROTE and growth (`P/TBV = (ROTE - g)/(k - g)`) | a single-step version of the model | ROTE, g | yes; it is a check, not a model |
| Regulatory capital (CET1) as the capital that must be held (distributable capital) | `V = sum (NI - needed growth in CET1 capital)`; the payout is then the free capital | CET1 capital, risk-weighted assets | No. The data are too thin (A2, finding 4). |

## A4. Measured drivers

Scripts: `bpanel.py`, `drivers.py` (`out/bank_ar1.txt`), `drivers2.py` (`out/bank_dist.txt`). The sample is the 35 deposit
banks (33 have at least 2 consecutive pairs), fiscal years 2014 to 2025. A pair is two fiscal years in sequence (350 to
380 days). A ROE outside -50% to +60% (ROTE: +80%) is not used. The estimators are the ones of `calibrate.rs`.

**AR(1) of the ROE** (`x` = ROE of the year before, `y` = ROE of the year; "FE+N" is the fixed-effect estimate with the
Nickell correction; the standard error is cluster-robust by bank; the mean is the long-run mean of the pairs; "sigma" is the
residual standard deviation; "hl" is the half-life in years):

| Group | Variable | Pairs | Banks | Pooled phi (se) | Within phi | FE+N phi (se) | Mean | Sigma | hl |
|---|---|---|---|---|---|---|---|---|---|
| Deposit banks | ROE | 328 | 33 | 0.57 (0.07) | 0.20 | **0.34 (0.07)** | 0.110 | 0.028 | 0.6 |
| Deposit banks | ROE, less year mean | 328 | 33 | 0.64 (0.07) | 0.24 | 0.39 (0.07) | 0 | 0.024 | 0.7 |
| Deposit banks | ROTE | 328 | 33 | 0.44 (0.06) | 0.16 | **0.30 (0.06)** | 0.149 | 0.038 | 0.6 |
| Deposit + custody | ROE | 358 | 36 | 0.57 (0.07) | 0.19 | 0.33 (0.06) | 0.110 | 0.028 | 0.6 |
| Consumer lenders (5) | ROE | 50 | 5 | 0.51 (0.09) | 0.14 | 0.27 (0.20) | 0.182 | 0.075 | 0.5 |
| Brokers and investment banks (6) | ROE | 56 | 6 | 0.08 (0.16) | -0.14 | -0.04 (0.17) | 0.148 | 0.071 | 0 |
| All 49 banks | ROE | 470 | 49 | 0.56 (0.10) | 0.08 | 0.21 (0.14) | 0.120 | 0.045 | 0.4 |

**Reading.** Two parts explain the ROE of a bank: a mean that is specific to the bank (the pooled phi of 0.57 is mostly
this), and a shock that fades in less than one year (FE+N phi of 0.34). The standard deviation of the true bank means is
0.023 (ROE) and 0.022 (ROTE); with 11 years of history the weight of the own mean is 0.77 (ROE) and 0.66 (ROTE)
(`drivers2.py`). The estimate has 328 pairs, 33 banks, and the standard error of phi is 0.07. The company years are
correlated in time (the same year hits all banks): the year means of the ROE of the deposit banks move from 8.7% (2020)
to 14.9% (2023), a standard deviation of 1.9 points, so the effective number of independent shocks is smaller than
328. The estimate is weak for broker, consumer, and all other cells. The ROE of brokers has no persistence, so a model
that starts from a ROE path has no basis for them.

**Distribution of the yearly values, deposit banks** (365 company-years):

| Variable | Mean | P10 | P50 | P90 |
|---|---|---|---|---|
| ROE | 0.111 | 0.068 | 0.106 | 0.154 |
| ROTE | 0.149 | 0.091 | 0.145 | 0.204 |
| Payout (dividends + buybacks) / net income | 0.606 | 0.235 | 0.567 | 1.016 |
| Payout of dividends only | 0.348 | 0.166 | 0.310 | 0.562 |
| Growth of common equity | 0.102 | -0.014 | 0.056 | 0.262 |
| Growth of tangible common equity | 0.103 | -0.020 | 0.080 | 0.238 |
| Clean-surplus gap | 0.053 | -0.040 | 0.011 | 0.183 |
| Growth of assets | 0.102 | -0.006 | 0.051 | 0.262 |
| Net interest income / assets | 0.030 | 0.023 | 0.029 | 0.039 |
| Efficiency (noninterest expense / revenue) | 0.600 | 0.481 | 0.605 | 0.680 |
| Provision / loans | 0.034 | 0.000 | 0.003 | 0.008 |

(The mean of provision / loans is moved by outliers; the median is 0.3%.) The growth of equity is much larger than
`ROE (1 - payout)` (about 4.5% for the median bank). The clean-surplus gap of 1.1% (median) to 5.3% (mean) is the reason:
acquisitions that use shares, and other comprehensive income (2022: equity growth is -0.1% on average). The model must
have the gap as a shock, or it must work on the value for each share. A5 and A6 show the effect.

Year means (deposit banks, `drivers2.py`): ROE 9.2% (2016), 11.9% (2018), 8.7% (2020), 12.6% (2021), 14.9% (2023),
10.6% (2024), 12.0% (2025). Payout 49% (2023) to 79% (2019).

## A5. A first calculation for five banks

Script `valuation5.py` (`out/bank_value5.txt`). One deterministic calculation at the driver means. The order of the work
is fixed before the price is read. Inputs:
- Base: the latest balance sheet in the cache (2026-09-30 for the five). Common equity = equity - preferred;
  tangible = less goodwill less intangibles.
  Shares at the balance sheet date where the tag exists (BAC, WFC), else the cover page count (JPM, USB, PNC).
- ROE process: `phi` = 0.34 (the FE+N value of the deposit cell), long-run mean `mu` = shrunk bank mean (weight 0.77 on
  the own mean of 11 years, 0.23 on the cell mean 10.8%), start value = the ROE of FY2025.
- Payout: 0.5 x the mean of the last 3 years of the bank + 0.5 x the cell mean (0.61), capped at 1.0.
- Cost of equity: `rf` = 10-year zero yield of the GSW file on 2026-11-13 (5.135%) + beta x 4.41%; beta = 2/3 raw + 1/3
  (weekly returns of 5 years against SPY: `market.py`). Terminal growth 2.25%, 10 years, terminal ROE = `mu` (choice (a)).
  The rate is flat (no rate path in this scratch).
- The price is the last close of the price cache (2026-11-18). It is not used in the calculation.

| Bank | BVPS | TBVPS | Close | P/B | `mu` ROE | Beta used | Ke | Value / share, ROE on common equity | Value / share, ROTE on tangible equity | Value / close (common; tangible) |
|---|---|---|---|---|---|---|---|---|---|---|
| JPM | 132.30 | 111.96 | 324.11 | 2.45 | 13.5% | 1.01 | 9.59% | 222.41 | 260.81 | 0.69; 0.80 |
| BAC | 39.11 | 28.88 | 48.91 | 1.25 | 9.4% | 1.07 | 9.86% | 37.01 | 44.13 | 0.76; 0.90 |
| WFC | 52.86 | 44.54 | 80.92 | 1.53 | 10.5% | 0.99 | 9.48% | 60.69 | 67.83 | 0.75; 0.84 |
| USB | 38.26 | 26.36 | 58.28 | 1.52 | 12.8% | 1.04 | 9.70% | 58.98 | 71.22 | 1.01; 1.22 |
| PNC | 157.15 | 123.59 | 221.14 | 1.41 | 9.9% | 1.00 | 9.56% | 166.61 | 196.08 | 0.75; 0.89 |

(`out/bank_value5.txt` has every line of the calculation. The payout differs for each bank: JPM 0.61, BAC 0.66, WFC 0.83,
PNC 0.57, USB 0.55.)

Parts of the JPM value (common-equity form): book 132.30 + PV of excess return in years 1 to 10 42.33 + terminal 47.78.
Value to book is 1.68 for JPM, 0.95 for BAC (below book: the long-run ROE of 9.4% is below the cost of equity of 9.9%),
1.15 WFC, 1.54 USB, 1.06 PNC.

**Sensitivity** (JPM, value per share; rows: long-run ROE; columns: cost of equity):

| Long-run ROE | Ke 8% | 9% | 10% | 11% | 12% |
|---|---|---|---|---|---|
| 8% | 137.9 | 116.6 | 101.0 | 89.0 | 79.5 |
| 10% | 189.2 | 158.9 | 136.5 | 119.5 | 106.0 |
| 12% | 246.4 | 205.7 | 175.9 | 153.2 | 135.3 |
| 14% | 309.9 | 257.7 | 219.4 | 190.3 | 167.4 |

The other four banks are in `out/bank_value5.txt` (for BAC at Ke 10%: 29.1 at ROE 8%, 62.4 at ROE 14%; USB: 28.3 and 64.3;
PNC: 115.7 and 260.5; WFC: 41.1 and 79.9). One point of the cost of equity moves the value by 10% to 20%; two points of the
long-run ROE move it by about 30%. Fade (b), where the whole ROE goes linearly to Ke over 10 years: JPM 153.3 (full
fade) and 186.8 (half fade) against 222.4; BAC 38.7 and 37.9 against 37.0 (BAC has almost no excess return);
WFC 55.2 and 57.9; USB 42.9 and 50.7; PNC 160.3 and 163.4.

**Reading.** For four of the five banks the value is 24% to 31% below the close with the book form, and 9% to 20% below
with the tangible form. USB is at the close (book) or 22% above (tangible). The model gives no rule to prefer one
form; the spread is model risk. This is the same direction as the investigation: with a risk-free rate of 5.1%, a beta
near 1 and a premium of 4.4%, the cost of equity of about 9.6% is higher than what the prices imply. We did not tune
anything to the price. The numbers do not say that the model is right or wrong; they say that the cost of equity is
the largest open input (open decision 2).

## A6. A point-in-time check that is possible now

Script `pit_bank.py` (`out/bank_pit.txt`). For each origin year y from 2018 to 2024, the cell (35 deposit banks) is
fitted with the data to year y only (the AR(1) with the FE+N estimate, the cell mean, the spread of the bank means, the
cell payout and the history of the clean-surplus gap). For each bank with at least 3 years of own history, 4,000 paths
give a P10 to P90 band for the ROE of years y+1, y+2, y+3 and for the ratio `B(y+h) / B(y)`. The band is compared with the
real value. A bank-year is one test. Target: 80% inside, 10% below, 10% above, 50% below the median.

| Quantity | h | Tests | Origins | Inside P10-P90 | Below P10 | Above P90 | Below median |
|---|---|---|---|---|---|---|---|
| ROE | 1 | 231 | 7 | 0.79 | 0.07 | 0.14 | 0.35 |
| ROE | 2 | 198 | 6 | 0.79 | 0.06 | 0.15 | 0.37 |
| ROE | 3 | 165 | 5 | 0.77 | 0.04 | 0.19 | 0.33 |
| Common equity ratio, with the gap shock | 1 | 231 | 7 | 0.71 | 0.19 | 0.10 | 0.37 |
| Common equity ratio, with the gap shock | 2 | 198 | 6 | 0.71 | 0.19 | 0.10 | 0.44 |
| Common equity ratio, with the gap shock | 3 | 165 | 5 | 0.62 | 0.28 | 0.09 | 0.51 |
| Common equity ratio, retention only (no gap) | 1 / 2 / 3 | 231 / 198 / 165 | | 0.18 / 0.20 / 0.15 | 0.26 / 0.24 / 0.31 | 0.56 / 0.56 / 0.55 | |
| ROTE | 1 / 2 / 3 | 231 / 198 / 165 | | 0.81 / 0.83 / 0.79 | 0.07 / 0.05 / 0.03 | 0.12 / 0.13 / 0.18 | 0.35 / 0.36 / 0.28 |
| Tangible form: equity ratio with the gap shock | 1 / 2 / 3 | | | 0.65 / 0.64 / 0.55 | 0.25 / 0.27 / 0.36 | 0.10 / 0.10 / 0.08 | |

**Reading.**
1. The ROE band works: 77% to 83% inside. The error is one-sided. The real ROE is above the median in 63% to 67% of the
   tests (below median 0.33 to 0.37). The years 2021 to 2025 had a ROE above the earlier years. The model is biased low,
   and the cause is likely a rise of the level of the ROE after 2021 that an AR(1) around a historic bank mean cannot see.
2. The equity band without the gap shock fails (18% inside; 56% above): a model that grows equity with the retention
   only is wrong by a wide margin. With the gap shock it works at 71% for 1 and 2 years and 62% for 3 years, and the
   band is too high on the lower side (19% to 28% below P10): the gap history (acquisition years, 2022 other comprehensive
   income) is not the same in each origin.
3. Limit: 5 to 7 origin years. The tests of one origin year are correlated (a common year). The coverage has no
   independent standard error; treat 0.62 to 0.83 as a range.

---

# Part B: utilities

## B1. The set

The template detection has no utility class. We looked at the raw facts of the two universe lists
(`util_find.py`, `util_tags.py`, `util_rule.py`; `out/util_rule.txt`).

**The tags that separate.** A reference list of 38 utilities and independent producers was made by name (not by the rule).
For each us-gaap tag in the latest fiscal year we counted the tickers in the reference list and not in it (`util_tags.py`).
- `PublicUtilitiesPropertyPlantAndEquipmentNet`, `...PlantInService`, `...AllowanceForFundsUsedDuringConstruction...` and
  `UtilitiesOperatingExpenseMaintenanceAndOperations`: 15 to 22 members, **0 outside the list**.
- `RegulatoryAssetsNoncurrent` 33 members, 2 outside (KMI, ET); `RegulatoryLiabilities` 21 members, 3 outside (DTM KMI WMB):
  pipelines also have regulatory assets. A rule on these tags alone has false members.
- Share of (regulatory assets + regulatory liabilities) in total assets: 31 utilities have 3.6% to 30%; the pipelines WMB
  (3.5%), DTM (1%), KMI (0.7%) and the refiner MPC (0.6%) are at or below 3.5%.

**The rule.** A company is a utility if, in a fiscal year that ends after 2024-06-30, (a) it has any of the four
`PublicUtilities...` / `UtilitiesOperatingExpense...` tags, or (b) (regulatory assets + regulatory liabilities) / total
assets is at least 7%. Result: **36 tickers (29 in `sp500.txt`, 7 in `next500.txt`), no false member** (checked by name
against all hits; the 36 are in the list below). The reference list has two misses: AES (a global producer with two
regulated utilities) and VST (a producer). The 27 utilities that the investigation lists are all found.

| Class | Tickers | Number |
|---|---|---|
| Regulated electric, gas and water utilities (a rate base) | AEE AEP ATO AWK CMS CNP D DTE DUK ED EIX ES ETR EVRG EXC FE IDA LNT NEE NFG NI OGE PCG PEG PNW PPL SO SRE SWX TXNM UGI WEC WTRG XEL | 34 |
| Independent producers with a utility tag (no rate base; regulatory share 0%) | CEG NRG | 2 |
| Missed by the rule, same kind | AES VST (and TLN, not in the reference list) | 2 to 3 |

Notes: UGI is a mixed company (propane and utilities; regulatory share 1.9%). SO has a regulatory share of 0.5% only
(its facts carry the regulatory items with a dimension), but it has the `PublicUtilities` tags. NEE is a regulated
utility (FPL) with a very large renewable producer. The model ticket must choose a rule for the producers: CEG and NRG have
no allowed return, so the excess-return model on book equity with an allowed return does not fit them (open decision 5).
The tests of this study use the 34 regulated utilities.

## B2. Why the free-cash-flow model fails for them

Script `util_fcf.py` (`out/util_fcf.txt`). FCF = operating cash flow - capex, in $B, from the mapping.

| Utility | Years with FCF below 0 (of 10) | 10-year sums: OCF; capex; FCF | Dividends (10 y) | Debt: first to last year | Capex / depreciation (range) |
|---|---|---|---|---|---|
| DUK | 9 | 86; 106; **-20** | 29 | 50.4 to 89.8 | 1.7 to 2.2 |
| ES | 9 | 22; 32; **-10** | 8 | 9.6 to 28.3 | 2.7 to 3.3 |
| FE | 7 | 26; 31; **-5** | 8 | 22.6 to 26.6 | 1.9 to 3.0 |
| ETR (2024 and 2025 only; see the mapping gap below) | 2 | 32; 14 (**wrong**); 18 | 8 | 14.8 to 30.9 | 2.2 to 3.3 (2024, 2025) |
| NEE (2023 to 2025 only) | 3 | | 30 | 30.8 to 95.0 | 3.7 to 4.5 |

DUK: net income of the 10 years is $32B, capex is 3.3 times that, dividends are 90% of it, and debt grows from $50B to
$90B while the PP&E grows from $83B to $130B. NEE: capex is 1.9 to 2.2 times the operating cash flow (-$12B to -$14B each
year). Over all company-years of the 34 regulated utilities, the median capex / operating cash flow is 1.25, and the median
capex / depreciation is 2.5 (`out/util_drivers.txt`). A model that takes the margin of the history, with capex as a
share of revenue, and discounts it at the cost of capital gets a negative value because the investment is paid now and the
return (an allowed return on the new rate base, earned for 30 to 40 years) is far in the future and not in the free cash
flow of the model. The debt is a part of the business model: the PP&E is 53% funded by debt in all years (median
debt / PP&E 0.54, P10 0.37, P90 0.69).

**Mapping gaps that the study found (a defect for the model ticket):**
1. ETR: the registry `capex` gives 0.0 to 0.3 $B in 2017 to 2023 (the real amount is $4B or more). ETR uses
   `PaymentsForCapitalImprovements` (2007 to 2023, $4.44B in 2023) before 2024 and `PaymentsToAcquirePropertyPlantAndEquipment`
   after 2022. With the zero capex its FCF is positive for 8 years: that is wrong.
2. NEE: no capex value for 2016 to 2022 (the filer uses custom tags). The 2023 to 2025 value is $25B a year.
3. `ppe_net` is wrong or missing for ES (no value; `PublicUtilitiesPropertyPlantAndEquipmentNet` $45.9B exists), ETR
   ($0.48B from `PropertyPlantAndEquipmentNet`; the utility plant is $52.9B) and FE (missing after 2023).
   Coverage of a plausible `ppe_net` (at least 30% of assets): 25 of 34 utilities in the latest year
   (`out/util_cov.txt`).

## B3. The model form

Notation as in A3. `B` is common equity (the book value of the rate base after debt, with the rate base equal to book
equity divided by the allowed equity share).

| Candidate | Formula | Variables needed | In the mapping? | Tags to add |
|---|---|---|---|---|
| (i) Dividend discount with the growth of the rate base | `V = D(1) / (k - g)`; `g = ROE (1 - pi)` or the growth of the rate base; or a path model of `D(t) = pi NI(t)`, `NI = r B(t-1)`, `B` from the rate base | dividends per share or dividends and shares, payout, ROE, growth of PP&E | yes (dividends, NI, equity, shares). `g` is not stable (below) | `dividends_per_share` |
| (ii) **Excess return on book equity** (the same engine as for banks) | `V = B(0) + sum (r(t) - k) B(t-1) / prod(1+k) + TV`; the ROE `r` goes to a long-run value (the allowed return); `B(t) = B(t-1)(1 + r(1 - pi_net))`, with `pi_net` = (dividends + buybacks - share issues) / net income; `TV = B(N)(r_inf - k)/(k - g)` | net income to common, equity, dividends, buybacks, share issues, shares | **yes**: all in the registry, with coverage of 31 to 34 utilities in the latest year (`out/util_cov.txt`) | none for the first version |
| (iii) Free cash flow with growth capex separated | `FCFF = revenue (margin - sbc) - maintenance capex - growth capex`; the growth capex adds to the rate base `RB(t) = RB(t-1) + growth capex - (depreciation - maintenance capex)`; the return on `RB` is the allowed return | maintenance capex, growth capex, rate base, allowed ROE and equity share | **no**. XBRL has no split of capex. A proxy is depreciation as the maintenance capex: the growth part of capex is then 47% (DUK), 67% (ES), 61% (FE), 61% (ETR), 47% (SO) of capex (`util_value.py`). The rate base and the allowed return are in no tag. | `PublicUtilitiesPropertyPlantAndEquipmentNet` (22 of 34 in the latest year), `...PlantInService` (16), `ConstructionInProgressGross` (20), `RegulatoryAssets` and `RegulatoryLiabilities` (32), `AllowanceForFundsUsedDuringConstruction` (22), capex chain (A: ETR and ES above) |

**Which candidate the data supports.** (ii). The ROE is the quantity that regulation sets (the allowed return), and the data
show it: the true heterogeneity of the bank means is small (B4: `tau` 0.7 points, so 90% of the weight of a utility goes
to the cell mean of about 10%). The inputs are in the registry now. (i) with a growth rate from the history does not work:
the median dividend growth of the history (2.1% to 6.1%) and the sustainable growth (2.4% to 6.1%) differ for each utility
by up to 4 points, and for ES the Gordon value is 2.6 times the close (`out/util_value5.txt`), because `Ke - g` is
below 2 points. (iii) needs a split that the data does not give. The rate-base form (ii) with the net payout and the
explicit 10 years carries the growth of the rate base in `B(t)`. The model ticket must keep (iii) as a later step.

**Cost of equity for utilities.** The raw betas against SPY are low (DUK 0.20, SO 0.23, FE 0.26, ES 0.40, ETR 0.42). With
2/3 raw + 1/3 they become 0.47 to 0.61 and Ke is 7.2% to 7.8%. With rf 5.1% the excess return `ROE - Ke` is 2 to 3 points
and the value is very sensitive to Ke (B5). The low beta is a measure on 5 years of weekly data with rate changes in it
(open decision 2).

## B4. Measured drivers

Script `util_drivers.py` (`out/util_drivers.txt`). 34 regulated utilities, fiscal years 2014 to 2025, 361 company-years of
ROE (outside -30% to +40% not used).

| Variable | Mean | P10 | P50 | P90 |
|---|---|---|---|---|
| ROE | 0.096 | 0.048 | 0.101 | 0.146 |
| Payout, dividends only | 0.707 | 0.470 | 0.620 | 1.034 |
| Payout, dividends + buybacks | 0.716 | 0.478 | 0.633 | 1.045 |
| Net payout (less share issues), cell median | | | 0.57 | |
| Growth of common equity | 0.076 | -0.009 | 0.056 | 0.164 |
| Growth of PP&E (the larger of `PublicUtilities...Net` and `ppe_net`) | 0.075 | 0.031 | 0.070 | 0.122 |
| Share issues / equity | 0.039 | 0.001 | 0.016 | 0.100 |
| Clean-surplus gap | 0.041 | -0.001 | 0.015 | 0.103 |
| Capex / operating cash flow | 1.34 | 0.77 | 1.25 | 2.03 |
| Capex / depreciation | 2.56 | 1.77 | 2.47 | 3.65 |
| Debt / PP&E | 0.53 | 0.37 | 0.54 | 0.69 |
| Growth of dividend per share | (outliers) | 0.007 | 0.057 | 0.099 |

(The mean of the dividend growth is moved by wrong units in a few tags; use the median.)

**AR(1) of the ROE:**

| Sample | Pairs | Utilities | Pooled phi (se) | Within phi | FE+N phi (se) | Mean | Sigma | hl |
|---|---|---|---|---|---|---|---|---|
| 34 regulated utilities | 323 | 34 | 0.15 (0.08) | 0.07 | **0.20 (0.09)** | 0.102 | 0.058 | 0.4 |
| less year mean | 322 | 34 | 0.15 (0.07) | 0.07 | 0.20 (0.09) | 0.003 | 0.058 | 0.4 |
| without ETR and NEE | 303 | 32 | 0.12 (0.08) | 0.04 | 0.17 (0.10) | 0.100 | 0.058 | 0.4 |

**Reading.** The ROE of a utility is not persistent and it is noisy (sigma 5.8 points: the large items are write-offs,
wildfire costs, impairments, for example PCG, ES, FE). The standard deviation of the true company means is 0.7 points
(`tau`), so the weight of the own mean is 0.10 with 11 years. All utilities go to a common ROE of about 10% (median 10.1%).
This is what an allowed return does. The year means of the ROE: 7.4% (2015), 11.3% (2018), 7.3% (2020), 10.7% (2021),
11.4% (2025). The estimate has 323 pairs; the standard error of phi is 0.09. The noise is large, so the value of a
utility rests on `mu`, `Ke` and the payout, not on a path of ROE.

## B5. A first calculation for five utilities

Script `util_value.py` (`out/util_value5.txt`). Candidate (ii). Inputs: the latest balance sheet that the cache has (the
facts of the 10-Qs end in 2026-03-31 for DUK, FE, ETR, in 2026-06-30 for ES, and in 2025-12-31 for SO). Shares at the
balance sheet date (DUK, ES, FE), else the cover page count (ETR, SO). `phi` 0.20, long-run `mu` = 0.10 x own mean + 0.90 x
cell median (10.1%), start = the ROE of FY2025, payout = 0.5 x own 3-year net payout + 0.5 x the cell median (0.57),
`rf` 5.135%, ERP 4.41%, beta 2/3 raw + 1/3, g 2.25%, 10 years, terminal ROE = `mu`.

| Utility | BVPS | Close | P/B | ROE FY2025 | `mu` | Net payout | Beta (raw; used) | Ke | Value / share | Value / book | Value / close |
|---|---|---|---|---|---|---|---|---|---|---|---|
| DUK | 70.00 | 130.05 | 1.86 | 9.8% | 9.7% | 0.70 | 0.20; 0.47 | 7.20% | 107.84 | 1.54 | 0.83 |
| ES | 43.39 | 71.53 | 1.65 | 11.4% | 9.9% | 0.38 | 0.40; 0.60 | 7.77% | 65.54 | 1.51 | 0.92 |
| FE | 21.88 | 47.69 | 2.18 | 8.2% | 9.9% | 0.75 | 0.26; 0.51 | 7.38% | 32.64 | 1.49 | 0.68 |
| ETR | 37.89 | 108.00 | 2.85 | 11.7% | 9.9% | 0.49 | 0.42; 0.61 | 7.84% | 55.32 | 1.46 | 0.51 |
| SO | 31.95 | 94.41 | 2.96 | 13.1% | 10.2% | 0.57 | 0.23; 0.49 | 7.28% | 54.29 | 1.70 | 0.58 |

All five have a positive value, which the free-cash-flow model does not give. The value is 1.46 to 1.70 times the book
value. The close is 1.7 to 3.0 times the book value; the model is 8% to 49% below the close.

**Sensitivity of the value per share** (Ke from 7% to 11%; long-run ROE from 8% to 12%, Ke as in the table):

| Utility | Ke 7% | 8% | 9% | 10% | 11% | ROE 8% | 9% | 10% | 11% | 12% |
|---|---|---|---|---|---|---|---|---|---|---|
| DUK | 112.4 | 92.1 | 77.9 | 67.4 | 59.4 | 81.5 | 96.2 | 111.6 | 127.7 | 144.4 |
| ES | 78.8 | 62.2 | 50.8 | 42.5 | 36.3 | 46.0 | 56.1 | 67.1 | 79.2 | 92.3 |
| FE | 35.3 | 29.1 | 24.7 | 21.5 | 19.0 | 24.5 | 28.7 | 33.2 | 37.7 | 42.5 |
| ETR | 66.9 | 53.5 | 44.2 | 37.5 | 32.3 | 39.5 | 47.5 | 56.2 | 65.5 | 75.5 |
| SO | 57.9 | 46.7 | 39.0 | 33.3 | 28.9 | 37.3 | 44.6 | 52.4 | 60.7 | 69.5 |

One point of Ke moves the value by 15% to 20%. One point of the long-run ROE moves it by 12% to 15%.

**The Gordon dividend model (candidate i)** with `g = mu (1 - payout)` gives: DUK 102.2, ES 187.6, FE 36.5, ETR 90.1, SO 98.4.
With the median dividend growth of each history: 84.5, 184.5, 23.8, 44.4, 69.7. The spread within one company is up to a factor
of two, and ES is 2.6 times the close. This shows that (i) has no stable input.

The comparison with the close is for information only; no input was set from the close.

---

# The design that I recommend

## Bank model (`T-20261119-evaluator-bank`)

**Scope of the first version.** The template `bank`, cell "deposit banks" (35 tickers) and the cell "custody banks"
(3). The cells "card and consumer lenders" (5) and "brokers and investment banks" (6) wait for step 2 (no value, the reason
named). CNH and FAF go out of the template.

**Model.** Excess return on common equity (section A3), engine as the path engine (`paths.rs`), a new mode `excess_return`
with processes:

| Element | Process | Source (standard variables) | Parameters |
|---|---|---|---|
| ROE `r(t)` | OU: `r(t) = mu + phi (r(t-1) - mu) + e` | `net_income_to_common`, `preferred_dividends`, `equity`, `preferred_equity` | cell: `phi` 0.34, `sigma` 0.028 (FE+N, re-estimated by `fa dcf calibrate`); `mu` per path from the shrunk bank mean |
| Payout `pi` | constant for a path; own 3 years shrunk 50% to the cell | `dividends_paid`, `preferred_dividends`, `repurchases_common_stock` | cell 0.57 to 0.61; cap 1.0 |
| Equity growth `x(t)` | the clean-surplus gap: bootstrap from the cell history | the same | cell P10 -4%, P90 +18% |
| Cost of equity | `rf(t)` from the rate path + beta ERP | `auto.rs` rules | as for the standard model |
| Terminal | choice (a) `r_inf = mu`, (b) fade; `g` = 2.25%, `pi_inf = 1 - g/r_inf` | | open decision 1 |

Output: value of equity, value per share (shares from `shares_outstanding`, else the cover page count, else the weighted
count), P10/P50/P90, and P/B and P/TBV of the P50. The tangible form (ROTE on TCE) is the second view; the
report prints both and the spread between them.

**Inputs and tags.** Add to the registry the variables of the A2 table: `noninterest_expense`, `provision_credit_losses`,
`loans_gross`, `loans_net`, `allowance_credit_losses`, `deposits`, `preferred_equity`, `preferred_dividends`,
`dividends_per_share`, `shares_outstanding_cover`; (for the second version also `charge_offs`, `recoveries`,
`nonaccrual_loans`, `interest_income_loans`, `interest_expense_deposits`). The first version needs
`preferred_equity`, `preferred_dividends` and the cover shares; the others are for the mapping check and the second
version. Do not add CET1.

**Tests.**
1. Fixed answer with all shocks off: a bank with `r = k` has `V = B`; a bank with a constant `r` above `k`, payout
   `1 - g/r`, `N` years and `g`: `V = B (r - g)/(k - g)` (hand calculation).
2. The identity: the same bank with the dividend form and the excess-return form gives the same value (clean surplus).
3. The registry: the new variables on JPM, BAC, WFC, USB, PNC against the filings (the way of `mapcheck crosscheck`).
4. Refusals: preferred equity larger than equity; negative common equity; fewer than 5 years; the cells that are not
   in the first version.
5. The point-in-time test on the banks of the panel: bands of ROE and of `B(t+h)/B(t)`, as A6, with the clustering by year
   reported; `fa dcf pit` gets a `--template bank` option. Target in the ticket: 70% to 90% inside for h = 1 to 3.

**What A6 asks for.** The band of the equity needs the clean-surplus gap shock (without it the coverage is 18%). The
ROE band is biased low in the sample (real ROE above the median in 63% to 67%): the ticket must test a drift of the
cell mean (for example a weight of the last 3 years in the cell mean) and report it without a change of the rule from
the price.

## Utility model (`T-20261120-evaluator-utility`)

**Scope.** The 34 regulated utilities by the rule of B1 (the tags `PublicUtilities...` or regulatory items of at least 7% of
assets), with the regulatory share of at least 1.5% or the `PublicUtilities` tags and a name check. CEG and NRG (and AES,
VST, TLN) stay in `standard` for the first version (open decision 5). A new template `utility` in `filer_template` with
the rule of B1.

**Model.** The same engine, mode `excess_return`, as for banks, with the utility cell:

| Element | Process | Source | Parameters |
|---|---|---|---|
| ROE | OU, `mu` = cell mean with weight 0.10 on the own mean | `net_income_to_common`, `equity`, `preferred_equity` | `phi` 0.20, `sigma` 0.058, `mu` 10.1% (cell median), re-estimated |
| Net payout | constant for a path | `dividends_paid`, `repurchases_common_stock`, `proceeds_stock_issuance` | cell median 0.57 |
| Equity growth | `r (1 - pi_net)` + the clean-surplus gap shock | the same | cell median gap 1.5% |
| Cost of equity | rate path + beta ERP | `auto.rs` | the beta needs a floor test (B3) |
| Terminal | `g` 2.25%, `r_inf = mu` or the fade | | |

Second step: (iii) with the rate base (`PublicUtilitiesPropertyPlantAndEquipmentNet`, plant in service, construction in
progress, regulatory assets and liabilities) when the allowed return and the equity share are known from the filings
(not XBRL tags; a manual input or an extraction).

**Tags to add** (B2 and B3): `ppe_net` alternative `us-gaap/PublicUtilitiesPropertyPlantAndEquipmentNet` (take the larger of it
and `PropertyPlantAndEquipmentNet`); capex chain with `PaymentsToAcquirePropertyPlantAndEquipment` and
`PaymentsForCapitalImprovements` (ETR) and a check "capex below 2% of PP&E is not a capex"; `regulatory_assets`
(`RegulatoryAssets`, else `RegulatoryAssetsNoncurrent` + `RegulatoryAssetsCurrent`), `regulatory_liabilities`
(`RegulatoryLiabilities`, else `RegulatoryLiabilityNoncurrent` + `RegulatoryLiabilityCurrent`), `utility_plant_in_service`,
`construction_in_progress`, `afudc_equity`, `dividends_per_share`. The NEE capex before 2023 has no tag: use
`net_cash_from_investing` as a fallback or mark the years as missing.

**Tests.** The fixed-answer tests of the bank model (same engine). The mapping tests: ETR capex 2017 to 2023 above
$4B, ES `ppe_net` $45.9B. A refusal for a utility with a regulatory share below 1.5% (a producer). The point-in-time
test of the ROE and of the equity growth for the 34 utilities, with the same bands; because `phi` is 0.20, the band is
mostly the cell distribution. The model value above zero for the 27 utilities of the investigation (the acceptance
test of the ticket; the price is not an input).

## Open decisions for the owner

1. **Terminal ROE of a bank and of a utility:** persistent at the own long-run mean (a), or fade to the cost of equity (b),
   or a mix. For JPM the choice moves the value from 222 to 153 to 187. The data cannot choose (A3).
2. **The cost of equity.** rf 5.13%, ERP 4.41%, beta 2/3 raw + 1/3: for banks this gives Ke 9.5% to 9.9%; for utilities 7.2% to
   7.8% (raw betas of 0.2 to 0.4). A value at the means is 31% below the close for four banks. A floor for the beta of
   a utility (for example 0.6), or the use of an industry beta, is a convention to decide. We did not tune it.
3. **Common equity or tangible common equity as the base.** The two forms differ by 5% to 20% (A5). Choose one for the
   headline, show the other.
4. **Cells of banks.** Do the card lenders (5) and the brokers and investment banks (6) get a model in the first version?
   Our advice: no for the brokers (no persistence of ROE, phi -0.04), later for the card lenders (50 pairs).
   Also: the 3 custody banks go with the deposit cell or alone?
5. **Producers** (CEG, NRG, AES, VST, TLN): `standard`, or a separate class, or the utility model with a producer cell.
6. **Template detection.** CNH and FAF are false members of `bank`; the rule for them is a change in `mapcheck`
   (point 5 of `CLAUDE.md`: it needs a build and tests).
7. **Per-share or total equity.** The gap shock (acquisitions with shares) must be in the model; the other way is to model
   the book value of a share. The second way is more exact and needs the share history (18 banks lack `shares_outstanding`).
8. **Source of shares.** Add the cover page count as a fallback (51 of 53 banks).

## What we could not do, and what is weak

- The panel is 11 years of fiscal-year data, 35 deposit banks. All estimates have the common year effect inside. The
  standard errors are cluster-robust by company, not by year.
- The quarterly balance of the first calculation comes from a raw read of the company facts in the scratch script (not
  from `mapcheck`). The facts of the utilities end in 2026-03 or 2026-06 for four of the five.
- The point-in-time check uses the historic data of the cache with the restated values of the latest filing for earlier
  years (the mapping rule): it is not a test with the values that were known on the date.
- The beta, the rate and the premium of the first calculation are the repo rules with a flat rate; the path model
  has the rate path.
- No CET1 or net charge-off data of use. The split of the ROE into margin, fee, cost and loss is not tested.
- BAC's company facts carry the entity name "BofA Finance LLC" (CIK 70858 is Bank of America Corporation; the
  values are those of the parent). No defect found; the name is the current EDGAR name.
