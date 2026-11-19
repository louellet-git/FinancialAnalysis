import json
from datetime import date

import pytest

from financial_analysis.market_data import load_prices
from financial_analysis.schema import PricePoint


def test_load_prices_reads_the_cache_file(tmp_path):
    cache_file = tmp_path / "NVDA" / "prices.json"
    cache_file.parent.mkdir(parents=True)
    cache_file.write_text(json.dumps([
        {"date": "2024-01-02", "open": 100.0, "high": 105.0, "low": 99.0, "close": 103.0, "volume": 1_000_000}
    ]), encoding="utf-8")

    result = load_prices("nvda", cache_dir=tmp_path)

    assert result == [PricePoint(date(2024, 1, 2), 100.0, 105.0, 99.0, 103.0, 1_000_000)]


def test_load_prices_without_a_cache_file_says_how_to_make_it(tmp_path):
    with pytest.raises(FileNotFoundError, match="fa analyze NVDA"):
        load_prices("NVDA", cache_dir=tmp_path)
