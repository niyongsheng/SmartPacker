import sys, traceback
sys.path.insert(0, "/Users/nigang/Downloads/3D-bin-packing-master")
import matplotlib
matplotlib.use("Agg")
from py3dbp import Packer, Bin, Item

bins = [Bin("b1", (589.8,243.8,259.1), 28080, 15, 0)]
items = []
for i in range(3):
    items.append(Item("Dyson%d"%(i+1), "Dyson", "cube", (170,82,46), 85.12, 1, 100, True, "#FF0000"))
for i in range(4):
    items.append(Item("wash%d"%(i+1), "wash", "cube", (85,60,60), 10, 1, 100, True, "#FFFF37"))
for i in range(4):
    items.append(Item("Cab%d"%(i+1), "cabint", "cube", (60,80,200), 80, 1, 100, True, "#842B00"))

p = Packer()
for b in bins: p.addBin(b)
for it in items: p.addItem(it)
p.binding = [("cabint","wash")]
for b in p.bins: b.formatNumbers(0)
for it in p.items: it.formatNumbers(0)
p.items.sort(key=lambda it: it.getVolume(), reverse=True)
p.items.sort(key=lambda it: it.loadbear, reverse=True)
p.items.sort(key=lambda it: it.level, reverse=False)
print("sorted items:", [(it.partno, it.name) for it in p.items])
p.sortBinding(object())
print("after sortBinding items:", [(it.partno, it.name) for it in p.items])
print("len", len(p.items))
print("unfit at this point:", [i.partno for i in p.unfit_items])