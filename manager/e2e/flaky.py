"""Run `cargo test --workspace` N times and report every test that failed at least once.
Usage: e2e/run.sh flaky [N]   (plain python3 is enough)"""

import collections
import os
import re
import subprocess
import sys

n = int(sys.argv[1]) if len(sys.argv) > 1 else 10
root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
fails = collections.defaultdict(list)  # test -> [(run, message)]
crashes = []
for i in range(1, n + 1):
    p = subprocess.run(["cargo", "test", "--workspace", "--no-fail-fast"], cwd=root, capture_output=True, text=True)
    out = p.stdout + "\n" + p.stderr
    failed = set(re.findall(r"^test (\S+) \.\.\. FAILED", out, re.M))
    for t in failed:
        m = re.search(r"---- " + re.escape(t) + r" stdout ----\n(.*?)(?:\n\n|\n----|\nfailures:)", out, re.S)
        fails[t].append((i, (m.group(1).strip() if m else "?")[:400]))
    if p.returncode != 0 and not failed:
        crashes.append((i, out[-600:]))
    print(f"run {i}/{n}: rc={p.returncode} failed={sorted(failed)}", flush=True)
print("\n===== FLAKY REPORT =====")
if not fails and not crashes:
    print(f"no test failed in {n} runs")
for t, rs in fails.items():
    print(f"{t}: failed {len(rs)}/{n} (runs {[r for r, _ in rs]})\n    {rs[0][1]}")
for i, o in crashes:
    print(f"run {i}: non-zero exit without a failed test (build/linker/crash):\n{o}")
sys.exit(1 if fails or crashes else 0)
