from __future__ import annotations

import csv
import math
from dataclasses import dataclass
from typing import Any

import numpy as np

from workload import Workload


def _proportion_to_int_sum(values: np.ndarray, target: int) -> np.ndarray:
    """
    Integer counts proportional to `values`, summing exactly to `target`.
    Preserves all-zeros when target is 0.
    """
    values = np.asarray(values, dtype=np.int64)
    if values.size == 0:
        return values.copy()
    s = int(values.sum())
    if target == 0:
        return np.zeros_like(values, dtype=np.int64)
    if s == 0:
        raise ValueError("cannot scale zero-sum counts to positive target")
    if target == s:
        return values.copy()
    prop = values.astype(np.float64) * (target / s)
    out = np.floor(prop).astype(np.int64)
    rem = int(target - out.sum())
    if rem > 0:
        flat_out = out.ravel()
        flat_frac = (prop - out).ravel()
        order = np.argsort(-flat_frac, kind="stable")
        for k in range(rem):
            flat_out[int(order[k % len(order)])] += 1
    return out


def apply_benchmark_volume_caps(
    rows: list[SpecificationRow],
    caps: dict[str, float],
) -> list[SpecificationRow]:
    """
    Enforce upper bounds on each listed benchmark's *share* of total
    invocations (sum of all minute columns across all rows).

    For each cap (processed in dict / insertion order), if benchmark B's
    current share exceeds ``max_frac``, scale B's minute counts down so its
    new total is ``floor(max_frac * T)`` (T = grand total invocations), and
    scale all other rows' minutes up proportionally so the grand total stays T.

    Rows are unchanged when the benchmark is already at or under the cap.
    """
    if not caps:
        return rows

    out: list[SpecificationRow] = list(rows)
    for bench, max_frac in caps.items():
        if max_frac <= 0.0 or max_frac >= 1.0:
            raise ValueError(
                f"volume cap for {bench!r}: max_frac must be in (0, 1), got {max_frac}"
            )
        out = _cap_single_benchmark_share(out, bench, max_frac)
    return out


def _cap_single_benchmark_share(
    rows: list[SpecificationRow], bench: str, max_frac: float
) -> list[SpecificationRow]:
    if not rows:
        return rows

    arrays = [np.asarray(r.minutes, dtype=np.int64) for r in rows]
    mask = [r.workload.benchmark == bench for r in rows]
    G = int(sum(arrays[i].sum() for i in range(len(rows)) if mask[i]))
    T = int(sum(a.sum() for a in arrays))
    if T == 0:
        return rows
    if G <= max_frac * T + 1e-9:
        return rows

    target_g = int(math.floor(max_frac * T + 1e-9))
    target_o = T - target_g
    other_sum = T - G
    if other_sum <= 0 and target_o > 0:
        raise ValueError(
            f"cannot apply cap {bench!r} at {max_frac}: no other benchmarks to absorb load"
        )

    # Build new minute arrays
    new_arrays = [a.copy() for a in arrays]
    g_idx = [i for i, m in enumerate(mask) if m]
    o_idx = [i for i, m in enumerate(mask) if not m]

    if G > 0 and g_idx:
        cat = np.concatenate([arrays[i].ravel() for i in g_idx])
        new_cat = _proportion_to_int_sum(cat, target_g)
        if len(g_idx) == 1:
            only = g_idx[0]
            new_arrays[only] = new_cat.reshape(arrays[only].shape)
        else:
            splits = np.cumsum([arrays[i].size for i in g_idx[:-1]])
            parts = np.split(new_cat, splits)
            for i, part in zip(g_idx, parts):
                new_arrays[i] = part.reshape(arrays[i].shape)
    elif g_idx:
        for i in g_idx:
            new_arrays[i].fill(0)

    if other_sum > 0 and o_idx:
        cat_o = np.concatenate([arrays[i].ravel() for i in o_idx])
        new_cat_o = _proportion_to_int_sum(cat_o, target_o)
        if len(o_idx) == 1:
            only_o = o_idx[0]
            new_arrays[only_o] = new_cat_o.reshape(arrays[only_o].shape)
        else:
            splits_o = np.cumsum([arrays[i].size for i in o_idx[:-1]])
            parts_o = np.split(new_cat_o, splits_o)
            for i, part in zip(o_idx, parts_o):
                new_arrays[i] = part.reshape(arrays[i].shape)
    elif o_idx and target_o == 0:
        for i in o_idx:
            new_arrays[i].fill(0)

    return [
        SpecificationRow(r.trace_exec_time, r.workload, new_arrays[j].tolist())
        for j, r in enumerate(rows)
    ]


@dataclass(frozen=True)
class SpecificationRow:
    trace_exec_time: float
    workload: Workload
    minutes: list[int]

    def to_list(self) -> list[Any]:
        return [self.trace_exec_time, str(self.workload)] + self.minutes


@dataclass(frozen=True)
class Specification:
    headers: list[str]
    sorted_rows: list[SpecificationRow]

    def to_csv(self, fout) -> None:  # fout: _typeshed.SupportsWrite
        """
        Export this experiment specification instance as a CSV file.

        :param fout: any object with a `write()` method
        """
        writer = csv.writer(fout)
        writer.writerow(self.headers)
        for row in self.sorted_rows:
            writer.writerow(row.to_list())
