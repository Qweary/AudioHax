#!/usr/bin/env python3
# tools/rs_sweep_analyze.py
import sys
import os
import pandas as pd
import matplotlib.pyplot as plt

def main(path):
    if not os.path.exists(path):
        print("File not found:", path)
        return
    df = pd.read_csv(path)

    # expected columns: burst_prob, flip_prob, trial, success (0/1), recovered_len, maybe others
    required = ['burst_prob','flip_prob','trial','success','recovered_len']
    for c in required:
        if c not in df.columns:
            print("CSV missing expected column:", c)
            print("Columns:", df.columns.tolist())
            return

    grouped = df.groupby(['burst_prob','flip_prob']).agg(
        trials=('trial','count'),
        successes=('success','sum'),
        success_rate=('success','mean'),
        avg_recovered_len=('recovered_len','mean'),
    ).reset_index()

    grouped['success_rate_pct'] = (grouped['success_rate']*100).round(2)
    grouped['avg_recovered_len'] = grouped['avg_recovered_len'].round(1)

    summary_csv = os.path.join(os.path.dirname(path), 'rs_sweep_summary.csv')
    grouped.to_csv(summary_csv, index=False)
    print("Wrote grouped summary to:", summary_csv)

    # pivot heatmap by success_rate_pct
    pivot = grouped.pivot(index='burst_prob', columns='flip_prob', values='success_rate_pct').fillna(0)
    fig, ax = plt.subplots(figsize=(10,6))
    im = ax.imshow(pivot.values, aspect='auto', origin='lower')
    ax.set_xticks(range(len(pivot.columns)))
    ax.set_xticklabels([f"{c:.5f}" for c in pivot.columns], rotation=45)
    ax.set_yticks(range(len(pivot.index)))
    ax.set_yticklabels([f"{r:.5f}" for r in pivot.index])
    ax.set_xlabel('flip_prob')
    ax.set_ylabel('burst_prob')
    ax.set_title('RS Sweep: Success Rate (%)')
    fig.colorbar(im, ax=ax, label='Success Rate (%)')
    heatmap_png = os.path.join(os.path.dirname(path), 'rs_sweep_heatmap.png')
    fig.tight_layout()
    fig.savefig(heatmap_png)
    print("Wrote heatmap to:", heatmap_png)

    # print top results
    top = grouped.sort_values(['success_rate','trials'], ascending=[False, False]).head(20)
    print("\nTop 20 combos by success rate:")
    print(top[['burst_prob','flip_prob','trials','success_rate_pct','avg_recovered_len']].to_string(index=False))

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: rs_sweep_analyze.py /path/to/rs_sweep.csv")
        sys.exit(1)
    main(sys.argv[1])
