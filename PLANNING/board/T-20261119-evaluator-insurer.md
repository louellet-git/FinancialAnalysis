---
id: T-20261119-evaluator-insurer
title: Build the value model for insurers
area: engine
status: backlog
owner: louellet-git
branch:
created: 2026-11-19
acceptance: `fa dcf auto` gives a value range for a company of the template `insurer` from an excess-return model with underwriting and investment drivers, and a point-in-time test on the insurers in the panel reports the coverage of its bands.
---
Sequence: 2 of 5. Start after `T-20261119-evaluator-bank`, because the two models use
the same excess-return base.

Why a separate model: an insurer holds the premiums of its customers as investments. Its
liabilities are operating items, not financing. `fa dcf auto` gives "no model".

Method (first version): excess-return model on book equity, with these drivers as
yearly processes:

- premium growth;
- underwriting margin (1 minus the combined ratio);
- investment yield on the invested assets (tied to the rate path);
- payout ratio; cost of equity.

Data in the repo now: premiums, policy liabilities, investment income, equity, net
income (40 insurers in the panel). Data to check first: losses and expenses that give
the combined ratio; life insurers and property insurers need different drivers.

Test: fixed answers with all shocks off; the point-in-time test for the 40 insurers.
