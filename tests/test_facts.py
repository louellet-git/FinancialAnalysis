from financial_analysis.facts import (
    ANNUAL_FORMS,
    QUARTERLY_FORMS,
    YTD_NOTE,
    period_ends,
    select,
    select_annual,
    select_quarter,
)


def _fact(start, end, val, form, filed="2025-02-01"):
    entry = {"end": end, "val": val, "form": form, "filed": filed}
    if start:
        entry["start"] = start
    return entry


def _facts(entries, concept="Revenues"):
    return {"facts": {"us-gaap": {concept: {"units": {"USD": entries}}}}}


# --- annual selection ---

def test_annual_ignores_a_quarter_value_in_a_10k():
    # A 10-K holds the full year and also the fourth quarter, with the same end date.
    entries = [
        _fact("2024-01-01", "2024-12-31", 100.0, "10-K", filed="2025-02-01"),
        _fact("2024-10-01", "2024-12-31", 27.0, "10-K", filed="2025-02-02"),
    ]
    assert select_annual(entries, "2024-12-31").value == 100.0


def test_annual_returns_none_when_only_a_quarter_exists():
    entries = [_fact("2024-10-01", "2024-12-31", 27.0, "10-K")]
    assert select_annual(entries, "2024-12-31") is None


def test_annual_accepts_a_53_week_year():
    entries = [_fact("2023-12-31", "2025-01-04", 100.0, "10-K")]  # 370 days
    assert select_annual(entries, "2025-01-04").value == 100.0


def test_annual_uses_the_latest_filing():
    entries = [
        _fact("2023-01-01", "2023-12-31", 90.0, "10-K", filed="2024-02-01"),
        _fact("2023-01-01", "2023-12-31", 88.0, "10-K", filed="2025-02-01"),  # restated
    ]
    assert select_annual(entries, "2023-12-31").value == 88.0


def test_fact_without_start_is_accepted():
    entries = [_fact(None, "2024-12-31", 100.0, "10-K")]
    assert select_annual(entries, "2024-12-31").value == 100.0


# --- quarter selection ---

def test_quarter_prefers_the_three_month_fact_over_year_to_date():
    entries = [
        _fact("2024-01-01", "2024-06-30", 50.0, "10-Q"),   # year to date
        _fact("2024-04-01", "2024-06-30", 26.0, "10-Q"),   # the quarter
    ]
    sel = select_quarter(entries, "2024-06-30")
    assert sel.value == 26.0
    assert sel.note == ""


def test_quarter_from_year_to_date_difference():
    # Cash-flow statements give only year-to-date values.
    entries = [
        _fact("2024-01-01", "2024-03-31", 10.0, "10-Q"),
        _fact("2024-01-01", "2024-06-30", 25.0, "10-Q"),
        _fact("2024-01-01", "2024-09-30", 45.0, "10-Q"),
    ]
    q2 = select_quarter(entries, "2024-06-30")
    q3 = select_quarter(entries, "2024-09-30")
    assert q2.value == 15.0 and q2.note == YTD_NOTE
    assert q3.value == 20.0 and q3.note == YTD_NOTE
    assert select_quarter(entries, "2024-03-31").value == 10.0


def test_quarter_is_none_when_the_prior_year_to_date_is_absent():
    entries = [_fact("2024-01-01", "2024-09-30", 45.0, "10-Q")]
    assert select_quarter(entries, "2024-09-30") is None


# --- instants ---

def test_instant_selects_by_end_date_and_form():
    entries = [
        _fact(None, "2024-12-31", 500.0, "10-K"),
        _fact(None, "2024-12-31", 501.0, "10-Q", filed="2025-05-01"),  # comparative column
        _fact(None, "2024-09-30", 480.0, "10-Q"),
    ]
    assert select(entries, "2024-12-31", ANNUAL_FORMS, "instant").value == 500.0
    assert select(entries, "2024-09-30", QUARTERLY_FORMS, "instant").value == 480.0
    assert select(entries, "2024-06-30", QUARTERLY_FORMS, "instant") is None


# --- period detection ---

def test_period_ends_annual_has_no_quarter_rows():
    facts = _facts([
        _fact("2023-01-01", "2023-12-31", 90.0, "10-K"),
        _fact("2024-01-01", "2024-12-31", 100.0, "10-K"),
        _fact("2024-07-01", "2024-09-30", 26.0, "10-K"),   # quarter data in a 10-K
        _fact("2024-10-01", "2024-12-31", 27.0, "10-K"),
    ])
    assert period_ends(facts, ["us-gaap/Revenues"], ANNUAL_FORMS) == ["2023-12-31", "2024-12-31"]


def test_period_ends_quarterly_has_no_year_to_date_only_rows():
    facts = _facts([
        _fact("2024-01-01", "2024-03-31", 24.0, "10-Q"),
        _fact("2024-04-01", "2024-06-30", 26.0, "10-Q"),
        _fact("2024-01-01", "2024-06-30", 50.0, "10-Q"),
    ])
    assert period_ends(facts, ["us-gaap/Revenues"], QUARTERLY_FORMS) == ["2024-03-31", "2024-06-30"]


def test_period_ends_keeps_the_latest_n():
    facts = _facts([_fact(f"{y}-01-01", f"{y}-12-31", 1.0, "10-K") for y in range(2015, 2025)])
    ends = period_ends(facts, ["us-gaap/Revenues"], ANNUAL_FORMS, n=3)
    assert ends == ["2022-12-31", "2023-12-31", "2024-12-31"]


def test_end_dates_that_are_near_each_other_are_one_period():
    # Two filings give the fiscal year 2016 two end dates, one day apart.
    facts = {"facts": {"us-gaap": {
        "Revenues": {"units": {"USD": [
            _fact("2015-11-02", "2016-10-30", 100.0, "10-K"),
            _fact("2015-11-01", "2016-10-31", 100.0, "10-K"),
            _fact("2016-10-31", "2017-10-29", 110.0, "10-K"),
        ]}},
        "NetIncomeLoss": {"units": {"USD": [_fact("2015-11-02", "2016-10-30", 9.0, "10-K")]}},
    }}}
    ends = period_ends(facts, ["us-gaap/Revenues", "us-gaap/NetIncomeLoss"], ANNUAL_FORMS)
    assert ends == ["2016-10-30", "2017-10-29"]   # the date with more anchor facts stays
