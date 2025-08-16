#!/usr/bin/env python3
# tools/rs_sweep_summary_simple.py
import csv, sys, math
from collections import defaultdict

if len(sys.argv) < 2:
    print("Usage: rs_sweep_summary_simple.py rs_sweep.csv")
    sys.exit(1)

path = sys.argv[1]
groups = defaultdict(lambda: {"trials":0,"successes":0,"total_recovered":0})

with open(path, newline='') as f:
    reader = csv.DictReader(f)
    for row in reader:
        try:
            bp = float(row.get("burst_prob", row.get("burst","0")))
            fp = float(row.get("flip_prob", row.get("flip","0")))
            success = int(row.get("success", row.get("ok", "0")))
            recovered_len = int(row.get("recovered_len", row.get("recovered", "0")))
        except Exception as e:
            continue
        key = (round(bp,6), round(fp,6))
        groups[key]["trials"] += 1
        groups[key]["successes"] += success
        groups[key]["total_recovered"] += recovered_len

print("burst_prob,flip_prob,trials,successes,success_rate%,avg_recovered_len")
for (bp,fp), stats in sorted(groups.items()):
    trials = stats["trials"]
    succ = stats["successes"]
    avg_rec = stats["total_recovered"] / trials if trials>0 else 0
    rate = (succ / trials * 100.0) if trials>0 else 0.0
    print(f"{bp:.6f},{fp:.6f},{trials},{succ},{rate:.2f},{avg_rec:.1f}")
