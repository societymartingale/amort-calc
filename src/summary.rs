//! Loan summary and calendar-year totals, all derived from the rounded
//! schedule rows so every figure reconciles with the monthly table.

use crate::fin::AmortRow;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq)]
pub struct LoanSummary {
    /// The level payment (first row's payment).
    pub pmt: Decimal,
    /// Date of the last payment.
    pub payoff_date: Option<NaiveDate>,
    pub total_paid: Decimal,
    pub total_interest: Decimal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct YearRow {
    pub year: i32,
    pub num_pmts: usize,
    pub princ: Decimal,
    pub int: Decimal,
    pub total: Decimal,
    pub end_bal: Decimal,
}

/// Summary of a schedule; `dates[i]` is the date of `sched[i]`.
pub fn summarize(sched: &[AmortRow], dates: &[NaiveDate]) -> Option<LoanSummary> {
    let first = sched.first()?;
    let n = sched.len().min(dates.len());
    Some(LoanSummary {
        pmt: first.pmt,
        payoff_date: if n == sched.len() {
            dates.get(n - 1).copied()
        } else {
            None
        },
        total_paid: sched.iter().map(|r| r.pmt).sum(),
        total_interest: crate::fin::total_interest(sched),
    })
}

/// Groups rows by the calendar year of their payment date. Rows without a
/// date (out of chrono's range) are dropped.
pub fn yearly_totals(sched: &[AmortRow], dates: &[NaiveDate]) -> Vec<YearRow> {
    let mut out: Vec<YearRow> = Vec::new();
    for (r, d) in sched.iter().zip(dates) {
        let y = d.year();
        match out.last_mut() {
            Some(cur) if cur.year == y => {
                cur.num_pmts += 1;
                cur.princ += r.princ_pmt;
                cur.int += r.int_pmt;
                cur.total += r.pmt;
                cur.end_bal = r.end_bal;
            }
            _ => out.push(YearRow {
                year: y,
                num_pmts: 1,
                princ: r.princ_pmt,
                int: r.int_pmt,
                total: r.pmt,
                end_bal: r.end_bal,
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::date_util::payment_dates;
    use crate::fin::{calc_amort_sched, checked_monthly_pmt};
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }
    fn ymd(y: i32, m: u32, dd: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, dd).unwrap()
    }
    fn build(loan: &str, apr: &str, n: u64, start: NaiveDate) -> (Vec<AmortRow>, Vec<NaiveDate>) {
        let i = d(apr) / d("1200");
        let pmt = checked_monthly_pmt(d(loan), i, n).unwrap();
        (
            calc_amort_sched(d(loan), i, pmt, n),
            payment_dates(start, n),
        )
    }

    fn assert_reconciles(s: &[AmortRow], ds: &[NaiveDate]) {
        let y = yearly_totals(s, ds);
        let sum = |f: fn(&AmortRow) -> Decimal| s.iter().map(f).sum::<Decimal>();
        assert_eq!(
            y.iter().map(|r| r.princ).sum::<Decimal>(),
            sum(|r| r.princ_pmt)
        );
        assert_eq!(y.iter().map(|r| r.int).sum::<Decimal>(), sum(|r| r.int_pmt));
        assert_eq!(y.iter().map(|r| r.total).sum::<Decimal>(), sum(|r| r.pmt));
        assert_eq!(y.iter().map(|r| r.num_pmts).sum::<usize>(), s.len());
        assert_eq!(y.last().unwrap().end_bal, Decimal::ZERO);
        for r in &y {
            assert_eq!(r.princ + r.int, r.total);
        }
        let sm = summarize(s, ds).unwrap();
        assert_eq!(sm.total_paid, sum(|r| r.pmt));
        assert_eq!(sm.total_interest, sum(|r| r.int_pmt));
        assert_eq!(sm.total_paid - sm.total_interest, s[0].beg_bal);
    }

    #[test]
    fn yearly_sums_match_schedule_360() {
        let (s, ds) = build("306000", "3.5", 360, ymd(2025, 1, 1));
        assert_reconciles(&s, &ds);
        let y = yearly_totals(&s, &ds);
        // Feb 2025 .. Jan 2055: 11 + 29*12 + 1
        assert_eq!(y.len(), 31);
        assert_eq!((y[0].year, y[0].num_pmts), (2025, 11));
        assert_eq!((y[30].year, y[30].num_pmts), (2055, 1));
        let sm = summarize(&s, &ds).unwrap();
        assert_eq!(sm.pmt, d("1374.08"));
        assert_eq!(sm.payoff_date, Some(ymd(2055, 1, 1)));
    }

    #[test]
    fn partial_first_and_last_years_october_start() {
        let (s, ds) = build("20000", "6", 24, ymd(2024, 10, 15));
        assert_reconciles(&s, &ds);
        let y = yearly_totals(&s, &ds);
        let years: Vec<_> = y.iter().map(|r| (r.year, r.num_pmts)).collect();
        // first payment 2024-11-15, last 2026-10-15
        assert_eq!(years, vec![(2024, 2), (2025, 12), (2026, 10)]);
        assert_eq!(y[0].end_bal, s[1].end_bal);
        assert_eq!(y[1].end_bal, s[13].end_bal);
        assert_eq!(
            summarize(&s, &ds).unwrap().payoff_date,
            Some(ymd(2026, 10, 15))
        );
    }

    #[test]
    fn one_month_loan() {
        let (s, ds) = build("1000", "12", 1, ymd(2024, 12, 5));
        assert_reconciles(&s, &ds);
        let y = yearly_totals(&s, &ds);
        assert_eq!(y.len(), 1);
        assert_eq!(y[0].year, 2025);
        assert_eq!(
            (y[0].princ, y[0].int, y[0].total),
            (d("1000"), d("10"), d("1010"))
        );
        let sm = summarize(&s, &ds).unwrap();
        assert_eq!(
            sm,
            LoanSummary {
                pmt: d("1010"),
                payoff_date: Some(ymd(2025, 1, 5)),
                total_paid: d("1010"),
                total_interest: d("10"),
            }
        );
    }

    #[test]
    fn zero_interest_summary() {
        let (s, ds) = build("1000", "0", 3, ymd(2024, 1, 1));
        let sm = summarize(&s, &ds).unwrap();
        assert_eq!(sm.pmt, d("333.33"));
        assert_eq!(sm.total_paid, d("1000"));
        assert_eq!(sm.total_interest, Decimal::ZERO);
        assert_eq!(sm.payoff_date, Some(ymd(2024, 4, 1)));
    }

    #[test]
    fn empty_schedule() {
        assert!(summarize(&[], &[]).is_none());
        assert!(yearly_totals(&[], &[]).is_empty());
    }
}
