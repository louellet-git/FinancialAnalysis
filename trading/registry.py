"""Discover bots: every non-underscore module in ``trading/strategies`` exporting ``STRATEGY``."""
from __future__ import annotations

import importlib
import pkgutil
from typing import Dict, Type

from trading.strategy import Strategy


def discover(include_private: bool = False) -> Dict[str, Type[Strategy]]:
    import trading.strategies as pkg

    found: Dict[str, Type[Strategy]] = {}
    for mod in pkgutil.iter_modules(pkg.__path__):
        if mod.name.startswith("_") and not include_private:
            continue
        module = importlib.import_module(f"{pkg.__name__}.{mod.name}")
        cls = getattr(module, "STRATEGY", None)
        if cls is None:
            continue
        name = cls.name or mod.name
        if name in found:
            raise ValueError(f"duplicate strategy name {name!r}")
        found[name] = cls
    return dict(sorted(found.items()))
