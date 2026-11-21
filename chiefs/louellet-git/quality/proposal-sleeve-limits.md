# Proposal: the limits of the paper sleeves

Author: Head of Quality and Risk, 2026-11-21. Ticket: `PLANNING/board/T-20261120-paper-desk.md`, decision 2
("Recommendation: the Head of Quality and Risk proposes the limits"). Approved by the owner on 2026-11-21
(about 17:10 UTC: "sure, but we are able to short stock and long stock, programs can and will place orders"),
with the two changes in the live pilot table below.
The values are conventions for a first paper book. They are not fitted to a result. Change them after the
first quarter of paper results.

## Why now
The first trade recommendation (NVDA, 2026-11-21) gives "no trade". A later recommendation can give a side.
The risk review found that no limit exists yet. Thus no recommendation with a side goes to a paper trade
before the owner approves limits.

## Proposed limits (each sleeve: Fundamental Research, Quant Research)
| Limit | Proposed value | Reason |
|---|---|---|
| Capital | Equal for the two sleeves (owner word, 2026-11-20); the owner sets the sum | Owner request |
| Gross exposure | At most 100% of the sleeve capital (no leverage) | A first book has no margin model |
| One position | At most 10% of the sleeve capital at entry; at most 15% at market | One wrong model must not decide the sleeve |
| One issuer, both sleeves | At most 15% of the whole book | The two sleeves can buy the same name |
| Liquidity | A position at most 1% of the 20-day average daily dollar volume | The fills of a paper book must be realistic |
| Stop of a position | The exit price is written before entry, in the write-up. Also an exit at the first review date (10-Q or earnings) if the thesis test fails | A stop that is set after the loss is not a stop |
| Loss in one day | 2% of the sleeve capital: no new position on the next day; the head writes why | It stops a run of errors |
| Stop of a sleeve | A fall of 10% from the peak of the sleeve: close to cash, review by the risk officer, owner decides the restart | It keeps a bad method from a large loss |
| Short sale | Only with a written borrow cost and a written maximum loss; at most 5% of the sleeve | A short has no natural loss limit |
| Options | Defined risk only (the maximum loss is known at entry); the premium at risk is in the 10% limit | The bots use model prices; a naked short option has no loss limit |
| Costs | The write-up states the spread and the fees; the result is reported after costs | `docs/QUALITY-STANDARDS.md`, section 6 |

## Live pilot (owner word, 2026-11-21 03:47 UTC, relayed by the Head of Fundamental Research)
Words: "And every week you will need to propose 3 trades to begin on the live trader, beginning next
Friday." The backtest evidence is weak (+2.9 to +3.5 points a year over the equal weight, t 1.6 to 2.1,
9 years, names of today). So the live limits are tighter than the paper limits above, until 12 weeks of
live results exist:
| Limit | Live pilot |
|---|---|
| One position | At most 5% of the live sleeve capital at entry (3 new names a week fill about 60% in 4 weeks) |
| Gross exposure | At most 100%; no new position when the sum is at the limit |
| Side and instrument | Long and short common stock (owner, 2026-11-21). A short follows the short-sale row above (borrow cost, maximum loss, at most 5%). No option until a hedging engine passes its gate |
| Loss in one day | 2% of the live sleeve: no new position next week until the head writes why |
| Stop of the sleeve | A fall of 8% from the peak: close to cash; the owner decides the restart |
| Orders | Paper orders by a program are allowed under `docs/QUALITY-STANDARDS.md` section 8 (owner, 2026-11-21: "paper orders first"). No live order until a new owner approval and a review |

## Checks
- Before each trade: the head of the sleeve checks the limits in the write-up (`trading/journal/README.md`).
- Each week: the Head of Quality and Risk checks the book against the limits (ticket, decision 3), and
  sends one message with each break.
- A break of a limit is a finding at the level MAJ. A trade with no write-up is CRIT.

## Not in this proposal
The sum of capital, the start date, and the place of the trades (repo book or Alpaca paper, decision 1).
These stay with the owner.
