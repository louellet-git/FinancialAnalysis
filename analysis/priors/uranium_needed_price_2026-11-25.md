# Uranium Price Needed: NexGen

*The flat uranium price at which the published value of a miner's mine, with its cash and debt, equals its market value. A reading of the company's own table for 2026-11-20; no model of ours.*

Background: this is the small step that you chose after the design of the miner model. Some miners print the value
of a mine at several uranium prices. From such a table we can read, with no new code, the uranium price that a stock
price needs. We opened the reports and filings of four URA miners.

Of the four, only NexGen prints a full table. **The price of NexGen (US$6.40 on 2026-11-20) needs a flat uranium price
of about US$84 a pound.** On 2026-10-30 the spot price was US$91.55 and the long-term contract price US$98.00. So the
price of NexGen needs a uranium price about 8% below spot. For the three other miners no such number can be read.

**Table 1: Which miner prints its mine value at several uranium prices.**
See: one of four. Rows 2 to 4 were read one time, by one agent, and are not audited.

| Miner | A table at several prices? | What the documents have |
|---|---|---|
| NexGen (Rook I, 100% owned) | Yes | 8 flat prices, US$30 to US$100, in the report of the third quarter of 2026 |
| Denison (Phoenix, Gryphon, Midwest) | No | 2 or 3 price points for each project, on different exchange rates and cost years |
| Uranium Energy (UEC) | No | Roughrider: one price (US$85) and a chart. Its annual report says that none of its properties has reserves |
| Cameco (Cigar Lake, McArthur River, Inkai) | No | One base value for each mine (before tax for two of them) and a chart |
| Energy Fuels | Not opened | |

**Table 2: NexGen, the sum at two uranium prices** (millions of Canadian dollars).
See: the market value is between the two sums; the needed price is read between US$80 and US$90.

| Block | At US$80 | At US$90 |
|---|---:|---:|
| Rook I mine: value after tax at 8%, with the costs of the 2024 update | 4,890 | 5,840 |
| Uranium held: 2.7 million pounds, at the price | 307 | 346 |
| Cash and short-term investments (2026-09-30) | 822 | 822 |
| Stake in IsoEnergy (27.8%), at book value | 181 | 181 |
| Debt: convertible debentures of US$250 million (2026-11-09) | -356 | -356 |
| **Sum** | **5,844** | **6,833** |
| **Market value**: 688.0 million shares at US$6.40, at 1.4231 Canadian dollars for one US dollar | **6,267** | **6,267** |

**Table 3: How the needed price moves with the inputs that are not sure.**
See: the cost update of 2024 is the large one; the others move the result by a few dollars.

| Input | Needed price |
|---|---:|
| As in Table 2 | US$84 |
| The exchange rate of the company's table (1.3333) for the market value, the uranium held and the debt | US$80 |
| Debt of US$320 million: the debt on 2026-09-30, the date of the cash. US$70 million of it became shares on 2026-10-22, and those shares are in the count. A date mix | US$85 |
| Add back the 176 million paid for the building of Rook I in the nine months of 2026 (the table subtracts the full build cost) | US$82 |
| The costs of the 2021 study (replaced by the 2024 update) | US$70 |

**What moves the number, and in which direction**

- Too high (the true needed price is lower): the ground outside the mine plan gets no value; the mine value has the
  costs of the end of 2023 and its own date, and it is not moved forward to 2026-11; the table subtracts the full build
  cost (about 2.2 billion), but the part that is paid has left the cash (only the 2026 part is known: Table 3).
- Too low (the true needed price is higher): the table has no delay and no building risk (construction started on
  2026-06-08; no date of first production); the company says that it needs more financing to complete the build, and
  no new share is counted (also not the 46.0 million options and the 23.3 million shares of the debentures).
- The table sells all pounds at one flat price, at the 8% rate of the company ("for illustrative purposes only").
  The input dates are mixed, and the exchange rate is of 2026-09-30 (our data have no exchange rate series).
- It is a measurement, not a value of the company and not a trade view.

**For your decision on the build**: the model is the only way to get this number for the other miners. UEC is not a
good first company (no reserves, no table). NexGen or Denison is; both need the Canadian reports and the IFRS data.

Status: NexGen numbers audited 2026-11-25. Sources: `uranium_needed_price_sources_2026-11-25.md`.
