# Model: algo engine and its paper broker (`fa algo`)

Updated: 2026-11-25. Commit: 9803c60.

## Purpose
`fa algo` runs one rule on live quotes, or on a replay, with one engine. It tests the pipe: data, decision, order, fill, journal.
The rule is `demo`, a mean reversion on one-minute closes. It has no edge. No result is used for a decision.
With `--broker alpaca-paper --send` the engine also sends LIMIT orders of 1 share to the Alpaca PAPER account.
Without `--send` the run is a dry run. It prints and journals the order that it would send.

## Inputs
| Input | Source | Note |
|---|---|---|
| Quote | Alpaca data host, latest quote, feed IEX (free plan) | IEX is not the NBBO. A quote older than 10 seconds is skipped |
| Market clock | `GET /v2/clock` on the paper host, with the paper keys | A dry run reads the paper keys only for this call, when they are set. It never sends them elsewhere and never makes an order request. Without paper keys the run makes no request to the trading host. It uses the New York hours rule: weekdays 09:30 to 16:00. Holidays are not known then |
| Account, positions, orders | Paper host, only with `--send` | Read before each order |
| 20-day dollar volume | Alpaca daily bars (IEX volume) | Used for the liquidity limit |
| Keys | Data: `APCA_API_KEY_ID` and `APCA_API_SECRET_KEY`. Orders: `ALPACA_PAPER_KEY_ID` and `ALPACA_PAPER_SECRET_KEY` | The `APCA_` keys must be paper keys or data keys, never live keys. On the order host both pairs can hold the same paper key. The order code reads only its own pair |

## The simulator
It fills in memory. A buy fills at the ask and a sell at the bid, on the first input 250 ms after the decision. The fee is 0.005 USD a share (A-155).
It has no partial fill, no queue and no market impact. The loss stop is approximate.
The simulator flattens its own position at a stop, for its own P&L. It never touches the broker position.

## The paper broker (`rust/src/trading/algo/paper.rs`)
- The order host is the constant `https://paper-api.alpaca.markets`. No variable changes it. The client follows no redirect.
- The Alpaca account object has no field "paper". The check is the host constant (a live key does not work there), status `ACTIVE` or `PAPER_ONLY`, no block, and USD.
  The run log says "paper: yes" or "paper: no". It never shows the account number.
- Each reply must match the request: id, client id, symbol, side, quantity, type and limit price. A mismatch stops the run.
- Orders are limit orders, day orders, 1 share, long only, with no margin and no option (A-156, A-157).
- The constant `LIMITS` holds the paper limits of `chiefs/louellet-git/quality/proposal-sleeve-limits.md`. A limit that cannot be checked refuses the order.
- The sleeve stop (10% below the peak) refuses a run at its start.
- A stop (kill file, 2% daily loss, three API errors, the end time `--until`) cancels all open orders and keeps the position.
  It writes the log line with the quantity and the average entry price. It sends no new order. A person closes a position.
  The run prints "position kept: N shares; a person closes it". The next run refuses to start while the account holds the symbol.
  An exit with no stop (an error path) also cancels the open orders.
  The program cancels only the orders that it sent: the client id has the prefix `fa-<date>-demo-SPY-` and is in the journal.
  It cancels nothing when the start did not complete (for example when the account has open orders).
- `--send` is an act of the person. The order-host check stops a mistake, not a person who wants to go around it.
- `--send` runs only on the laptop or the order host. It refuses in a Claude Code or cloud session (a variable `CLAUDECODE` or `CCR_*`).
  It needs the order host file outside the repo: `$HOME/.config/fa/order-host`, on Windows `%APPDATA%\fa\order-host`.
  Its first line must be the host name of the machine. The refusal prints the one command that makes the file:
  `mkdir -p ~/.config/fa && echo <host name> > ~/.config/fa/order-host`.
- The journal is `data/processed/algo/<date>/demo-orders.jsonl`. Git ignores it. It holds the intent before each call, the broker reply,
  and a `paper_pair` line. The pair shows the order, the broker fill and the simulator fill of one decision.

## Limits of use
- No live order. A live order needs a new owner approval and a review.
- A paper result is not evidence for a live result. A paper fill has no queue, no market impact, no regulatory fee and no depth.
- IEX quotes are not the NBBO. The NBBO is at least as good as the IEX quote, so a limit at the IEX touch is marketable.
- Step A allows SPY shares only. Options are not allowed (section 8, control 6).
- The sleeve capital is the equity of the paper account. The write-up limits (exit price, spread and fees) are for a person, not for this code.
- A run is a pipe test and a cost measurement. It does not show an edge.

## Backtest: none applies, because the demo rule is a pipe test with no claim of return, and the paper fills are a forward measurement of costs.

## Controls of `docs/QUALITY-STANDARDS.md` section 8
| Control | Code | Named test |
|---|---|---|
| 0 Terms | `docs/DATA-SOURCES.md`, section "Alpaca paper trading" | No test: a document |
| 1 Endpoint | `paper.rs`: `PAPER_HOST`, `paper_url`, `host_is_paper`, `RealTransport::send`; `mapcheck` `Client::no_redirect` | `host_is_compared_exactly_and_the_url_is_built_from_the_constant`, `a_base_url_setting_never_changes_the_order_host`, `a_redirect_is_not_followed_and_the_order_is_not_sent_again`, `no_redirect_client_follows_no_redirect_over_real_http` |
| 2 Keys | `paper.rs`: `PaperKeys::read`, `choose_mode`, `check_order_host`, `parse_paper_account`, `ny_hours_gate`; `cli.rs`: `plan_mode`, `clock_gate` | `only_the_paper_variables_are_read`, `without_send_or_without_keys_no_request_is_possible`, `without_send_the_mode_is_a_dry_run_even_when_the_paper_keys_are_set`, `send_makes_a_transport_only_on_an_order_host_with_keys`, `send_is_refused_in_a_claude_code_or_cloud_session`, `send_needs_the_order_host_file_with_this_host_name`, `the_inside_the_repo_check_resolves_dots_symlinks_and_relative_paths`, `the_account_check_refuses_a_bad_account_before_any_order`, `a_secret_in_an_error_text_is_taken_out`, `without_the_paper_keys_the_start_gate_uses_the_new_york_hours_and_no_request` |
| 3 Limits | `paper.rs`: `LIMITS`, `check_order` | `the_limit_constant_is_the_paper_table`, `each_limit_refuses_and_a_missing_fact_refuses`, `the_hook_reads_the_account_for_each_order_and_refuses_when_it_cannot` |
| 4 Stops | `paper.rs`: `PaperHook::stop`, `cancel_all`, `Drop`, `poll`; `engine.rs` | `the_stop_cancels_open_orders_keeps_the_position_and_sends_no_new_order`, `a_loss_stop_through_the_engine_keeps_the_broker_position`, `the_broker_loss_stop_asks_the_engine_to_stop`, `a_send_run_with_the_kill_file_makes_no_post`, `the_cancels_at_a_stop_skip_the_budget_and_the_error_count`, `an_exit_with_no_stop_cancels_the_open_orders`, `a_restart_keeps_the_day_pnl_from_the_journal`, `a_refused_start_with_an_open_order_that_is_not_ours_sends_no_cancel`, `a_stop_cancels_only_the_orders_that_this_program_sent` |
| 5 Duplicates, errors | `paper.rs`: `next_id`, `accept_reply`, `verify_order`, `apply_fill` | `client_order_ids_are_unique_short_journaled_and_never_repeated`, `a_failed_send_reads_the_order_by_its_client_id_and_never_retries`, `three_api_errors_in_a_row_end_the_run`, `one_failed_send_counts_once_in_the_error_count`, `a_reply_that_does_not_match_the_request_marks_the_order_failed_and_stops_the_run`, `a_filled_quantity_above_the_order_or_negative_or_not_whole_is_refused`, `a_reply_with_no_id_is_read_by_the_client_id_so_no_open_order_is_untracked`, `a_status_read_for_another_order_or_with_a_bad_fill_stops_the_run`, `a_status_read_that_falls_from_filled_1_to_filled_0_is_refused_and_stops_the_run` |
| 6 Caps | `paper.rs`: `check_order`, `Budget` | `orders_are_day_limit_orders_with_the_quote_as_the_limit`, `no_option_and_no_other_symbol`, `no_margin_the_order_value_must_be_inside_the_cash`, `at_most_six_orders_a_minute_and_ten_a_day_and_the_budget` |
| 7 Log | `paper.rs`: `PaperHook::log`, `write_pairs`, `journal_path` | `the_order_journal_is_under_the_ignored_folder_and_each_line_has_the_time` |
| 8 Shorts | `paper.rs`: `check_order` (sell branch); `engine.rs`: `long_only` | `a_sell_that_makes_the_position_negative_is_refused`, `long_only_rejects_a_sell_that_makes_the_position_negative` |
| Tripwire | `source.rs` | `no_order_code_in_the_repo`, `the_tripwire_catches_planted_forms` |

## Records
A-155 (simulator fill rule), A-156 (order caps), A-157 (limit price).

## Code and command
`rust/src/trading/algo/`. Command: `fa algo run --strategy demo --symbol SPY --broker alpaca-paper [--send]`.
Tests: `cargo test --manifest-path rust/Cargo.toml --locked algo`.
