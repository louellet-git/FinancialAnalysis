# ANT research ledger: facts from the final prospectus

Claude models made this ledger. Anthropic makes Claude. Each value has a source that a
person can check.

This ledger gives facts only. It gives no opinion about value. It is a short ledger: it
has the facts that a DCF model needs. It does not have the risk factors, the governance
terms, or the lock-up terms.

## Source document

| Item | Value |
|---|---|
| Registrant | Anthropic, PBC (ticker ANT, CIK 2130357) |
| Form | 424B4 (final prospectus), prospectus date 2026-11-18, filed 2026-11-19 |
| Accession | 0001628280-26-074944 |
| URL | https://www.sec.gov/Archives/edgar/data/2130357/000162828026074944/anthropicpbc-424b4.htm |
| File in the cache | `data/raw/ANT/filings/0001628280-26-074944/anthropicpbc-424b4.htm` (14,400,530 bytes; SHA-256 starts `207ffea1`) |
| Command | `mapcheck fetch --prospectus ANT` |
| Text file | `424b4.txt`, 6,053 lines, SHA-256 starts `81109347`. Copy in the cache: `data/raw/ANT/filings/0001628280-26-074944/424b4.txt` (`data/` is not committed) |

**The standard mapping gives no variables for ANT.** The registration statement and
the prospectus have no XBRL values (`docs/HIDDEN-ASSUMPTIONS.md` #68). A person or an
agent typed each value in this ledger from the prospectus text. The first 10-Q or 10-K
will give XBRL values.

**How the text file was made.** A scratch script changed the HTML to plain text. The
HTML has a fixed page layout. The rules of the text file are:

- One paragraph is one line. A paragraph that continues on the next page is two lines.
- One table row is one line. ` | ` separates the cells. Spacer columns are removed.
- `$`, `%` and parentheses stay with their number. `(1,260)` is a negative number.
- Each page starts with `[page N | printed P]`. The running header and the page-number
  footer are removed. An image gives the line `[image: FILE]`. The text of an image is
  not in the text file (46 images).

**How to read a source.** `L438` is line 438 of `424b4.txt`. The text after the line
number is the row label or an exact quote. Amounts are in USD million, unless the row
gives a different unit. "FY" is the year that ends on December 31. "9M" is the nine
months that end on September 30. The 9M values and the quarterly values are unaudited.

## 1. Offering

| Fact | Value | Unit | Date | Source |
|---|---|---|---|---|
| IPO price | 83.00 | USD for each Class A share | 2026-11-18 | Cover, L9 "The initial public offering price per share is $83.00"; L19 |
| Class A shares that the company sells | 956,351,354 | shares | offering | Cover, L8; The Offering, L332 |
| Class A shares that the selling stockholders sell | 23,648,646 | shares | offering | L8; L333 |
| Total shares in the offering | 980,000,000 | shares | offering | L6 |
| Option of the underwriters (from the company) | 147,000,000 | shares | offering | L26; L334 |
| Gross amount of the offering | 81,340,000,000 | USD | offering | L19 "Initial public offering price" |
| Underwriting discount | 0.56122 for each share; 550,000,000 total | USD | offering | L20. No discount on the option shares (L25) |
| Proceeds to the company, before expenses | 78,840,438,875 | USD | offering | L21 |
| Proceeds to the selling stockholders, before expenses | 1,949,565,525 | USD | offering | L22 |
| Concurrent private placement of Class N shares, at the IPO price | 313,253,007 shares; 25,999,999,581 USD (stated as $26.0 billion) | shares; USD | closes after the offering | L335; L399; L1575. No discount (L335) |
| Buyers in the private placement | Google 10.0, NVIDIA 10.0, AMD 2.5, Broadcom 1.0, Micron 1.0, Microsoft 1.0, TSMC 0.5 | USD billion | closes after the offering | L335 |
| Net proceeds to the company (offering and private placement) | approximately 104.8 (117.0 with the full option) | USD billion | estimate | Use of Proceeds, L1494 |
| Use of proceeds | General corporate purposes: working capital, operating expenses, capital expenditures (compute and infrastructure), personnel | - | - | L1496 |
| Use of proceeds: tax on the RSU Settlement | 4.3 | USD billion | at the IPO | L1497 "tax withholding and remittance obligations of $4.3 billion"; assumed tax rate 50.90% |
| Directed share program | not in the lines that were read | - | - | - |
| Interest of directors and related entities to buy in the offering | up to 305 | USD million | not binding | L29; L347 |
| Dividend | The company paid none and expects none | - | - | L1502 |

## 2. Shares after the offering

All counts are "as of September 30, 2026", after the pro forma changes (L357 to L365).
The company did a 10-for-1 stock split on 2026-09-17 (L4738). All counts are after the
split.

### 2.1 Shares outstanding

| Fact | Value | Unit | Source |
|---|---|---|---|
| Class A after the offering and the private placement | 7,583,103,996 (7,730,103,996 with the full option) | shares | L336; L400 |
| Class N after the offering and the private placement, legal count | 5,636,538,278 | shares | L337; L1522 "5,636,538,278 shares issued" |
| Class N, accounting count (without the shares that a wholly owned subsidiary holds) | 4,789,697,318 | shares | L400; L1522 "4,789,697,318 shares outstanding" |
| Class T, Class F, Class E | 1; 1; 0 (no economic rights for T and F, L384) | shares | L340 to L342 |
| All classes, legal count | 13,219,642,276 (13,366,642,276 with the full option) | shares | L343 |
| Class A and Class N before the new shares (pro forma) | 6,626,752,642 Class A; 5,323,285,271 Class N | shares | L357 |

The steps from the historical count to the count after the offering (table at L383 to
L400):

| Step | Class A | Class N | Source |
|---|---|---|---|
| Historical, September 30, 2026 | 2,168,788,227 | 30,643,060 | L386; L387 |
| Preferred stock converts (853,394,043 preferred shares) | +4,408,209,450 | +4,153,032,598 | L394; L358 |
| Convertible notes convert ($612.7 million of principal, Amazon) | - | +292,768,653 | L395; L360 |
| RSU Settlement (net of 51,578,975 shares withheld for tax) | +49,754,965 | - | L396; L362 to L365 |
| Company sells in the offering | +956,351,354 | - | L398 |
| Private placement | - | +313,253,007 | L399 |
| Total (accounting count) | 7,583,103,996 | 4,789,697,318 | L400 |
| Initial Public Benefit Issuance (legal shares; a wholly owned subsidiary holds them) | - | +846,840,960 | L409; L361; L1533 |

### 2.2 The count excludes these items

| Fact | Value | Unit | Source |
|---|---|---|---|
| Stock options outstanding, weighted-average exercise price $2.91 | 1,774,342,693 | Class A shares | L367 |
| RSUs outstanding that do not settle at the IPO | 1,026,660,010 | Class A shares | L368; L400 |
| RSUs granted after September 30, 2026 | 47,041,287 | Class A shares | L369 |
| Shares issued for acquisitions after September 30, 2026 | 561,963 Class A; 848,882 Class N | shares | L370 |
| Acquisition holdback shares | 1,590,190 | Class A shares | L406 |
| Reserved for the 2026 Equity Incentive Plan | 1,920,580,692, plus up to 3,401,163,438 | Class A shares | L371 |
| Reserved for the Employee Stock Purchase Plan | 128,038,713 | Class A shares | L372 |
| Reserved for the Donation Matching Plan | 933,698,210 (of which 553,441,297 is the Vested Matching Obligation) | Class N shares | L373 |
| Matching Issuance after September 30, 2026 | 27,991,850 | Class N shares | L374 |
| Amazon notes that stay outstanding (rollover notes), shares on conversion | 1,547,676,309 | Class N shares | L404 |
| Automatic increase of the plans each year | yes; Donation Matching Plan: the smaller of 1% of the shares or a board number, 2027 to 2036 | - | L375; L378 |

### 2.3 Fully diluted count of the company (treasury stock method at $83)

The prospectus gives this count itself (table at L383 to L410, row "Total" at L410).

| Column | Value | Source |
|---|---|---|
| Class A common stock | 7,585,256,149 | L410 |
| Class N common stock (with rollover notes 1,547,676,309, Matching Issuance, Vested Matching Obligation 553,441,297, Initial Public Benefit Issuance 846,840,960) | 7,766,496,616 | L410; L404 to L409 |
| Stock options, net: 1,774,342,693 less 62,208,882 by the treasury stock method (exercise proceeds approximately $5.2 billion at $2.91, shares bought at $83.00) | 1,712,133,811 | L390; L403; L410; L413 |
| RSUs: 1,026,660,010 plus 47,041,287 | 1,073,701,297 | L400; L402; L410 |
| Class T and Class F (no economic rights) | 2 | L410 |

The table does not include the shares reserved for future grants (2.2), the underwriter
option, or a tax withholding on the RSUs that stay.

## 3. Income statements

Source: Consolidated Statements of Operations, L4552 to L4569 (the same rows are at
L438 to L455 and L1777 to L1792). USD million.

| Line | FY2023 | FY2024 | FY2025 | 9M2025 | 9M2026 | Source |
|---|---|---|---|---|---|---|
| Revenue | 41 | 386 | 4,585 | 2,545 | 33,242 | L4552 |
| Compute and infrastructure | 433 | 2,522 | 7,326 | 4,381 | 18,808 | L440 |
| Personnel | 95 | 454 | 1,839 | 1,027 | 4,863 | L441 |
| Bartz settlement | - | - | 1,500 | 1,500 | - | L442 |
| Sales, marketing, and partnerships | 2 | 43 | 568 | 322 | 5,529 | L443 |
| Other operating and administrative | 62 | 347 | 1,412 | 539 | 5,302 | L444 |
| Total operating expenses | 592 | 3,366 | 12,645 | 7,769 | 34,502 | L445 |
| Operating loss | (551) | (2,980) | (8,060) | (5,224) | (1,260) | L4560 |
| Interest income | 69 | 106 | 431 | 239 | 2,359 | L447 |
| Change in fair value of convertible notes, derivative liabilities, and SAFEs | (1,045) | (5,434) | (34,308) | (20,107) | (94,324) | L448 |
| Other income (expense), net | (2) | - | (8) | (3) | (10,397) | L449 |
| Loss before provision for income taxes | (1,529) | (8,308) | (41,945) | (25,095) | (103,622) | L451 |
| Provision for income taxes | 2 | 1 | 20 | 12 | 417 | L452 |
| Net loss | (1,531) | (8,309) | (41,965) | (25,107) | (104,039) | L4567 |
| Net loss per share, basic and diluted (USD) | (1.42) | (5.43) | (23.04) | (13.90) | (50.10) | L4568 |
| Weighted-average shares, basic and diluted | 1,076,880,510 | 1,528,994,540 | 1,821,084,860 | 1,806,802,920 | 2,076,810,122 | L455 |

The income statement has no cost of revenue line and no gross profit line. The company
changed the presentation in 2025 (L4733). A note gives the operating expenses by
function (L5415 to L5423):

| Line | FY2023 | FY2024 | FY2025 | 9M2025 | 9M2026 | Source |
|---|---|---|---|---|---|---|
| Cost of revenue | 26 | 752 | 2,773 | 1,796 | 4,812 | L5418 |
| Research and development | 422 | 2,275 | 6,338 | 3,594 | 19,240 | L5419 |
| Sales and marketing | 76 | 163 | 911 | 530 | 6,619 | L5420 |
| Bartz settlement | - | - | 1,500 | 1,500 | - | L5421 |
| General and administrative | 68 | 176 | 1,123 | 349 | 3,831 | L5422 |

- Cost of revenue is the compute for customer workloads (L5426). Research and
  development has the compute for model development, research and internal use (L5427).
- Other income (expense), net of 9M2026: the fair value of assets and liabilities
  under partner financing arrangements (L1897 to L1900).

### 3.1 Quarters (unaudited), L2129 to L2145

| Line | Q1 2025 | Q2 2025 | Q3 2025 | Q4 2025 | Q1 2026 | Q2 2026 | Q3 2026 | Source |
|---|---|---|---|---|---|---|---|---|
| Revenue | 376 | 787 | 1,382 | 2,040 | 4,731 | 11,555 | 16,956 | L2130 |
| Compute and infrastructure | 944 | 1,390 | 2,047 | 2,945 | 3,387 | 5,558 | 9,863 | L2132 |
| Personnel | 218 | 320 | 489 | 812 | 1,295 | 1,857 | 1,711 | L2133 |
| Bartz settlement | - | - | 1,500 | - | - | - | - | L2134 |
| Sales, marketing, and partnerships | 24 | 96 | 202 | 246 | 1,033 | 2,135 | 2,361 | L2135 |
| Other operating and administrative | 125 | 181 | 233 | 873 | 683 | 1,031 | 3,588 | L2136 |
| Operating income (loss) | (935) | (1,200) | (3,089) | (2,836) | (1,667) | 974 | (567) | L2138 |
| Interest income | 51 | 64 | 124 | 192 | 349 | 762 | 1,248 | L2139 |
| Net loss | (6,592) | (6,377) | (12,138) | (16,858) | (14,439) | (66,309) | (23,291) | L2145 |
| Stock-based compensation | 105 | 181 | 305 | 574 | 924 | 1,261 | 1,070 | L2210 |
| Non-cash charitable contributions | - | - | - | 541 | 126 | - | 1,960 | L2211 |
| Adjusted Operating Income (Loss) | (830) | (1,019) | (1,284) | (1,721) | (617) | 2,235 | 2,463 | L2213 |
| Adjusted Operating Margin | (221)% | (129)% | (93)% | (84)% | (13)% | 19% | 15% | L2216 |

- Q3 2026 has $5.0 billion of compute under short-term commitments and $2.0 billion of
  non-cash donation matching charges (L1647). 9M2026 has $6.5 billion of compute under
  short-term commitments (L1841).
- Price reductions decreased revenue by approximately $600 million from Q2 2026 to
  Q3 2026, and by approximately $70 million from Q3 2025 to Q4 2025 (L2177).
- Revenue of Q4 2024: $197 million (L856).

### 3.2 Results for a period after September 30, 2026

The prospectus has no income statement and no preliminary results for a later period.
It gives one later number: Run-Rate Revenue "was approximately $81 billion as of
October 31, 2026, up $10 billion from September 30, 2026" (L2683).

### 3.3 Non-GAAP measures of the company, L2086 to L2115

| Line | FY2023 | FY2024 | FY2025 | 9M2025 | 9M2026 | Source |
|---|---|---|---|---|---|---|
| Stock-based compensation expense | 29 | 192 | 1,165 | 591 | 3,255 | L2089 |
| Non-cash charitable contributions | - | - | 541 | - | 2,086 | L2090 |
| Adjusted Operating Income (Loss) | (522) | (2,788) | (4,854) | (3,133) | 4,081 | L2092 |
| Adjusted Operating Margin | (1,273)% | (722)% | (106)% | (123)% | 12% | L2095 |
| Change in fair value of certain mark-to-market financial instruments | - | - | 3 | - | 10,277 | L2113 |
| Adjusted Net Income (Loss) | (457) | (2,683) | (4,455) | (2,911) | 5,591 | L2115 |

Definition (L2083): Adjusted Operating Income (Loss) is the operating loss without
(i) stock-based compensation expense, (ii) non-cash charitable contributions, and
(iii) the Bartz settlement. Adjusted Net Income (Loss) also removes the fair value
changes and the related tax (L2101). The company does not define free cash flow.

## 4. Revenue detail and metrics

| Fact | FY2023 | FY2024 | FY2025 | 9M2025 | 9M2026 | Source |
|---|---|---|---|---|---|---|
| Consumption-based revenue | 39 | 288 | 3,796 | 2,125 | 26,306 | L4963 |
| Subscription-based revenue | 2 | 98 | 789 | 420 | 6,936 | L4964 |
| Revenue, United States (billing address) | 34 | 263 | 3,362 | 1,810 | 23,470 | L4954 |
| Revenue, International | 7 | 123 | 1,223 | 735 | 9,772 | L4955 |

| Fact | Value | Date or period | Source |
|---|---|---|---|
| Run-Rate Revenue | approximately $81 billion | 2026-10-31 | L2683 |
| Run-Rate Revenue | approximately $71 billion (the ledger calculates 81 less 10) | 2026-09-30 | L2683 |
| Definition of Run-Rate Revenue | consumption fees of the last 28 days times 13, plus monthly subscription fees times 12 | - | L156 |
| Net Revenue Retention | approximately 900% | 2026-09-30 | L1626; definition L142 |
| Customers with $100,000 or more of revenue (trailing 12 months) | about 20; about 200; about 1,500; more than 11,900 | end of 2023; 2024; 2025; 2026-09-30 | L1715; L1624 |
| Revenue from these customers, share of total revenue | 90%; 57%; 76%; 74% | FY2023; FY2024; FY2025; 12 months to 2026-09-30 | L1715 |
| Customers with $1 million or more; with $10 million or more | more than 1,900; approximately 250 | 2026-09-30 | L1716 |
| Customers below $100,000 | approximately 40 million customers, 26% of revenue | 12 months to 2026-09-30 | L1715 |
| Largest customer | No customer was more than 10% of revenue | 9M2026 | L1718 |
| Revenue outside the top five customers | approximately 79% (64% in Q1 2026) | Q3 2026 | L1718 |
| Fees to channel partners (in sales, marketing, and partnerships) | increase of $4.8 billion against 9M2025 | 9M2026 | L1866 |
| Remaining performance obligations | $13.8 billion; approximately 75% in the next 12 months | 2026-09-30 | L4983 |
| Deferred revenue | 28; 269; 1,971 | end of 2024; 2025; 2026-09-30 | L4515 |
| Employees | approximately 940; approximately 5,300 | 2024-12-31; 2026-09-30 | L856 |
| Seasonal effect | Customer spend decreased by approximately 20% in the two weeks to 2026-01-02 against the two weeks before | - | L1721 |

Revenue by product (API, Claude Code, each application) and by cloud partner: not in
the lines that were read. The prospectus gives the split by consumption and
subscription only.

## 5. Stock-based compensation, depreciation, cash flow

Source: Consolidated Statements of Cash Flows, L4678 to L4721. USD million.

| Line | FY2023 | FY2024 | FY2025 | 9M2025 | 9M2026 | Source |
|---|---|---|---|---|---|---|
| Stock-based compensation expense | 29 | 192 | 1,158 | 584 | 3,255 | L4680 |
| Depreciation and amortization | 3 | 21 | 40 | 24 | 250 | L4681 |
| Donation matching program liability (non-cash) | - | - | 541 | - | 2,086 | L4683 |
| Change in accounts receivable | (10) | (90) | (834) | (500) | (7,076) | L4687 |
| Change in prepaid expenses and other assets | (1,253) | (3,190) | 1,771 | 779 | 1,435 | L4688 |
| Net cash from operating activities | (1,627) | (5,692) | (2,637) | (2,024) | 6,569 | L4692 |
| Purchases of property and equipment | (2) | (15) | (131) | (111) | (925) | L4694 |
| Prepayments for leased assets and equipment | - | - | (54) | (51) | (1,955) | L4695 |
| Purchases of short-term investments | - | - | (3,082) | (1,831) | (40,318) | L4696 |
| Net cash from investing activities | (2) | (22) | (3,077) | (2,007) | (36,546) | L4699 |
| Proceeds from preferred stock | 445 | 1,188 | 16,548 | 16,548 | 110,432 | L4701 |
| Principal payments for finance leases | - | - | - | - | (88) | L4702 |
| Convertible notes issued for cash | 1,750 | 6,050 | 3,450 | 2,050 | - | L4703 |
| Net cash from financing activities | 2,320 | 7,248 | 19,960 | 18,508 | 110,735 | L4708 |

Stock-based compensation by expense line: the footnote says that personnel expense
includes it (L1795). A split by function is not in the lines that were read.

## 6. Balance sheet at September 30, 2026

### 6.1 Actual (unaudited), L4500 to L4520. Earlier dates for comparison.

| Line | 2024-12-31 | 2025-12-31 | 2026-09-30 | Source |
|---|---|---|---|---|
| Cash and cash equivalents | 3,141 | 17,388 | 95,562 | L4500 |
| Short-term investments | - | 2,889 | 36,283 | L4501 |
| Restricted cash | 13 | 13 | 2,540 | L4505 |
| Property and equipment, net | 19 | 146 | 3,061 | L4506 |
| Total assets | 7,752 | 24,661 | 154,320 | L4510 |
| Operating lease liabilities, current and noncurrent | 13 + 39 | 37 + 80 | 274 + 2,536 | L4514; L4517 |
| Convertible notes payable (at fair value) | 14,116 | 41,677 | 130,502 | L4518 |
| Other noncurrent liabilities | 150 | 823 | 1,974 | L4519 |
| Total liabilities | 14,679 | 45,397 | 146,683 | L4520 |
| Redeemable convertible preferred stock | 2,874 | 29,769 | 157,510 | L4611; L4621; L4665 |
| Total stockholders' deficit | (9,801) | (50,505) | (149,873) | L4539 |

### 6.2 Actual, pro forma, and pro forma as adjusted, L1516 to L1530 and L485 to L492

| Line | Actual | Pro forma | Pro forma as adjusted | Source |
|---|---|---|---|---|
| Cash, cash equivalents, and short-term investments | 131,845 | 131,845 | 232,351 | L1516 |
| Convertible notes payable | 130,502 | 109,742 | 109,742 | L1517 |
| Redeemable convertible preferred stock | 157,510 | - | - | L1518 |
| Total assets | 154,320 | 154,320 | 254,800 | L487 |
| Total liabilities | 146,683 | 130,204 | 125,913 | L488 |
| Additional paid-in capital | 6,481 | 198,759 | 303,530 | L1526 |
| Accumulated deficit | (156,193) | (174,482) | (174,482) | L1528 |
| Total stockholders' equity (deficit) | (149,873) | 24,116 | 128,887 | L1529 |

With the full underwriter option, cash and investments are $244.6 billion and total
equity is $141.1 billion (L1534).

### 6.3 What converts at the IPO, and what stays

| Fact | Value | Source |
|---|---|---|
| All preferred stock converts to Class A and Class N common stock | 853,394,043 preferred shares become 8,561,242,048 common shares | L389; L358; L472 |
| Amazon notes that convert at the IPO | $612.7 million of principal becomes 292,768,653 Class N shares | L360 |
| Amazon notes outstanding before the IPO (principal, no interest) | $6,203 million: $2,186 million at $2.0928 for each share, $4,017 million at $5.0478 | L2275 to L2277 |
| Amazon notes that stay (rollover notes) | 1,547,676,309 Class N shares on conversion. Amazon has a 12% ownership limit and did not waive it. The notes convert at the end of each fiscal quarter, up to the limit | L404; L2269; L2280 |
| Maturity of the Amazon notes | March 2034; January, May and November 2035 | L2282; L2283 |
| Accumulated deficit, part from fair value changes of the notes | $135.1 billion of $156.2 billion | L2244 |
| Revolving credit facility | $15.0 billion, matures 2031-09-04; $14.8 billion was available on 2026-09-30 | L2258; L1656 |
| Note purchase agreements not drawn | $77.0 billion: Google 20.0 (becomes 10.0 at the private placement), Amazon 15.0, Broadcom 42.0 (9.00% each year, paid in kind) | L1656; L2288 to L2291; L2294; L2306; L2320 |
| Liquidity before the IPO | $146.6 billion: cash 95.6, short-term investments 36.3, credit facility 14.8 | L1656 |
| SAFEs | The balance sheets of 2024 to 2026 have no SAFE line. SAFEs became preferred stock in 2023 ($1 million) and in 2024 ($134 million) | L4513 to L4520; L4600; L4607 |

## 7. Commitments

Contractual obligations as of September 30, 2026 (L2380 to L2384). USD million.

| Line | Total | 2026 (3 months) | 2027 | 2028 | 2029 | 2030 | After 2030 | Source |
|---|---|---|---|---|---|---|---|---|
| Operating leases | 4,224 | 57 | 303 | 462 | 445 | 466 | 2,491 | L2381 |
| Finance leases | 1,950 | 121 | 438 | 422 | 406 | 390 | 173 | L2382 |
| Purchase commitments | 313,819 | 5,053 | 28,424 | 43,473 | 46,901 | 44,214 | 145,754 | L2383 |
| Total | 319,993 | 5,231 | 29,165 | 44,357 | 47,752 | 45,070 | 148,418 | L2384 |

The table is a part of the commitments. The prospectus gives these other parts:

| Fact | Value (USD billion) | Source |
|---|---|---|
| Total committed spend, as of and after September 30, 2026 | approximately 900.9 | L1655; L2374 |
| Of this, before December 31, 2030 | approximately 451.6 | L1655 |
| Of this, in the 12 months after September 30, 2026 | approximately 70.7 (4.2 has conditions) | L1655; L2397 |
| The remainder | "substantially all" between 2031 and 2045 | L1655 |
| Short-term commitments not in the table | approximately 17.4 | L2394 |
| Leases signed that did not start (start 2026 to 2028, average term about 10 years) | approximately 451.2 | L2395 |
| Commitments made after September 30, 2026 | approximately 105.7, and 6.6 short-term | L2396; L6049 |
| Arrangements with no fixed dollar amount (not in the 900.9) | estimate 150 to 175 | L2374 |
| Microsoft agreement, minimum spend | 47.2 to 49.2, November 2026 to June 2033 | L2401 |
| Google hosting agreement, minimum spend | approximately 121.3, April 2026 to July 2033 | L2402; L2405 |
| Amazon agreement, minimum spend | 110.0, May 2026 to April 2036 | L2406 |
| Purchase commitments as of December 31, 2025, first years | USD million: 2026: 7,059; 2027: 11,128; 2028: 10,832; 2029: 8,304 | L5931 to L5934 |

## 8. Tax

| Fact | Value | Date | Source |
|---|---|---|---|
| US federal net operating loss carryforwards | $8.2 billion, no expiry | 2025-12-31 | L5836 |
| State net operating loss carryforwards | $4.2 billion, start to expire in 2033 | 2025-12-31 | L5836 |
| Ireland net operating loss carryforwards | $85 million, no expiry | 2025-12-31 | L5836 |
| Valuation allowance | 200; 1,206; 3,422 (USD million) | end of 2023; 2024; 2025 | L5834 |
| Valuation allowance is full | "Given the Company's full valuation allowance" | 2025-12-31 | L5855 |
| Provision for income taxes | 417 (USD million); state and foreign taxes | 9M2026 | L452; L1909 |

Net operating loss carryforwards as of September 30, 2026: not in the lines that were
read.

## 9. Forward statements with a number

| Statement | Value | Source |
|---|---|---|
| Stock-based compensation expense that the company records at the completion of the offering (RSUs) | approximately $14.7 billion | L2443; L1508 |
| Unrecognized stock-based compensation for RSUs, September 30, 2026 | approximately $32.3 billion, of which $8.2 billion has the service condition satisfied or partially satisfied | L2443 |
| Unrecognized expense not in the pro forma loss per share | $24.1 billion for RSUs, over approximately 2.0 years; $2.1 billion for other awards, over approximately 1.4 years | L480 |
| Unrecognized cost of stock options | approximately $1.8 billion, over 1.2 years | L2438 |
| RSUs granted after September 30, 2026 (approximately 47 million) | $3.7 billion, over approximately 3.8 years | L6036 |
| Change of the vesting date of approximately 67 million RSUs | approximately $3.4 billion more expense | L6041 |
| Tax payment for the RSU Settlement | $4.3 billion | L1497 |
| Committed spend | See section 7 | L1655 |
| Pro forma net loss per share (USD) | (3.01) for FY2025; (8.78) for 9M2026, on 10,675,104,082 and 10,934,495,542 shares | L476; L475 |
| Revenue growth rate | "We expect our revenue growth rate to decline over time" (no number) | L1666 |
| Operating margin | The company expects more short-term compute "in the near term", and that this decreases the operating margin (no number) | L1617 |
| Capital expenditures | "We expect our capital expenditures to increase significantly"; net cash used in operating and investing activities "for the foreseeable future" (no number) | L2249 |
| Revenue or margin guidance for a future period | not in the lines that were read | - |

## 10. Cross-checks (the ledger calculates these)

All inputs are rows of this ledger. USD million, unless a row gives a different unit.

**Sums.**

- Quarters of 2025: 376 + 787 + 1,382 + 2,040 = 4,585. Equal to FY2025 revenue.
- Quarters of 2026: 4,731 + 11,555 + 16,956 = 33,242. Equal to 9M2026 revenue.
- Revenue of the 12 months to 2026-09-30: 4,585 - 2,545 + 33,242 = 35,282.
- Q3 2026 revenue times 4: 16,956 x 4 = 67,824. Run-Rate Revenue at the same date:
  approximately 71,000.
- Adjusted Operating Income, 9M2026: (1,260) + 3,255 + 2,086 = 4,081. Equal to L2092.
- Commitments (USD billion): 319.993 + 17.4 + 451.2 + 105.7 + 6.6 = 900.9. Equal to
  L1655.

**Growth of revenue.**

- FY2024: 386 / 41 - 1 = 841%. FY2025: 4,585 / 386 - 1 = 1,088%.
- 9M2026 against 9M2025: 33,242 / 2,545 - 1 = 1,206%.
- Q3 2026 against Q3 2025: 16,956 / 1,382 - 1 = 1,127%.
- Q3 2026 against Q2 2026: 16,956 / 11,555 - 1 = 47%. Q2 against Q1 2026:
  11,555 / 4,731 - 1 = 144%.

**Margins.**

| Margin | FY2023 | FY2024 | FY2025 | 9M2025 | 9M2026 |
|---|---|---|---|---|---|
| Gross margin, (revenue - cost of revenue by function) / revenue | 36.6% | (94.8)% | 39.5% | 29.4% | 85.5% |
| Compute and infrastructure / revenue | 1,056% | 653% | 160% | 172% | 57% |
| Operating margin, operating loss / revenue | (1,344)% | (772)% | (176)% | (205)% | (3.8)% |
| Stock-based compensation / revenue | 71% | 50% | 25% | 23% | 9.8% |

- Example: 9M2026 gross margin = (33,242 - 4,812) / 33,242 = 85.5%.
- The operating loss does not contain the fair value changes of the notes. They are
  below the operating loss.

**Cash flow (the company does not define free cash flow).**

| Calculation | FY2023 | FY2024 | FY2025 | 9M2025 | 9M2026 |
|---|---|---|---|---|---|
| Operating cash flow - purchases of property and equipment | (1,629) | (5,707) | (2,768) | (2,135) | 5,644 |
| The same, also less prepayments for leased assets and equipment | (1,629) | (5,707) | (2,822) | (2,186) | 3,689 |

- Example: 9M2026 = 6,569 - 925 = 5,644; 5,644 - 1,955 = 3,689.

**Proceeds and cash.**

- Company shares x price: 956,351,354 x 83.00 = 79,377,162,382 USD (equal to L1574).
- Discount on all 980,000,000 shares: 980,000,000 x 0.56122 = 549,995,600 USD
  (L20 gives 550,000,000).
- Proceeds before expenses + private placement: 78,840 + 26,000 = 104,840. The
  prospectus gives approximately 104,800 after expenses.
- Pro forma as adjusted cash - actual cash: 232,351 - 131,845 = 100,506. Net proceeds
  104,800 - RSU tax 4,300 = 100,500.
- RSU tax: 51,578,975 shares x 83.00 = 4,281 (L1497 gives $4.3 billion).

**Share counts.**

- Legal count: 7,583,103,996 + 5,636,538,278 + 2 = 13,219,642,276 (equal to L343).
- Accounting count: 7,583,103,996 + 4,789,697,318 = 12,372,801,314.
- Treasury stock method: 1,774,342,693 x 2.91 = 5,163 (USD million); 5,163 / 83.00 =
  62,208,882 shares; 1,774,342,693 - 62,208,882 = 1,712,133,811 (equal to L410).
- Fully diluted count of the company: 7,585,256,149 + 7,766,496,616 + 1,712,133,811 +
  1,073,701,297 = 18,137,587,873 (Class T and Class F not included).

**Market capitalization at $83.00.**

| Count | Shares | x 83.00 (USD billion) |
|---|---|---|
| Accounting count (without the 846,840,960 shares that the subsidiary holds) | 12,372,801,314 | 1,026.9 |
| Legal count (L343) | 13,219,642,276 | 1,097.2 |
| Fully diluted count of the company (L410) | 18,137,587,873 | 1,505.4 |
| Underwriter option, if exercised | +147,000,000 | +12.2 |

Parts of the fully diluted count at $83.00 (USD billion): rollover notes
1,547,676,309 shares = 128.5; Initial Public Benefit Issuance 846,840,960 shares =
70.3; Vested Matching Obligation 553,441,297 shares = 45.9.

## 11. Points that a model must not count two times

- The rollover notes are in the balance sheet as "convertible notes payable" (109,742
  pro forma, at fair value) and they are also 1,547,676,309 shares in the fully diluted
  count. The principal is $5.6 billion (6,203 - 613). Use the shares or the liability,
  not the two.
- Stock-based compensation has three values for FY2025: 1,165 (L2089), 1,158 (L4680),
  and "$1.2 billion" (L1795). 9M2025 has 591 (L2089) and 584 (L4680). The prospectus
  lines that were read do not explain the difference of 7.
- The donation matching charge (541 in FY2025; 2,086 in 9M2026) is a non-cash expense
  that the company settles in Class N shares (L2098). It is not in "stock-based
  compensation expense".
- Commitments after September 30, 2026: L2396 gives $105.7 billion and $6.6 billion;
  L6049 gives "approximately $106 billion" and "approximately $7 billion". These are
  the same items with different rounding.

## 12. Limits of this ledger

- The ledger is a short version. A person or an agent read only the sections that the
  sources name. "Not in the lines that were read" does not mean that the prospectus
  does not have the fact.
- No skeptic, reviewer or auditor examined this ledger. Do not use it alone for a
  trade.
- The 46 images can contain numbers (charts). The text file does not have them.
