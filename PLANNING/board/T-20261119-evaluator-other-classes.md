---
id: T-20261119-evaluator-other-classes
title: Decide and build the value models for the classes that have no data or no revenue
area: engine
status: backlog
owner: louellet-git
branch:
created: 2026-11-19
acceptance: Each class that `mapcheck` gives the status N/A has a decision (a model, or a recorded reason for no model), and one class has a working model.
---
Sequence: 5 of 5. Split this ticket into one ticket for each class when the work starts.

Classes that `mapcheck` does not judge now (126 of 1,249 tickers in the cache):

- Foreign filers (forms 20-F and 40-F, many with IFRS data). Needs: a mapping of the
  IFRS tags to the standard variables. Then the standard model applies.
- Filers that do not report in USD. Needs: a currency rule for the value and the price.
- New registrants with no 10-K, or with no XBRL values (example: ANT). Needs: the
  values from the prospectus by hand, or wait for the first 10-Q.
- Companies with no revenue (for example drug developers before approval). A DCF of
  the history is not possible. Possible method: cash and the time that the cash lasts,
  and a value of each project with a probability of success.
- Utilities and other regulated companies are in the template `standard` now. Check if
  they need a model on the regulated asset base.
