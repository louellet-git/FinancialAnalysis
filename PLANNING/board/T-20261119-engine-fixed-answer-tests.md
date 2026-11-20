---
id: T-20261119-engine-fixed-answer-tests
title: Add fixed-answer tests for the two DCF engines
area: engine
status: backlog
owner: merlan1996
head: fundamental
order: 8
branch:
created: 2026-11-19
acceptance: Pytest compares each engine, with randomness off, against a DCF calculated by hand, and the values agree.
---
Created for merlan1996 by the session of louellet-git; the owner can change it. Source: pull request #16 discussion (the owner is not sure that the engine is stable). The JNJ run already agrees with a hand DCF at driver means ($112.0194); make this a test, and add one for IBM and one for the classic engine.
