//! Golden-value regression tests against independent references.
//!
//! REFERENCE values come from `tools/reference_amort.py`, an independent
//! 50-digit `decimal` implementation (cross-checked against numpy-financial
//! `pmt`) using the same convention as the app: payment and monthly interest
//! rounded to the cent half away from zero, last payment absorbs the rest.
//! Row arrays are [beginning balance, payment, interest, principal, ending].
//!
//! CALCULATOR_NET values were read from calculator.net's amortization
//! calculator on 2026-10-31. It rounds the payment the same way but keeps
//! unrounded balances internally and reports total interest as
//! `exact_pmt * n - loan`, so its totals differ from ours by a few cents to
//! dollars; we compare its totals against that formula instead.

use rust_decimal::prelude::*;

use crate::fin::{calc_amort_sched, checked_monthly_pmt, round_cents, total_interest};

struct Golden {
    loan: &'static str,
    apr: &'static str,
    n: u64,
    pmt: &'static str,
    total_int: &'static str,
    n_rows: usize,
    first: [&'static str; 5],
    last: [&'static str; 5],
}

#[rustfmt::skip]
const REFERENCE: &[Golden] = &[
    Golden { loan: "306000", apr: "3.5", n: 360, pmt: "1374.08", total_int: "188666.86", n_rows: 360, first: ["306000.00", "1374.08", "892.50", "481.58", "305518.42"], last: ["1368.15", "1372.14", "3.99", "1368.15", "0"] },
    Golden { loan: "100000", apr: "6", n: 360, pmt: "599.55", total_int: "115838.45", n_rows: 360, first: ["100000.00", "599.55", "500.00", "99.55", "99900.45"], last: ["597.01", "600.00", "2.99", "597.01", "0"] },
    Golden { loan: "200000", apr: "4.5", n: 180, pmt: "1529.99", total_int: "75397.32", n_rows: 180, first: ["200000.00", "1529.99", "750.00", "779.99", "199220.01"], last: ["1523.40", "1529.11", "5.71", "1523.40", "0"] },
    Golden { loan: "250000", apr: "7", n: 360, pmt: "1663.26", total_int: "348769.07", n_rows: 360, first: ["250000.00", "1663.26", "1458.33", "204.93", "249795.07"], last: ["1649.11", "1658.73", "9.62", "1649.11", "0"] },
    Golden { loan: "1000", apr: "0", n: 3, pmt: "333.33", total_int: "0.00", n_rows: 3, first: ["1000.00", "333.33", "0.00", "333.33", "666.67"], last: ["333.34", "333.34", "0.00", "333.34", "0"] },
    Golden { loan: "25000", apr: "5.99", n: 60, pmt: "483.20", total_int: "3992.25", n_rows: 60, first: ["25000.00", "483.20", "124.79", "358.41", "24641.59"], last: ["481.05", "483.45", "2.40", "481.05", "0"] },
    Golden { loan: "1000", apr: "12", n: 1, pmt: "1010.00", total_int: "10.00", n_rows: 1, first: ["1000.00", "1010.00", "10.00", "1000.00", "0"], last: ["1000.00", "1010.00", "10.00", "1000.00", "0"] },
    Golden { loan: "100000", apr: "0.001", n: 360, pmt: "277.82", total_int: "15.03", n_rows: 360, first: ["100000.00", "277.82", "0.08", "277.74", "99722.26"], last: ["277.65", "277.65", "0.00", "277.65", "0"] },
    Golden { loan: "100000", apr: "99", n: 1200, pmt: "8250.00", total_int: "9900000.00", n_rows: 1200, first: ["100000.00", "8250.00", "8250.00", "0.00", "100000.00"], last: ["100000.00", "108250.00", "8250.00", "100000.00", "0"] },
    Golden { loan: "1000000000000", apr: "30", n: 360, pmt: "25003446738.53", total_int: "8001240826960.53", n_rows: 360, first: ["1000000000000.00", "25003446738.53", "25000000000.00", "3446738.53", "999996553261.47"], last: ["24393607637.33", "25003447828.26", "609840190.93", "24393607637.33", "0"] },
    Golden { loan: "5000", apr: "18", n: 24, pmt: "249.62", total_int: "990.87", n_rows: 24, first: ["5000.00", "249.62", "75.00", "174.62", "4825.38"], last: ["245.92", "249.61", "3.69", "245.92", "0"] },
    Golden { loan: "0.05", apr: "0", n: 10, pmt: "0.01", total_int: "0.00", n_rows: 5, first: ["0.05", "0.01", "0.00", "0.01", "0.04"], last: ["0.01", "0.01", "0.00", "0.01", "0"] },
];

type CalcNetRow = (
    &'static str,
    &'static str,
    u64,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

/// (loan, APR %, months, monthly payment, total interest, month-1 interest,
/// month-1 principal, month-1 ending balance) as published by
/// https://www.calculator.net/amortization-calculator.html
#[rustfmt::skip]
const CALCULATOR_NET: &[CalcNetRow] = &[
    ("306000", "3.5", 360, "1374.08", "188667.63", "892.50", "481.58", "305518.42"),
    ("250000", "7", 360, "1663.26", "348772.25", "1458.33", "204.92", "249795.08"),
    ("25000", "5.99", 60, "483.20", "3992.23", "124.79", "358.41", "24641.59"),
];

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn row(r: &crate::fin::AmortRow) -> [Decimal; 5] {
    [r.beg_bal, r.pmt, r.int_pmt, r.princ_pmt, r.end_bal]
}

#[test]
fn matches_independent_reference() {
    for g in REFERENCE {
        let ctx = format!("{} @ {}% x {}", g.loan, g.apr, g.n);
        let (loan, i) = (d(g.loan), d(g.apr) / d("1200"));
        let pmt = checked_monthly_pmt(loan, i, g.n).unwrap();
        let sched = calc_amort_sched(loan, i, pmt, g.n);
        assert_eq!(round_cents(pmt), d(g.pmt), "payment {ctx}");
        assert_eq!(
            total_interest(&sched),
            d(g.total_int),
            "total interest {ctx}"
        );
        assert_eq!(sched.len(), g.n_rows, "rows {ctx}");
        assert_eq!(row(&sched[0]), g.first.map(d), "first row {ctx}");
        assert_eq!(row(sched.last().unwrap()), g.last.map(d), "last row {ctx}");
    }
}

#[test]
fn matches_calculator_net() {
    for &(loan, apr, n, pmt, total, int1, princ1, end1) in CALCULATOR_NET {
        let ctx = format!("{loan} @ {apr}% x {n}");
        let (loan, i) = (d(loan), d(apr) / d("1200"));
        let exact = checked_monthly_pmt(loan, i, n).unwrap();
        let sched = calc_amort_sched(loan, i, exact, n);
        assert_eq!(round_cents(exact), d(pmt), "payment {ctx}");
        assert_eq!(
            round_cents(exact * Decimal::from(n) - loan),
            d(total),
            "unrounded total {ctx}"
        );
        // our cent-rounded total stays within $5 of the unrounded convention
        assert!(
            (total_interest(&sched) - d(total)).abs() < d("5"),
            "rounded total {ctx}"
        );
        // calculator.net splits month 1 using the unrounded payment, so
        // principal and balance can differ from our cent ledger by 1 cent.
        let r = row(&sched[0]);
        assert_eq!(r[2], d(int1), "month 1 interest {ctx}");
        assert!(
            (r[3] - d(princ1)).abs() <= d("0.01"),
            "month 1 principal {ctx}"
        );
        assert!((r[4] - d(end1)).abs() <= d("0.01"), "month 1 balance {ctx}");
    }
}
