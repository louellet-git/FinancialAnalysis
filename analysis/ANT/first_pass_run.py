"""ANT first-pass runner (seed 7, 10,000 simulations). Run from the repo root:

    python -m analysis.ANT.first_pass_run

Prints the headline, the engine check against a hand DCF at the driver means, the
one-at-a-time sensitivities, the values that the market price implies, and the
drivers that explain the spread. Findings: analysis/ANT/FIRST-PASS.md.

First pass, not reviewed. Claude models made it, and Anthropic makes Claude.
"""
import sys
from dataclasses import replace
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import numpy as np

from analysis.ANT import driver_model as dm
from financial_analysis.driver_dcf import DriverDCF

IPO_PRICE = 83.00      # prospectus cover, L9
# ANT had no trade at 11:15 New York time on 2026-11-19 (stockanalysis.com/stocks/ant,
# stocktitan.net live page, Yahoo Finance live blog). The second reference price is the
# last opening indication that the press gave ($105.00, Yahoo Finance live blog, about
# 10:20 New York time). An indication is not a trade.
REF_PRICE = 105.00
REF_LABEL = "opening indication (press), no trade yet"
SEED, N = 7, 10_000
B = 1e9


def _run(*, mean=None, net_debt=None, shares=None, **build_args):
    """One engine run. `mean` = {driver: new mean}; truncation bounds move with it."""
    b = dm.build(**build_args)
    model = b["model"]
    if mean:
        drivers = []
        for d in model.drivers:
            if d.name in mean:
                shift = mean[d.name] - d.mean
                d = replace(d, mean=mean[d.name],
                            lo=None if d.lo is None else d.lo + shift,
                            hi=None if d.hi is None else d.hi + shift)
            drivers.append(d)
        model = replace(model, drivers=drivers)
    years = build_args.get("years", dm.PROJECTION_YEARS)
    engine = DriverDCF(model, b["segments"],
                       net_debt=b["net_debt"] if net_debt is None else net_debt,
                       shares=b["shares"] if shares is None else shares,
                       projection_years=years)
    return engine, engine.run(N, SEED)


def _p50(**kw) -> float:
    return _run(**kw)[1].per_share_p50


def _means(build: dict) -> dict:
    return {d.name: d for d in build["model"].drivers}


def _wacc_at_means(d: dict) -> float:
    m = lambda name: d[name].mean
    return (m("equity_weight") * (m("rf") + m("beta") * m("erp"))
            + (1 - m("equity_weight")) * (m("rf") + m("kd_spread")) * (1 - m("tax_rate")))


def _path_at_means(growth_by_year=None):
    """(revenue, margin after SBC) for each year at the driver means. `growth_by_year`
    replaces the constant growth of years 2..N (hand calculation for the fade)."""
    b = dm.build()
    d = _means(b)
    seg = b["segments"][0]
    val = lambda name, t: d[name].schedule[t - 1] if d[name].schedule else d[name].mean
    rev, margin = [], []
    level = seg.base_revenue * (1 + seg.year1_growth)
    for t in range(1, dm.PROJECTION_YEARS + 1):
        if t > 1:
            level *= 1 + (growth_by_year[t - 2] if growth_by_year else d["g_total"].mean)
        rev.append(level)
        margin.append(val("core_margin", t) + val("margin_mix_drift", t)
                      - val("sbc_ratio", t) - val("ma_recurrence", t) * val("ma_ratio", t))
    return rev, margin


def _hand_dcf_at_means(growth_by_year=None):
    """Independent closed-form DCF with each driver at its mean.
    Returns (value for each share, EV, PV of the terminal value)."""
    b = dm.build()
    d = _means(b)
    wacc = _wacc_at_means(d)
    rev, margin = _path_at_means(growth_by_year)
    pv = sum(r * mg / (1 + wacc) ** t for t, (r, mg) in enumerate(zip(rev, margin), 1))
    tg = min(d["terminal_growth"].mean, wacc - 0.02)
    tv = rev[-1] * margin[-1] * (1 + tg) / (wacc - tg) / (1 + wacc) ** dm.PROJECTION_YEARS
    return (pv + tv - b["net_debt"]) / b["shares"], pv + tv, tv


def _bisect(f, lo, hi, target, steps=40):
    """Solve f(x) = target. f must increase from lo to hi. None if out of range."""
    if not f(lo) <= target <= f(hi):
        return None
    for _ in range(steps):
        mid = 0.5 * (lo + hi)
        lo, hi = (mid, hi) if f(mid) < target else (lo, mid)
    return 0.5 * (lo + hi)


def _year10_revenue(g: float, year1: float = dm.YEAR1_REVENUE) -> float:
    return year1 * (1 + g) ** (dm.PROJECTION_YEARS - 1)


def _fade_growth():
    """Growth path that starts high in year 2 and goes down linearly to the terminal
    growth mean in year 10, with the same year-10 revenue as the constant mean."""
    n = dm.PROJECTION_YEARS - 1
    end = 0.0225
    target = (1 + dm.G_MEAN) ** n
    path = lambda g2: [g2 + (end - g2) * i / (n - 1) for i in range(n)]
    g2 = _bisect(lambda x: float(np.prod([1 + g for g in path(x)])), end, 2.0, target)
    return path(g2)


def main() -> None:
    engine, r = _run()
    b = dm.build()
    d = _means(b)
    paths = r.per_share_paths
    shares, net_debt = b["shares"], b["net_debt"]
    q = np.percentile(paths, [10, 25, 50, 75, 90])
    wacc = _wacc_at_means(d)

    print(f"ANT first pass - seed {SEED}, {N:,} simulations, {dm.PROJECTION_YEARS} years")
    print(f"  IPO price ${IPO_PRICE:.2f}; second reference ${REF_PRICE:.2f} = {REF_LABEL}")
    print(f"  base revenue ${dm.BASE_REVENUE / B:.3f}B (12 months to 2026-09-30); year 1 pinned "
          f"at ${dm.YEAR1_REVENUE / B:.1f}B ({dm.YEAR1_REVENUE / dm.BASE_REVENUE - 1:+.0%})")
    print("\nHEADLINE (value for each share, USD)")
    print("  P10 {:.2f} | P25 {:.2f} | P50 {:.2f} | P75 {:.2f} | P90 {:.2f}".format(*q))
    print(f"  paths with value below ${IPO_PRICE:.2f}: {np.mean(paths < IPO_PRICE):.1%}; "
          f"below ${REF_PRICE:.2f}: {np.mean(paths < REF_PRICE):.1%}")
    print(f"  WACC at means {wacc:.2%} (rf {d['rf'].mean:.2%} + beta {d['beta'].mean:.2f} x "
          f"ERP {d['erp'].mean:.2%}, equity weight {d['equity_weight'].mean:.2f}); "
          f"terminal growth pinned on {r.tg_pinned_fraction:.2%} of paths")
    print(f"  terminal value share of EV at means {r.tv_ev_share_at_means:.1%}")
    rev, margin = _path_at_means()
    for t in (1, 3, 5, 10):
        print(f"  year {t:>2} at means: revenue ${rev[t - 1] / B:7.1f}B, FCFF margin after SBC "
              f"{margin[t - 1]:+6.1%}, FCFF ${rev[t - 1] * margin[t - 1] / B:7.1f}B")
    g_draws = r.driver_draws["g_total"]
    y10 = [_year10_revenue(g) / B for g in np.percentile(g_draws, [10, 50, 90])]
    print("  year-10 revenue: P10 ${:.0f}B | P50 ${:.0f}B | P90 ${:.0f}B".format(*y10))
    equity_p50 = q[2] * shares
    ev_p50 = equity_p50 + net_debt
    print(f"  equity value at P50 ${equity_p50 / B:,.0f}B on {shares / 1e6:,.1f}M shares; "
          f"net cash ${-net_debt / B:.1f}B (${-net_debt / shares:.2f} a share); "
          f"EV at P50 ${ev_p50 / B:,.0f}B = {ev_p50 / dm.BASE_REVENUE:.1f} x trailing revenue")
    for price in (IPO_PRICE, REF_PRICE):
        print(f"  market value at ${price:.2f}: ${price * shares / B:,.0f}B; EV "
              f"{(price * shares + net_debt) / dm.BASE_REVENUE:.0f} x trailing revenue")
    print(f"  base rate: AMZN 27.4% a year for 10 years from this base = "
          f"${dm.BASE_REVENUE * 1.274 ** 10 / B:.0f}B in year 10")

    print("\nENGINE CHECK (each driver at its mean)")
    engine_mean = float(np.ravel(engine._valuation(engine._mean_draws())[0])[0])
    hand, ev_hand, tv_hand = _hand_dcf_at_means()
    print(f"  engine ${engine_mean:.4f} | hand DCF ${hand:.4f} | EV ${ev_hand / B:,.1f}B, "
          f"of which terminal value ${tv_hand / B:,.1f}B")

    print("\nENGINE GROWTH LIMIT (hand calculation at means, same margins, same year-10 revenue)")
    fade = _fade_growth()
    fade_value = _hand_dcf_at_means(fade)[0]
    fade_rev, _ = _path_at_means(fade)
    print(f"  constant {dm.G_MEAN:.0%} from year 2: ${hand:.2f}; year-5 revenue ${rev[4] / B:.0f}B")
    print(f"  fade from {fade[0]:.1%} in year 2 to {fade[-1]:.2%} in year 10: ${fade_value:.2f}; "
          f"year-5 revenue ${fade_rev[4] / B:.0f}B; difference ${fade_value - hand:+.2f}")

    print("\nCOMMITMENT CHECK (prospectus L1655, L440; revenue at means)")
    cum = sum(rev[:4]) + 0.25 * rev[4]          # 2026-10-01 to 2030-12-31 = 4.25 model years
    g10 = float(np.percentile(g_draws, 10))
    low = [dm.YEAR1_REVENUE * (1 + g10) ** t for t in range(5)]
    cum_low = sum(low[:4]) + 0.25 * low[4]
    print(f"  next 12 months: committed $70.7B = {70.7e9 / rev[0]:.0%} of year-1 revenue "
          f"(compute and infrastructure was 57% of 9M2026 revenue)")
    print(f"  to 2030-12-31: committed $451.6B = {451.6e9 / cum:.0%} of revenue at mean growth "
          f"(${cum / B:.0f}B), {451.6e9 / cum_low:.0%} at P10 growth {g10:.1%} (${cum_low / B:.0f}B)")
    # Assumption of this check: 70.7B in year 1; the remainder to 2030-12-31 (380.9B) in
    # equal parts over 3.25 years; other cash costs stay at 31% of revenue (9M2026:
    # personnel less SBC 1,608 + sales, marketing and partnerships 5,529 + other
    # operating less donation matching 3,216 = 10,353 of 33,242); no more compute.
    spend = [70.7e9] + [380.9e9 / 3.25] * 3
    pre_sbc = [m_ + s_ for m_, s_ in zip(margin, d["sbc_ratio"].schedule)]
    implied = [1 - s_ / r_ - 0.31 for s_, r_ in zip(spend, rev)]
    print("  FCFF margin before SBC, years 1 to 4: model "
          + " / ".join(f"{x:+.0%}" for x in pre_sbc[:4])
          + "; from the committed spend " + " / ".join(f"{x:+.0%}" for x in implied))
    gap = sum(min(i_ - p_, 0.0) * r_ / (1 + wacc) ** t
              for t, (i_, p_, r_) in enumerate(zip(implied, pre_sbc, rev), 1))
    print(f"  where the committed spend gives a lower margin (years 2 to 4), the effect at "
          f"means is ${gap / shares:+.2f} a share; this is NOT in the base")

    print("\nSENSITIVITIES (P50, one change, the other drivers at base)")
    print(f"  base: {q[2]:.2f}")
    for g in (0.10, 0.15, 0.25, 0.30):
        print(f"  growth mean {g:.0%} (year-10 revenue ${_year10_revenue(g) / B:,.0f}B): "
              f"{_p50(mean={'g_total': g}):.2f}")
    for cm in (0.16, 0.28, 0.34):
        print(f"  mature margin after SBC {cm - dm.SBC_MATURE:.0%} (year 1 held): "
              f"{_p50(core_mean=cm):.2f}")
    for y1, why in ((80.6e9, "run rate stays at $81B"), (125e9, "run rate adds as in 2026")):
        print(f"  year-1 revenue ${y1 / B:.1f}B ({why}): {_p50(year1_revenue=y1):.2f}")
    for ramp in (6, 10):
        print(f"  mature margin and SBC reached in year {ramp}: {_p50(ramp_years=ramp):.2f}")
    print(f"  SBC year 1 at 15% (expense basis): {_p50(sbc_year1=0.15):.2f}")
    for beta in (1.10, 1.70):
        print(f"  beta {beta:.2f} (cost of equity {d['rf'].mean + beta * d['erp'].mean:.2%}): "
              f"{_p50(mean={'beta': beta}):.2f}")
    for rf in (0.0407, 0.0607):
        print(f"  rf {rf:.2%}: {_p50(mean={'rf': rf}):.2f}")
    print(f"  horizon 15 years, growth mean {dm.G_MEAN:.0%} (year-15 revenue "
          f"${dm.YEAR1_REVENUE * (1 + dm.G_MEAN) ** 14 / B:,.0f}B, no precedent): "
          f"{_p50(years=15):.2f}")
    # Year-15 revenue if growth goes down after year 10: 17%, 14%, 11%, 8%, 5%.
    y15 = _year10_revenue(dm.G_MEAN) * 1.17 * 1.14 * 1.11 * 1.08 * 1.05
    g15 = (y15 / dm.YEAR1_REVENUE) ** (1 / 14) - 1
    print(f"  horizon 15 years, growth mean {g15:.1%} (year-15 revenue ${y15 / B:,.0f}B): "
          f"{_p50(years=15, mean={'g_total': g15}):.2f}")
    no_fund = shares - 846_840_960
    print(f"  shares without the 846.8M that the subsidiary holds ({no_fund / 1e6:,.1f}M): "
          f"{_p50(shares=no_fund):.2f}")
    outstanding = 7_583_103_996 + 4_789_697_318
    print(f"  shares outstanding only ({outstanding / 1e6:,.1f}M), notes as a liability of "
          f"$109.7B; no options, no RSUs, thus not a correct count: "
          f"{_p50(shares=outstanding, net_debt=net_debt + 109_742e6):.2f}")
    kd = d["rf"].mean + d["kd_spread"].mean
    pay = 451.2e9 / 10                               # leases not started, L2395
    extra = sum(pay / (1 + kd) ** t - pay / (1 + wacc) ** t for t in range(2, 12))
    print(f"  leases not started ($451.2B) as debt at {kd:.2%} and out of the margin "
          f"(net debt +${extra / B:.1f}B): {_p50(net_debt=net_debt + extra):.2f}")
    print(f"  Damodaran debt $472.3B added, margin not changed (counts the cost two times): "
          f"{_p50(net_debt=net_debt + 472.3e9):.2f}")

    print("\nWHAT A PRICE IMPLIES (bisection on the engine P50, one driver, others at base)")
    for price, label in ((IPO_PRICE, "IPO price"), (REF_PRICE, REF_LABEL)):
        print(f"  ${price:.2f} ({label}); equity ${price * shares / B:,.0f}B")
        g = _bisect(lambda x: _p50(mean={"g_total": x}), 0.0, 0.80, price)
        print(f"    (a) growth from year 2: {g:.1%} a year for 9 years; year-10 revenue "
              f"${_year10_revenue(g) / B:,.0f}B" if g is not None else "    (a) growth: no solution to 80%")
        cm = _bisect(lambda x: _p50(core_mean=x), 0.10, 1.00, price)
        print(f"    (b) mature FCFF margin after SBC: {cm - dm.SBC_MATURE:.1%} "
              f"({cm:.1%} before SBC)" if cm is not None
              else "    (b) mature FCFF margin: no solution below 100% of revenue")
        beta = _bisect(lambda x: -_p50(mean={"beta": x}), 0.0, 3.0, -price)
        print(f"    (c) beta: {beta:.2f}; cost of equity {d['rf'].mean + beta * d['erp'].mean:.2%}"
              if beta is not None else "    (c) beta: no solution above 0")
        # Joint case: Damodaran year-10 FCFF margin (19.0% after SBC) and beta 1.10.
        joint = {"beta": 1.10}
        gj = _bisect(lambda x: _p50(mean={**joint, "g_total": x}, core_mean=0.25), 0.0, 0.80, price)
        print(f"    joint: growth {gj:.1%} (year-10 revenue ${_year10_revenue(gj) / B:,.0f}B) with "
              f"mature margin 19% after SBC and beta 1.10 (cost of equity "
              f"{d['rf'].mean + 1.10 * d['erp'].mean:.2%})")
    # Damodaran revenue and margin on the repo discount conventions.
    g_dam = (1_011e9 / dm.YEAR1_REVENUE) ** (1 / 9) - 1
    print(f"  Damodaran year-10 revenue $1,011B (growth {g_dam:.1%}) and margin 19% after SBC, "
          f"repo discount rate: P50 {_p50(mean={'g_total': g_dam}, core_mean=0.25):.2f}")
    print(f"  Damodaran equity value $1,475B on this share count: ${1_475e9 / shares:.2f} a share")

    print("\nDRIVERS OF THE SPREAD (engine attribution: P50 change for mean -1 std / +1 std)")
    ranked = sorted(r.attribution.values(),
                    key=lambda a: abs(a.p50_plus_1sd - a.p50_minus_1sd), reverse=True)
    for a in ranked:
        print(f"  {a.driver:16s} {a.p50_minus_1sd:+7.2f} / {a.p50_plus_1sd:+7.2f} "
              f"(std {d[a.driver].std:.2%})")
    print("  beta has no draw (deterministic): see the beta lines above.")


if __name__ == "__main__":
    main()
