import sys, traceback
sys.path.insert(0, "/Users/nigang/Downloads/3D-bin-packing-master")
import matplotlib
matplotlib.use("Agg")
from py3dbp import Packer, Bin, Item

def run(binding, items, bins, **kw):
    p = Packer()
    for b in bins:
        p.addBin(b)
    for it in items:
        p.addItem(it)
    p.pack(**kw)
    return p

# scenario A: example4-like binding
try:
    bins = [Bin("b1", (589.8,243.8,259.1), 28080, 15, 0)]
    items = []
    for i in range(3):
        items.append(Item("Dyson%d"%(i+1), "Dyson", "cube", (170,82,46), 85.12, 1, 100, True, "#FF0000"))
    for i in range(4):
        items.append(Item("wash%d"%(i+1), "wash", "cube", (85,60,60), 10, 1, 100, True, "#FFFF37"))
    for i in range(4):
        items.append(Item("Cab%d"%(i+1), "cabint", "cube", (60,80,200), 80, 1, 100, True, "#842B00"))
    p = run([("cabint","wash")], items, bins, bigger_first=True, distribute_items=False, fix_point=True, check_stable=True, support_surface_ratio=0.75, number_of_decimals=0)
    print("A OK bins=%d unfitted=%d" % (len(p.bins), len(p.unfit_items)))
    for b in p.bins:
        print("A bin items=%d unfitted=%d" % (len(b.items), len(b.unfitted_items)))
except Exception:
    print("A CRASH:")
    traceback.print_exc()

# scenario B: binding single group, one bin, no corner
try:
    bins = [Bin("b1", (20,20,20), 1000, 0, 1)]
    items = [Item("a1","apple","cube",(5,5,5),1,1,100,True,"red"),
             Item("a2","apple","cube",(5,5,5),1,1,100,True,"red"),
             Item("o1","orange","cube",(4,4,4),1,1,100,True,"blue"),
             Item("o2","orange","cube",(4,4,4),1,1,100,True,"blue")]
    p = run([("apple","orange")], items, bins, bigger_first=True, distribute_items=False, fix_point=True, check_stable=True, support_surface_ratio=0.75, number_of_decimals=0)
    print("B OK bins=%d" % len(p.bins))
    for b in p.bins:
        for it in b.items:
            print("B item", it.partno, it.position, it.rotation_type)
        print("B unfitted", [i.partno for i in b.unfitted_items])
except Exception:
    print("B CRASH:")
    traceback.print_exc()