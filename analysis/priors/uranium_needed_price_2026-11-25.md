# Uranium Price Needed: NexGen

*The flat uranium price at which the published value of a miner's mine, with its cash and debt, equals its market value. A reading of the company's own table for 2026-11-20; no model of ours.*

Background: this is the small step that you chose after the design of the miner model. Some miners print the value
of a mine at several uranium prices. From such a table we can read, with no new code, the uranium price that a stock
price needs. We checked the technical reports and filings of five URA miners.

Only NexGen prints a full table. **The price of NexGen (US$6.40 on 2026-11-20) needs a flat uranium price of about
US$84 a pound.** On 2026-10-30 the spot price was US$91.55 and the long-term contract price US$98.00. So the price of
NexGen needs a uranium price about 8% below spot. For the four other miners no such number can be read.

**Table 1: Which miner prints its mine value at several uranium prices.**
See: one of five. The others print one or two price points, or a chart only.

| Miner | A table at several prices? | What the documents have |
|---|---|---|
| NexGen (Rook I, 100% owned) | Yes | 8 flat prices, US$30 to US$100, in the report of the third quarter of 2026 |
| Denison (Phoenix, Gryphon, Midwest) | No | 2 or 3 price points for each project, on different exchange rates and cost years |
| Uranium Energy (UEC) | No | Roughrider: one price (US$85) and a chart. Its annual report says that none of its properties has reserves |
| Cameco (Cigar Lake, McArthur River, Inkai) | No | One base value for each mine (before tax for two of them) and a chart |
| Energy Fuels | Not checked | The reports were not opened |

**Table 2: NexGen, the sum at two uranium prices** (millions of Canadian dollars).
See: the market value is between the two sums; the needed price is read between US$80 and US$90.

| Block | At US$80 | At US$90 |
|---|---:|---:|
| Rook I mine: value after tax at 8%, with the costs of the 2024 update | 4,890 | 5,840 |
| Uranium held: 2.7 million pounds, at the price | 307 | 346 |
| Cash and short-term investments (2026-09-30) | 822 | 822 |
| Stake in IsoEnergy (27.8%), at book value | 181 | 181 |
| Debt: convertible debentures of US$250 million | -356 | -356 |
| **Sum** | **5,844** | **6,833** |
| **Market value**: 688.0 million shares at US$6.40, at 1.4231 Canadian dollars for one US dollar | **6,267** | **6,267** |

**Table 3: How the needed price moves with the inputs that are not sure.**
See: the cost update of 2024 is the large one; the others move the result by a few dollars.

| Input | Needed price |
|---|---:|
| As in Table 2 | US$84 |
| The exchange rate of the company's table (1.3333) for the market value | US$80 |
| Debt of US$320 million (a second number in the same filing) | US$85 |
| The costs of the 2021 study (replaced by the 2024 update) | US$70 |

**What this number does not say**

- It is one date, read by hand. The dates are not the same: the balance sheet is of 2026-09-30, the share count of
  2026-11-09, the exchange rate of 2026-09-30 (our data have no exchange rate series).
- Too high: the ground outside the mine plan gets no value. Too low: the table has no delay and no building risk
  (construction started on 2026-06-08; the filing gives no date of first production).
- The table sells all pounds at one flat price, at the 8% rate of the company. The company calls it "for illustrative
  purposes only".
- It is a measurement, not a value of the company and not a trade view.

**For your decision on the build**: the model is the only way to get this number for the four other miners. UEC is
not a good first company (no reserves, no table). NexGen or Denison is; both need the Canadian reports and the IFRS data.

Status: read from the filings by one agent, checked a second time for NexGen. Sources: `uranium_needed_price_sources_2026-11-25.md`.
