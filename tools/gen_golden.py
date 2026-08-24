#!/usr/bin/env python3
"""Generate golden benchmark JSON for the smartpacker crate.

Runs the reference Python implementation (py3dbp) across a curated set of
scenarios and serializes both the *input* (pre-pack) and *expected* (post-pack)
state to `smartpacker/tests/golden/<name>.json`.

Usage:
    PY3DBP_PATH=/path/to/3D-bin-packing-master python3 tools/gen_golden.py
"""

import os
import sys

sys.dont_write_bytecode = True

import json
from decimal import Decimal

DEFAULT_PY3DBP = "/Users/nigang/Downloads/3D-bin-packing-master"
PY3DBP = os.environ.get("PY3DBP_PATH", DEFAULT_PY3DBP)
sys.path.insert(0, PY3DBP)

import matplotlib
matplotlib.use("Agg")  # headless; no display dependency

from py3dbp import Packer, Bin, Item  # noqa: E402

OUT_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                       "smartpacker", "tests", "golden")


def f(v):
    """Convert Decimal/float/int to a plain float (JSON-safe)."""
    if isinstance(v, Decimal):
        return float(v)
    return float(v)


def item_input(it):
    return {
        "partno": it.partno,
        "name": it.name,
        "typeof": it.typeof,
        "whd": [it.width, it.height, it.depth],
        "weight": it.weight,
        "level": it.level,
        "loadbear": it.loadbear,
        "updown": it.updown,
        "color": it.color,
    }


def item_expected(it):
    return {
        "partno": it.partno,
        "name": it.name,
        "typeof": it.typeof,
        "width": f(it.width),
        "height": f(it.height),
        "depth": f(it.depth),
        "weight": f(it.weight),
        "color": it.color,
        "rotation_type": int(it.rotation_type),
        "position": [f(it.position[0]), f(it.position[1]), f(it.position[2])],
    }


def bin_input(b):
    return {
        "partno": b.partno,
        "whd": [b.width, b.height, b.depth],
        "max_weight": b.max_weight,
        "corner": b.corner,
        "put_type": b.put_type,
    }


def bin_expected(b):
    return {
        "partno": b.partno,
        "width": f(b.width),
        "height": f(b.height),
        "depth": f(b.depth),
        "max_weight": f(b.max_weight),
        "corner": f(b.corner),
        "put_type": int(b.put_type),
        "gravity": [f(g) for g in b.gravity],
        "items": [item_expected(i) for i in b.items],
        "unfitted_items": [item_expected(i) for i in b.unfitted_items],
    }


def build(bins, items, options):
    packer = Packer()
    for b in bins:
        packer.addBin(b)
    for it in items:
        packer.addItem(it)

    name = options.pop("_name", "")

    # Input snapshot: capture BEFORE pack (raw values, insertion order).
    input_snapshot = {
        "bins": [bin_input(b) for b in packer.bins],
        "items": [item_input(it) for it in packer.items],
    }

    packer.pack(**options)

    expected = {
        "bins": [bin_expected(b) for b in packer.bins],
        "unfit_items": [item_expected(it) for it in packer.unfit_items],
    }

    options_out = {
        "bigger_first": bool(options.get("bigger_first", False)),
        "distribute_items": bool(options.get("distribute_items", True)),
        "fix_point": bool(options.get("fix_point", True)),
        "check_stable": bool(options.get("check_stable", True)),
        "support_surface_ratio": f(options.get("support_surface_ratio", 0.75)),
        "number_of_decimals": int(options.get("number_of_decimals", 0)),
        "binding": [list(g) for g in options.get("binding", [])],
    }

    return {
        "name": name,
        "options": options_out,
        "input": input_snapshot,
        "expected": expected,
    }


def scenario_readme_simple():
    bins = [Bin("example", (30, 10, 15), 99, 0, 1)]
    items = [
        Item("test1", "test", "cube", (9, 8, 7), 1, 1, 100, True, "red"),
        Item("test2", "test", "cube", (4, 25, 1), 1, 1, 100, True, "blue"),
        Item("test3", "test", "cube", (2, 13, 5), 1, 1, 100, True, "gray"),
        Item("test4", "test", "cube", (7, 5, 4), 1, 1, 100, True, "orange"),
        Item("test5", "test", "cube", (10, 5, 2), 1, 1, 100, True, "lawngreen"),
    ]
    opts = dict(_name="readme_simple", bigger_first=True, distribute_items=True,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, items, opts


def scenario_ex0_fix_on():
    bins = [Bin("example0", (589.8, 243.8, 259.1), 28080, 15, 0)]
    items = []
    for i in range(5):
        items.append(Item("Dyson DC34 Animal%d" % (i + 1), "Dyson", "cube",
                          (170, 82, 46), 85.12, 1, 100, True, "#FF0000"))
    for i in range(10):
        items.append(Item("wash%d" % (i + 1), "wash", "cube", (85, 60, 60), 10, 1, 100, True, "#FFFF37"))
    for i in range(5):
        items.append(Item("Cabinet%d" % (i + 1), "cabint", "cube", (60, 80, 200), 80, 1, 100, True, "#842B00"))
    for i in range(10):
        items.append(Item("Server%d" % (i + 1), "server", "cube", (70, 100, 30), 20, 1, 100, True, "#0000E3"))
    opts = dict(_name="ex0_fix_on", bigger_first=True, distribute_items=False,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, items, opts


def scenario_ex0_fix_off():
    bins = [Bin("example0", (589.8, 243.8, 259.1), 28080, 15, 0)]
    items = []
    for i in range(5):
        items.append(Item("Dyson DC34 Animal%d" % (i + 1), "Dyson", "cube",
                          (170, 82, 46), 85.12, 1, 100, True, "#FF0000"))
    for i in range(10):
        items.append(Item("wash%d" % (i + 1), "wash", "cube", (85, 60, 60), 10, 1, 100, True, "#FFFF37"))
    for i in range(5):
        items.append(Item("Cabinet%d" % (i + 1), "cabint", "cube", (60, 80, 200), 80, 1, 100, True, "#842B00"))
    for i in range(10):
        items.append(Item("Server%d" % (i + 1), "server", "cube", (70, 100, 30), 20, 1, 100, True, "#0000E3"))
    opts = dict(_name="ex0_fix_off", bigger_first=True, distribute_items=False,
                fix_point=False, check_stable=False, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, items, opts


def scenario_ex1_cylinder():
    bins = [Bin("example1", (5.6875, 10.75, 15.0), 70.0, 0, 0)]
    items = [
        Item("50g [powder 1]", "test", "cube", (2, 2, 4), 1, 1, 100, True, "red"),
        Item("50g [powder 2]", "test", "cube", (2, 2, 4), 2, 1, 100, True, "blue"),
        Item("50g [powder 3]", "test", "cube", (2, 2, 4), 3, 1, 100, True, "gray"),
        Item("50g [powder 4]", "test", "cube", (2, 2, 4), 3, 1, 100, True, "orange"),
        Item("50g [powder 5]", "test", "cylinder", (2, 2, 4), 3, 1, 100, True, "lawngreen"),
        Item("50g [powder 6]", "test", "cylinder", (2, 2, 4), 3, 1, 100, True, "purple"),
        Item("50g [powder 7]", "test", "cylinder", (1, 1, 5), 3, 1, 100, True, "yellow"),
        Item("250g [powder 8]", "test", "cylinder", (4, 4, 2), 4, 1, 100, True, "pink"),
        Item("250g [powder 9]", "test", "cylinder", (4, 4, 2), 5, 1, 100, True, "brown"),
        Item("250g [powder 10]", "test", "cube", (4, 4, 2), 6, 1, 100, True, "cyan"),
        Item("250g [powder 11]", "test", "cylinder", (4, 4, 2), 7, 1, 100, True, "olive"),
        Item("250g [powder 12]", "test", "cylinder", (4, 4, 2), 8, 1, 100, True, "darkgreen"),
        Item("250g [powder 13]", "test", "cube", (4, 4, 2), 9, 1, 100, True, "orange"),
    ]
    opts = dict(_name="ex1_cylinder", bigger_first=True, distribute_items=False,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, items, opts


def scenario_ex2_complex():
    bins = [Bin("example2", (30, 10, 15), 99, 0, 1)]
    whd = [(9, 8, 7), (4, 25, 1), (2, 13, 5), (7, 5, 4), (10, 5, 2), (6, 5, 2),
           (5, 2, 9), (10, 8, 5), (1, 3, 5), (8, 4, 7), (2, 5, 3), (1, 9, 2),
           (7, 5, 4), (10, 2, 1), (3, 2, 4), (5, 7, 8), (4, 8, 3), (2, 11, 5),
           (8, 3, 5), (7, 4, 5), (2, 4, 11), (1, 3, 4), (10, 5, 2), (7, 4, 5),
           (2, 10, 3), (3, 8, 1), (7, 2, 5), (8, 9, 5), (4, 5, 10), (10, 10, 2)]
    colors = ["red", "blue", "gray", "orange", "lawngreen", "purple", "yellow",
              "pink", "brown", "cyan", "olive", "darkgreen", "orange", "lawngreen",
              "purple", "yellow", "white", "brown", "cyan", "olive", "darkgreen",
              "orange", "lawngreen", "purple", "yellow", "pink", "brown", "cyan",
              "olive", "darkgreen"]
    items = [Item("test%d" % (i + 1), "test", "cube", whd[i], 1, 1, 100, True, colors[i])
             for i in range(30)]
    opts = dict(_name="ex2_complex", bigger_first=True, distribute_items=True,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, items, opts


def scenario_ex3():
    bins = [Bin("example3", (6, 1, 5), 100, 0, 0)]
    items = [
        Item("Box-1", "test", "cube", (2, 1, 3), 1, 1, 100, True, "yellow"),
        Item("Box-2", "test", "cube", (3, 1, 2), 1, 1, 100, True, "pink"),
        Item("Box-3", "test", "cube", (2, 1, 3), 1, 1, 100, True, "brown"),
        Item("Box-4", "test", "cube", (2, 1, 3), 1, 1, 100, True, "cyan"),
        Item("Box-5", "test", "cube", (2, 1, 3), 1, 1, 100, True, "olive"),
    ]
    opts = dict(_name="ex3", bigger_first=True, distribute_items=False,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, items, opts


def scenario_ex4_batch():
    bins = [Bin("example4", (589.8, 243.8, 259.1), 28080, 15, 0)]
    items = []
    for i in range(15):
        items.append(Item("Dyson DC34 Animal%d" % (i + 1), "Dyson", "cube",
                          (170, 82, 46), 85.12, 1, 100, True, "#FF0000"))
    for i in range(18):
        items.append(Item("wash%d" % (i + 1), "wash", "cube", (85, 60, 60), 10, 1, 100, True, "#FFFF37"))
    for i in range(15):
        items.append(Item("Cabinet%d" % (i + 1), "cabint", "cube", (60, 80, 200), 80, 1, 100, True, "#842B00"))
    for i in range(42):
        items.append(Item("Server%d" % (i + 1), "server", "cube", (70, 100, 30), 20, 1, 100, True, "#0000E3"))
    opts = dict(_name="ex4_batch", bigger_first=True, distribute_items=False,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, items, opts


def scenario_ex5_stable():
    bins = [Bin("example5", (5, 4, 3), 100, 0, 0)]
    items = [
        Item("Box-3", "test", "cube", (2, 5, 2), 1, 1, 100, True, "pink"),
        Item("Box-3", "test", "cube", (2, 3, 2), 1, 2, 100, True, "pink"),
        Item("Box-4", "test", "cube", (5, 4, 1), 1, 3, 100, True, "brown"),
    ]
    opts = dict(_name="ex5_stable", bigger_first=True, distribute_items=False,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, items, opts


def scenario_ex6_stable():
    bins = [Bin("example6", (5, 4, 7), 100, 0, 0)]
    items = [
        Item("Box-1", "test", "cube", (5, 4, 1), 1, 1, 100, True, "yellow"),
        Item("Box-2", "test", "cube", (1, 1, 4), 1, 2, 100, True, "olive"),
        Item("Box-3", "test", "cube", (3, 4, 2), 1, 3, 100, True, "pink"),
        Item("Box-4", "test", "cube", (1, 1, 4), 1, 4, 100, True, "olive"),
        Item("Box-5", "test", "cube", (1, 2, 1), 1, 5, 100, True, "pink"),
        Item("Box-6", "test", "cube", (1, 2, 1), 1, 6, 100, True, "pink"),
        Item("Box-7", "test", "cube", (1, 1, 4), 1, 7, 100, True, "olive"),
        Item("Box-8", "test", "cube", (1, 1, 4), 1, 8, 100, True, "olive"),
        Item("Box-9", "test", "cube", (5, 4, 2), 1, 9, 100, True, "brown"),
    ]
    opts = dict(_name="ex6_stable", bigger_first=True, distribute_items=False,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, items, opts


def _example7_items():
    items = [
        Item("Box-1", "test1", "cube", (5, 4, 1), 1, 1, 100, True, "yellow"),
        Item("Box-2", "test2", "cube", (1, 2, 4), 1, 1, 100, True, "olive"),
        Item("Box-3", "test3", "cube", (1, 2, 3), 1, 1, 100, True, "olive"),
        Item("Box-4", "test4", "cube", (1, 2, 2), 1, 1, 100, True, "olive"),
        Item("Box-5", "test5", "cube", (1, 2, 3), 1, 1, 100, True, "olive"),
        Item("Box-6", "test6", "cube", (1, 2, 4), 1, 1, 100, True, "olive"),
        Item("Box-7", "test7", "cube", (1, 2, 2), 1, 1, 100, True, "olive"),
        Item("Box-8", "test8", "cube", (1, 2, 3), 1, 1, 100, True, "olive"),
        Item("Box-9", "test9", "cube", (1, 2, 4), 1, 1, 100, True, "olive"),
        Item("Box-10", "test10", "cube", (1, 2, 3), 1, 1, 100, True, "olive"),
        Item("Box-11", "test11", "cube", (1, 2, 2), 1, 1, 100, True, "olive"),
        Item("Box-12", "test12", "cube", (5, 4, 1), 1, 1, 100, True, "pink"),
        Item("Box-13", "test13", "cube", (1, 1, 4), 1, 1, 100, True, "olive"),
        Item("Box-14", "test14", "cube", (1, 2, 1), 1, 1, 100, True, "pink"),
        Item("Box-15", "test15", "cube", (1, 2, 1), 1, 1, 100, True, "pink"),
        Item("Box-16", "test16", "cube", (1, 1, 4), 1, 1, 100, True, "olive"),
        Item("Box-17", "test17", "cube", (1, 1, 4), 1, 1, 100, True, "olive"),
        Item("Box-18", "test18", "cube", (5, 4, 2), 1, 1, 100, True, "brown"),
    ]
    return items


def scenario_ex7_dist_false():
    bins = [Bin("example7-Bin1", (5, 5, 5), 100, 0, 0),
            Bin("example7-Bin2", (3, 3, 5), 100, 0, 0)]
    opts = dict(_name="ex7_dist_false", bigger_first=True, distribute_items=False,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, _example7_items(), opts


def scenario_ex7_dist_true():
    bins = [Bin("example7-Bin1", (5, 5, 5), 100, 0, 0),
            Bin("example7-Bin2", (3, 3, 5), 100, 0, 0)]
    opts = dict(_name="ex7_dist_true", bigger_first=True, distribute_items=True,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, _example7_items(), opts


def scenario_edge_bigger_first_false():
    bins = [Bin("example", (30, 10, 15), 99, 0, 1)]
    items = [
        Item("test1", "test", "cube", (9, 8, 7), 1, 1, 100, True, "red"),
        Item("test2", "test", "cube", (4, 25, 1), 1, 1, 100, True, "blue"),
        Item("test3", "test", "cube", (2, 13, 5), 1, 1, 100, True, "gray"),
        Item("test4", "test", "cube", (7, 5, 4), 1, 1, 100, True, "orange"),
        Item("test5", "test", "cube", (10, 5, 2), 1, 1, 100, True, "lawngreen"),
    ]
    opts = dict(_name="edge_bigger_first_false", bigger_first=False, distribute_items=True,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, items, opts


def scenario_edge_updown_false():
    bins = [Bin("edge_updown", (20, 20, 20), 1000, 0, 1)]
    items = [
        Item("u1", "test", "cube", (5, 8, 3), 1, 1, 100, False, "red"),
        Item("u2", "test", "cube", (6, 4, 2), 1, 1, 100, False, "blue"),
        Item("u3", "test", "cube", (3, 9, 4), 1, 1, 100, False, "gray"),
        Item("u4", "test", "cube", (4, 4, 5), 1, 1, 100, False, "orange"),
        Item("u5", "test", "cube", (9, 2, 2), 1, 1, 100, False, "lawngreen"),
    ]
    opts = dict(_name="edge_updown_false", bigger_first=True, distribute_items=True,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, items, opts


def scenario_edge_weight_limit():
    bins = [Bin("edge_weight", (20, 20, 20), 50, 0, 1)]
    items = [
        Item("w1", "test", "cube", (5, 5, 5), 20, 1, 100, True, "red"),
        Item("w2", "test", "cube", (5, 5, 5), 20, 1, 100, True, "blue"),
        Item("w3", "test", "cube", (5, 5, 5), 20, 1, 100, True, "gray"),
    ]
    opts = dict(_name="edge_weight_limit", bigger_first=True, distribute_items=True,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0)
    return bins, items, opts


def scenario_edge_decimals1():
    bins = [Bin("edge_dec", (10.5, 8.2, 6.4), 50, 0, 1)]
    items = [
        Item("d1", "test", "cube", (3.2, 2.5, 1.8), 5.5, 1, 100, True, "red"),
        Item("d2", "test", "cube", (2.5, 3.2, 2.5), 5.5, 1, 100, True, "blue"),
        Item("d3", "test", "cube", (4.1, 2.5, 2.5), 6.5, 1, 100, True, "gray"),
        Item("d4", "test", "cube", (2.5, 2.5, 3.8), 5.5, 1, 100, True, "orange"),
    ]
    opts = dict(_name="edge_decimals1", bigger_first=True, distribute_items=True,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=1)
    return bins, items, opts


def scenario_binding():
    bins = [Bin("binding", (30, 20, 15), 1000, 0, 1)]
    items = [
        Item("a1", "apple", "cube", (5, 5, 5), 1, 1, 100, True, "red"),
        Item("a2", "apple", "cube", (5, 5, 5), 1, 1, 100, True, "red"),
        Item("o1", "orange", "cube", (4, 4, 4), 1, 1, 100, True, "blue"),
        Item("o2", "orange", "cube", (4, 4, 4), 1, 1, 100, True, "blue"),
        Item("free1", "free", "cube", (6, 6, 6), 1, 1, 100, True, "green"),
    ]
    opts = dict(_name="binding", bigger_first=True, distribute_items=False,
                fix_point=True, check_stable=True, support_surface_ratio=0.75,
                number_of_decimals=0, binding=[("apple", "orange")])
    return bins, items, opts


SCENARIOS = [
    scenario_readme_simple,
    scenario_ex0_fix_on,
    scenario_ex0_fix_off,
    scenario_ex1_cylinder,
    scenario_ex2_complex,
    scenario_ex3,
    scenario_ex4_batch,
    scenario_ex5_stable,
    scenario_ex6_stable,
    scenario_ex7_dist_false,
    scenario_ex7_dist_true,
    scenario_edge_bigger_first_false,
    scenario_edge_updown_false,
    scenario_edge_weight_limit,
    scenario_edge_decimals1,
    scenario_binding,
]


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    for sc in SCENARIOS:
        bins, items, opts = sc()
        doc = build(bins, items, opts)
        name = doc["name"]
        path = os.path.join(OUT_DIR, name + ".json")
        with open(path, "w") as fh:
            json.dump(doc, fh, indent=2)
            fh.write("\n")
        print("wrote %s (%d bins, %d items)" % (path, len(doc["expected"]["bins"]),
                                                len(doc["input"]["items"])))


if __name__ == "__main__":
    main()