---
name: release-checks
description: Use before a pull request (and before a QA agent starts) to run the checks that every change must pass: both test suites, clippy on the new code, the reviewed numbers, the document test counts, and the Windows compile check when platform code changed.
---

# Release checks

Run from the repo root. The CI runs the two test suites (`.github/workflows/tests.yml`).
Run the checks locally first.

## Steps
1. Build and test:
   ```bash
   cargo build --release --manifest-path rust/mapcheck/Cargo.toml
   cargo test --manifest-path rust/mapcheck/Cargo.toml
   cargo build --release --manifest-path rust/Cargo.toml
   cargo test --manifest-path rust/Cargo.toml --locked
   ```
   Write the counts (passed, failed, ignored).
2. Clippy on the changed crate. Fix each warning in the code that you wrote. Do not fix
   old warnings in this change:
   ```bash
   cargo clippy --manifest-path rust/Cargo.toml --all-targets
   ```
3. The real command of the change runs and its output is usable by the next step
   (`CLAUDE.md`, "Smoke test").
4. Reviewed numbers: `rust/target/release/fa dcf drivers IBM` (seed 7) gives
   $83.84 / $151.84 / $258.03. If a change moves them, the change must say why.
5. Test counts in documents. `CLAUDE.md`, `README.md` and `docs/ARCHITECTURE.md` name the
   counts. Update them in the same change, with the numbers of step 1.
6. If the change touched platform code (`rust/src/sync/interrupt.rs`, paths, console
   code), compile for Windows when the target is installed:
   `cargo check --manifest-path rust/Cargo.toml --target x86_64-pc-windows-gnu`. If the
   target is not installed, write "Windows not compiled" (trap #259).
7. Assumption values: if a value changed, its record in `docs/assumptions/` and the
   model document in `docs/models/` changed too.
8. `git status`: no `data/`, `.env`, `target/`, scratch file.

## Check
- Both suites pass, with the counts written.
- No new clippy warning in the new code.
- The IBM numbers and the document counts are right.
