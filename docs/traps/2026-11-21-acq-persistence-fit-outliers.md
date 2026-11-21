# The fit of the spend persistence is made by a few extreme company-years

*Trap:* The mean spend / revenue of small companies has extreme values (above 100% of revenue in one year). A least-squares fit of the next 3 years on the past 5 years over all company-years gives unstable numbers: slope 0.13 (half 0), 0.13 (half 1) for all sizes against 0.19 and 0.45 for 10B and above. The class "above 5%" has a mean past spend of 22% to 27%, and that mean is the mean of a few companies.

*What to do:* Fit on company-years with base revenue of 1B and above, and cap both means at 25% of revenue (`ACQ_PERSIST_CAP`). With the cap at 10%, 25% or 50% the slope of half 0 is 0.39 to 0.40 and the intercept 1.3% to 1.9%. Cap the past spend in the rule in the same way. The point-in-time test is a median test and the charge is a mean: a change of the base moves the sign of the median error (from over-charged to under-charged), and the size can stay the same. Read the coverage (inside P10 to P90) and the share of real values below P50 too.

*Evidence:* `fa dcf pit --half 0 --diag acq` (fits and class table), `rust/src/dcf/auto_pit.rs` (`render_acq_persistence`), `docs/assumptions/A-002-acquisition-charge.md`, "Limits and options".
