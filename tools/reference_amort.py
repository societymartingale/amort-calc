"""Independent amortization reference used for src/golden.rs.

Run: pip install numpy-financial && python tools/reference_amort.py
50-digit Decimal implementation + numpy-financial cross-check.
"""
import json, sys
from decimal import Decimal as D, getcontext, ROUND_HALF_UP
import numpy_financial as npf
getcontext().prec = 50
C = D("0.01")
r2 = lambda x: x.quantize(C, rounding=ROUND_HALF_UP)

SCEN = [("306000","3.5",360),("100000","6",360),("200000","4.5",180),("250000","7",360),
        ("1000","0",3),("25000","5.99",60),("1000","12",1),("100000","0.001",360),
        ("100000","99",1200),("1000000000000","30",360),("5000","18",24),("0.05","0",10)]

def ref(L, apr, n):
    L, i = D(L), D(apr) / 1200
    exact = L / n if i == 0 else L * i / (1 - (1 + i) ** -n)
    pmt = r2(exact); bal = r2(L); rows = []
    if i and pmt <= r2(bal * i):  # would never amortize: next cent up
        pmt += C
    for k in range(1, n + 1):
        it = r2(bal * i); payoff = bal + it
        if k == n or payoff <= pmt:
            rows.append((k, bal, payoff, it, bal, D(0))); break
        pr = pmt - it; rows.append((k, bal, pmt, it, pr, bal - pr)); bal -= pr
    f = lambda r: [str(x) for x in r]
    npf_pmt = -npf.pmt(float(apr) / 1200, n, float(L)) if float(apr) else float(L) / n
    npf_int = -sum(npf.ipmt(float(apr)/1200, range(1, n+1), n, float(L))) if float(apr) else 0.0
    return dict(scen=[str(L), apr, n], exact_pmt=str(exact.quantize(D("1e-10"))), pmt=str(pmt),
                total_int=str(sum(r[3] for r in rows)), exact_total_int=str(r2(exact * n - L)),
                npf_pmt=f"{npf_pmt:.6f}", npf_total_int=f"{npf_int:.2f}",
                n_rows=len(rows), first=f(rows[0]), last=f(rows[-1]))
json.dump([ref(*s) for s in SCEN], sys.stdout, indent=1)
