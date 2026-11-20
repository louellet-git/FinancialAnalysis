# Trade write-ups

Each trade of the paper account, and later of a live account, has one write-up. It explains the
logic of the trade and the steps taken. A write-up is at most 2 pages (about 120 lines).

- One file for each trade: `trading/journal/<YYYY-MM-DD>-<sleeve>-<ticker>.md` (`<sleeve>` is
  `fundamental` or `quant`). One file for each trade, so two heads do not edit the same file.
- The head of the sleeve writes it before the order, and adds the result after the close.
- It is a record: correct an error with a dated line; do not delete a write-up.
- No secret, no account number and no key in a write-up (this repo is public).

## Form

```markdown
# <YYYY-MM-DD> <sleeve> <ticker>: <buy or sell> <instrument>

Account: paper | live. Status: planned | open | closed. Ticket: <id>.

## Logic
- The thesis in 2 or 3 sentences.
- The signal or the value: the command, the store commit, and the number (for example
  `fa dcf auto <T>`, P50 against the price).
- The evidence: the backtest or the review that supports the strategy (pull request or file).
- What would show that the thesis is wrong.

## The trade
Instrument, side, size (and its share of the sleeve), entry price or limit, exit plan (target,
stop, date), the limits that apply.

## Steps taken
1. Data: the store commit and the date of each data set used.
2. Model or strategy run: the commands and their outputs (short).
3. Review: who checked it, and the verdict.
4. Order: when, where (paper or live), the fill.

## Risks
The main risks and the costs (spread, fees, tax if known).

## Result (after the close)
Exit date and price, profit or loss, and what we learned.
```
