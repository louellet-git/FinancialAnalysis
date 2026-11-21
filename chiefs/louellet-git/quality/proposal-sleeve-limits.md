# Proposal: the limits of the paper sleeves

Author: Head of Quality and Risk, 2026-11-21. Ticket: `PLANNING/board/T-20261120-paper-desk.md`, decision 2
("Recommendation: the Head of Quality and Risk proposes the limits"). This is a proposal. The owner decides.
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

## Checks
- Before each trade: the head of the sleeve checks the limits in the write-up (`trading/journal/README.md`).
- Each week: the Head of Quality and Risk checks the book against the limits (ticket, decision 3), and
  sends one message with each break.
- A break of a limit is a finding at the level MAJ. A trade with no write-up is CRIT.

## Not in this proposal
The sum of capital, the start date, and the place of the trades (repo book or Alpaca paper, decision 1).
These stay with the owner.
