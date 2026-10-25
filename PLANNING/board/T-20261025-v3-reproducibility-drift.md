---
id: T-20261025-v3-reproducibility-drift
title: Resolve IBM v3 reproducibility drift
area: ticker:IBM
status: backlog
owner: louellet-git
branch:
created: 2026-10-25
acceptance: analysis.IBM.dcf_scenarios prints the signed-off $37/$141/$232 again, or its output carries a reproduces-as-of note.
---
HIDDEN-ASSUMPTIONS #14: dcf_scenarios derives margins live from the re-imported store and now prints $43/$149/$243. Pin the v3 inputs as constants or stamp the output.
