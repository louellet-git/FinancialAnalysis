# Sources and calculation for the page "Uranium Price Needed: NexGen" (2026-11-25)

Ticket `T-20261125-uranium-needed-price`. The page: `uranium_needed_price_2026-11-25.md`. Read only: no code, no model.
How the numbers were read: one research agent read the documents on the company sites and on SEC EDGAR (30 tool
calls). A second agent read the NexGen filing again, and its numbers are the same. This head did the calculation two
times (by hand and with a one-line script). Not audited when this file was written.

## 1. NexGen Energy (NXE)

Document A: Management's Discussion and Analysis for the three and nine months ended 2026-09-30, prepared 2026-11-09.
`https://www.sec.gov/Archives/edgar/data/1698535/000106299326005850/exhibit99-2.htm`. Amounts in thousands of Canadian
dollars, except as noted. The HTML has no page numbers; the section names are given.

| Item | Value | Where in document A |
|---|---|---|
| NPV of Rook I at US$30, 40, 50, 60, 70, 80, 90, 100 a pound, "Interim Trend Update (Q4 2023 Dollars)", C$ billion | 0.23, 1.19, 2.10, 3.04, 3.96, 4.89, 5.84, 6.79 | Section "Sensitivity of NPV and IRR to Uranium Price" |
| The same, "Feasibility Study (Q4 2020 Dollars)", C$ billion | 1.59, 2.53, 3.47, 4.40, 5.33, 6.27, 7.20, 8.13 | The same section |
| Discount rate; after tax | 8% (footnote 1); footnote 3: "positive after-tax NPV (8%) is maintained across the range" | The same section |
| Exchange rate of the table | "C$/US$ of 1.00:0.75" (1.3333 Canadian dollars for one US dollar) | The same section, footnote |
| Cash | 567,172 | Balance sheet; text "$567.2 million of cash on hand as at September 30, 2026" |
| Short-term investments | 254,398 | Balance sheet; text "$254.4 million of short-term investments" |
| Uranium held | "2.7 million lbs of U3O8 at a cost of $341.2 million"; balance sheet line "Strategic inventory 341,150" | Text and balance sheet |
| IsoEnergy | "approximately 27.8%"; balance sheet line "Investment in associate 181,010" | Text and balance sheet |
| Debentures of 2024 | "As at November 9, 2026, US$250 million of the principal of the 2024 Debentures remain outstanding"; conversion price US$10.73 | Text |
| Debentures of 2023 | US$110 million; US$40 million converted on 2026-09-15, US$70 million on 2026-10-22; "no longer outstanding" on 2026-11-09 | Text |
| The debt on 2026-09-30 | "the aggregate US$320 million principal amount of the Debentures" | Text (two places). It is US$250 million plus the US$70 million of the 2023 debentures that were converted on 2026-10-22: the debt at the date of the cash. The share count of 2026-11-09 has those new shares. US$250 million agrees with the share date; US$320 million with the cash date (found by the auditor) |
| Shares | "As at November 9, 2026, there were 688,034,761 Shares" | Text |
| Project status | Licence to prepare the site and construct: 2026-03-05; "Licensed construction activities commenced on June 8, 2026" | Text. First production date: not found |

Not opened: the technical report of the feasibility study (dated 2021-03-10) and its own table. The release of
2024-08-01 ("NexGen Provides Updated Economics for the Rook I Project") has the same rows at US$50, 80 and 100.

Other inputs:
- Close of NXE on 2026-11-20: US$6.40 (data store fa-data a798b9b, `NXE/prices.json`, Nasdaq site).
- Exchange rate: 1.4231 Canadian dollars for one US dollar on 2026-09-30. Calculated from the interim statements of
  Denison Mines for 2026-09-30: "US$345,000,000 ($490,970,000)" gives 490.970 / 345 = 1.42310. Our data store has no
  exchange rate series. The rate of 2026-11-20 is not known to this head.
- Uranium prices of 2026-10-30: spot US$91.55, long-term US$98.00 a pound (the URA page of the Head of Economics,
  `analysis/macro/2026-11-23-ura-one-pager.md`, month-end points from Cameco's page).

## 2. The calculation

Sum(p), in millions of Canadian dollars = NPV(p) x 1000 + 2.7 x p x fx + 567.172 + 254.398 + 181.010 - debt x fx.
Market value = 688.034761 x 6.40 x fx. The needed price is the straight line between the two table prices around it.

| Case | fx | Debt, US$ million | Market value | Sum below | Sum above | Needed price |
|---|---:|---:|---:|---|---|---:|
| Table 2 of the page (NPV with the costs of the 2024 update) | 1.4231 | 250 | 6,266.5 | 5,844.2 at US$80 | 6,832.6 at US$90 | US$84.3 |
| The exchange rate of the company's table | 1.3333 | 250 | 5,871.2 | 5,847.2 at US$80 | 6,833.2 at US$90 | US$80.2 |
| The debt of 2026-09-30 (a date mix) | 1.4231 | 320 | 6,266.5 | 5,744.6 at US$80 | 6,733.0 at US$90 | US$85.3 |
| NPV with the costs of the 2021 study | 1.4231 | 250 | 6,266.5 | 6,245.8 at US$70 | 7,224.2 at US$80 | US$70.2 |

Blocks at US$80 and US$90 (fx 1.4231): mine 4,890 and 5,840; uranium held 2.7 x 80 x 1.4231 = 307.4 and 345.8; cash and
short-term investments 821.6; IsoEnergy 181.0; debt 250 x 1.4231 = 355.8. 84.3 against 91.55 is -8.0%; against 98.00
it is -14.0%. Market value in US dollars: 4,403.4 million.

## 3. The four other miners (the check only)

| Miner | Document | What it has | Where |
|---|---|---|---|
| Denison (Phoenix, Gryphon; effective 95%) | "NI 43-101 Technical Report on the Wheeler River Project", effective 2023-06-23; `https://denisonmines.com/site/assets/files/6449/wheeler-river-technical-report-vfinal.pdf` | After-tax value for a base case and one case at US$65; a chart, no table at several prices. Base and US$65 cases use different exchange rates (1.35 and 1.30) | Tables 22-7, 22-9, 22-10, 22-13, 22-15; Figures 22-1, 22-3 |
| Denison (Midwest, 25.17%) | Release of 2025-08-06 (preliminary economic assessment) | Three price points: US$65, 80, 95 | Release, Tables 2 and 6 |
| Uranium Energy (Roughrider) | "S-K 1300 Initial Assessment Report - Roughrider Uranium Project", 2024-11-05; `https://www.uraniumenergy.com/_resources/reports/UEC-SK-1300-Technical-Report-November-2024.pdf` | One price (US$85) at 6%, 8%, 10%; a chart | Tables 19-2 to 19-4; Figure 19-3 |
| Uranium Energy (the US projects) | Form 10-K for the year to 2026-07-31; `https://www.sec.gov/Archives/edgar/data/1334933/000143774926031414/uec20260731_10k.htm` | "none of our properties contain mineral reserves"; the single reports were not opened | Item 1A, Item 2 |
| Cameco (Cigar Lake) | Technical report, effective 2023-12-31; `https://www.cameco.com/sites/default/files/documents/cameco-2023-cigar-lake-technical-report.pdf` | A value before tax; a chart | Table 22-1, Figure 22-1 |
| Cameco (McArthur River / Key Lake) | Technical report, effective 2018-12-31 (a newer one was not looked for) | A value before tax; a chart | Tables 22-1, 22-2; Figure 22-1 |
| Cameco (Inkai) | Technical report, effective 2024-09-30 | A value after tax at 12%; a chart | Table 22-1, Figure 22-1 |
| Energy Fuels | Not opened | Search results only | - |

Denison has no price file in the data store, so its market value was not calculated.

## 4. Limits

- The numbers of documents on SEC EDGAR came through a text copy of the page made by the fetch tool. The NexGen
  numbers were read two times with the same result. The numbers of section 3 were read one time.
- The dates are mixed: balance sheet 2026-09-30, share count 2026-11-09, close 2026-11-20, exchange rate 2026-09-30,
  uranium prices 2026-10-30.
- No value for the ground outside the mine plan (the needed price is too high). No delay and no building risk in the
  table (the needed price is too low). All pounds at one flat price. The 8% rate is the company's.
- A measurement only: not a value of the company, not an input of a model, not a trade view.

## 5. Added after the audit of 2026-11-25 (Head of Quality and Risk: REVISE, three major points; all applied)

- The PDF of the first version had damaged text where a line had two dollar signs (the converter read them as a
  formula). The PDF is now made with `pandoc -f gfm-tex_math_dollars`, and its text is checked with `pdftotext`.
- More numbers of document A (read by the second agent; C$ thousands unless noted):
  - Change in cash, nine months to 2026-09-30: "$159.5 million associated with the development of the Rook I Project
    including detailed engineering, procurement and construction activities"; "$16.9 million of advances made for the
    Rook I Project construction"; "$99.8 million of exploration expenditures in connection with the development of
    the Rook I Project prior to the receipt of the Licence and 2026 exploration drilling".
  - The 2024 update: "an expected increase in pre-production capital costs from approximately C$1.3 billion in the
    Rook I FS Technical Report to approximately C$2.2 billion". The capital spent to date and the remaining capital:
    not found as one number.
  - Funding: "will continue to require additional financing to fund construction of the Rook I Project to
    completion". Options: 45,985,100 on 2026-11-09. The 2024 debentures convert into 23,299,161 shares at US$10.73.
  - Balance sheet: "Mineral property, plant and equipment" 979,026 (2025-12-31: 7,565); "Exploration and evaluation
    assets" 161,796 (812,270).
- The new row of Table 3: the sum with 159.5 + 16.9 = 176.4 million added back (the table of the company subtracts
  the full build cost, and this part is paid and has left the cash). Sums: 6,020.6 at US$80 and 7,009.0 at US$90;
  market value 6,266.5; needed price US$82.5. Only the 2026 part of the paid capital is known; the part paid before
  2026 is not in this row.
- Directions of the limits (on the page): too high, because the ground outside the plan has no value, the mine value
  is not moved forward from its cost base of Q4 2023, and the paid part of the build cost is counted two times; too
  low, because the table has no delay and no building risk and no new share is counted for the financing that the
  company says it needs.
- Table 1: the rows of Denison, Uranium Energy and Cameco were read one time, by one agent, and are not audited.
  Energy Fuels was not opened: the page says "four miners that we opened".
