use rust_decimal::prelude::*;

#[derive(PartialEq, Clone, Debug)]
pub struct AmortRow {
    pub period: u64,
    pub beg_bal: Decimal,
    pub pmt: Decimal,
    pub int_pmt: Decimal,
    pub princ_pmt: Decimal,
    pub end_bal: Decimal,
}

/// Level monthly payment for a fully amortizing loan. Returns `None` instead
/// of panicking on overflow or a zero payment count.
pub fn checked_monthly_pmt(
    loan_amt: Decimal,
    int_monthly: Decimal,
    num_pmts: u64,
) -> Option<Decimal> {
    if num_pmts == 0 {
        return None;
    }
    if int_monthly.is_zero() {
        return loan_amt.checked_div(Decimal::from(num_pmts));
    }
    // P = L * i / (1 - (1 + i)^-n). Using the discount factor v = 1 / (1 + i)
    // keeps v^n <= 1, so long terms at high rates cannot overflow.
    let v = Decimal::ONE.checked_div(Decimal::ONE + int_monthly)?;
    let den = Decimal::ONE.checked_sub(v.checked_powu(num_pmts)?)?;
    loan_amt.checked_mul(int_monthly)?.checked_div(den)
}

/// Rounds a money amount to the cent, half away from zero (e.g. 0.005 -> 0.01).
pub fn round_cents(x: Decimal) -> Decimal {
    x.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

/// Builds the schedule in whole cents: the payment and each month's interest
/// are rounded to the cent, and the final payment absorbs the rounding
/// remainder so the ending balance is exactly zero. If a rounded-up payment
/// pays the loan off early, the schedule ends at that (smaller) payment.
pub fn calc_amort_sched(
    loan_amt: Decimal,
    int_monthly: Decimal,
    pmt_monthly: Decimal,
    num_pmts: u64,
) -> Vec<AmortRow> {
    let pmt_monthly = round_cents(pmt_monthly);
    let mut res: Vec<AmortRow> = vec![];
    let mut beg_bal = round_cents(loan_amt);
    for p in 1..=num_pmts {
        let int_pmt = round_cents(beg_bal * int_monthly);
        // Pay off on the last period, or earlier if a rounded-up payment
        // would otherwise overpay and drive the balance negative.
        let payoff = beg_bal + int_pmt;
        let last = p == num_pmts || payoff <= pmt_monthly;
        let (pmt, princ_pmt) = if last {
            (beg_bal + int_pmt, beg_bal)
        } else {
            (pmt_monthly, pmt_monthly - int_pmt)
        };
        let end_bal = beg_bal - princ_pmt;
        res.push(AmortRow {
            period: p,
            beg_bal,
            pmt,
            int_pmt,
            princ_pmt,
            end_bal,
        });
        if last {
            break;
        }
        beg_bal = end_bal;
    }
    res
}

/// Total interest: the sum of the (rounded) interest column.
pub fn total_interest(sched: &[AmortRow]) -> Decimal {
    sched.iter().map(|r| r.int_pmt).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calc_monthly_pmt(loan_amt: Decimal, int_monthly: Decimal, num_pmts: u64) -> Decimal {
        checked_monthly_pmt(loan_amt, int_monthly, num_pmts).unwrap()
    }

    #[test]
    fn checked_pmt_rejects_zero_payments_and_overflow() {
        assert_eq!(
            checked_monthly_pmt(Decimal::ONE_THOUSAND, Decimal::ONE, 0),
            None
        );
        assert_eq!(
            checked_monthly_pmt(Decimal::ONE_THOUSAND, Decimal::ZERO, 0),
            None
        );
        assert_eq!(checked_monthly_pmt(Decimal::MAX, Decimal::TWO, 1000), None);
    }

    #[test]
    fn test_calc_monthly_pmt() {
        let res = calc_monthly_pmt(
            Decimal::from_f64(306000.0).unwrap(),
            Decimal::from_f64(3.5 / 1200.0).unwrap(),
            360,
        );
        assert_eq!(
            res.round_dp(12).to_string(),
            Decimal::from_f64(1374.076744695003)
                .unwrap()
                .round_dp(12)
                .to_string()
        );
    }

    #[test]
    fn test_calc_amort_sched() {
        let res = calc_amort_sched(
            Decimal::from_f64(306000.0).unwrap(),
            Decimal::from_f64(3.5 / 12.0 / 100.0).unwrap(),
            Decimal::from_f64(1374.076744695003).unwrap(),
            360,
        );
        assert_eq!(res.len(), 360);
        assert_eq!(
            res.first().unwrap().beg_bal,
            Decimal::from_f64(306000.0).unwrap()
        );
        assert_eq!(res.last().unwrap().end_bal, Decimal::zero());
    }

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn pmt_matches_known_values() {
        // $100,000 at 6% APR over 30 years -> $599.55/month
        assert_eq!(
            calc_monthly_pmt(d("100000"), d("0.005"), 360).round_dp(2),
            d("599.55")
        );
        // $200,000 at 4.5% APR over 15 years -> $1,529.99/month
        assert_eq!(
            calc_monthly_pmt(d("200000"), d("4.5") / d("1200"), 180).round_dp(2),
            d("1529.99")
        );
    }

    #[test]
    fn zero_interest_pmt_is_principal_split_evenly() {
        assert_eq!(calc_monthly_pmt(d("12000"), Decimal::ZERO, 12), d("1000"));
        let sched = calc_amort_sched(d("12000"), Decimal::ZERO, d("1000"), 12);
        assert!(
            sched
                .iter()
                .all(|r| r.int_pmt.is_zero() && r.princ_pmt == d("1000"))
        );
        assert_eq!(sched.last().unwrap().end_bal, Decimal::ZERO);
    }

    #[test]
    fn single_payment_repays_principal_plus_one_period_interest() {
        assert_eq!(
            calc_monthly_pmt(d("1000"), d("0.01"), 1).round_dp(10),
            d("1010")
        );
    }

    #[test]
    fn schedule_totals_and_final_balance() {
        let loan = d("250000");
        let i = d("7") / d("1200");
        let n = 360;
        let pmt = calc_monthly_pmt(loan, i, n);
        let sched = calc_amort_sched(loan, i, pmt, n);
        assert_eq!(sched.len(), n as usize);
        assert_eq!(sched.last().unwrap().end_bal, Decimal::ZERO);
        let total_princ: Decimal = sched.iter().map(|r| r.princ_pmt).sum();
        let total_int: Decimal = sched.iter().map(|r| r.int_pmt).sum();
        let total_pmt: Decimal = sched.iter().map(|r| r.pmt).sum();
        assert_eq!(total_princ, loan);
        assert_eq!(total_int + total_princ, total_pmt);
        // the rounded total interest stays within a few dollars of pmt * n - loan
        assert!((pmt * Decimal::from(n) - loan - total_int).abs() < d("5"));
        // only the last payment differs from the level payment
        let level = round_cents(pmt);
        assert!(sched[..n as usize - 1].iter().all(|r| r.pmt == level));
        assert!((sched.last().unwrap().pmt - level).abs() < d("5"));
    }

    #[test]
    fn schedule_rows_are_consistent() {
        let loan = d("50000");
        let i = d("0.004");
        let pmt = calc_monthly_pmt(loan, i, 60);
        let sched = calc_amort_sched(loan, i, pmt, 60);
        for (idx, r) in sched.iter().enumerate() {
            assert_eq!(r.period, idx as u64 + 1);
            assert_eq!(r.int_pmt, round_cents(r.beg_bal * i));
            assert_eq!(r.int_pmt + r.princ_pmt, r.pmt);
            assert_eq!(r.end_bal, r.beg_bal - r.princ_pmt);
            if idx > 0 {
                assert_eq!(r.beg_bal, sched[idx - 1].end_bal);
                // interest shrinks and principal grows each period
                assert!(r.int_pmt <= sched[idx - 1].int_pmt);
                assert!(r.princ_pmt >= sched[idx - 1].princ_pmt);
            }
        }
    }

    #[test]
    fn total_interest_is_sum_of_rounded_interest_column() {
        let loan = d("306000");
        let i = d("3.5") / d("1200");
        let pmt = checked_monthly_pmt(loan, i, 360).unwrap();
        let sched = calc_amort_sched(loan, i, pmt, 360);
        let expected: Decimal = sched.iter().map(|r| r.int_pmt).sum();
        assert_eq!(total_interest(&sched), expected);
        // pmt * n - loan disagrees with the schedule the user sees
        assert_ne!(pmt * Decimal::from(360) - loan, expected);
        assert_eq!(total_interest(&[]), Decimal::ZERO);
    }

    #[test]
    fn round_cents_is_half_away_from_zero() {
        assert_eq!(round_cents(d("0.125")), d("0.13"));
        assert_eq!(round_cents(d("0.135")), d("0.14"));
        assert_eq!(round_cents(d("-0.125")), d("-0.13"));
        assert_eq!(round_cents(d("0.1249")), d("0.12"));
    }

    #[test]
    fn empty_schedule_for_zero_payments() {
        assert!(calc_amort_sched(d("1000"), d("0.01"), d("100"), 0).is_empty());
    }

    #[test]
    fn schedule_is_in_cents_and_ends_at_exactly_zero() {
        // $1,000 at 0% over 3 payments: 333.33, 333.33, 333.34
        let pmt = checked_monthly_pmt(d("1000"), Decimal::ZERO, 3).unwrap();
        let sched = calc_amort_sched(d("1000"), Decimal::ZERO, pmt, 3);
        let pmts: Vec<_> = sched.iter().map(|r| r.pmt).collect();
        assert_eq!(pmts, vec![d("333.33"), d("333.33"), d("333.34")]);
        assert_eq!(pmts.iter().sum::<Decimal>(), d("1000"));
        assert_eq!(sched.last().unwrap().end_bal, Decimal::ZERO);

        let loan = d("306000");
        let i = d("3.5") / d("1200");
        let pmt = checked_monthly_pmt(loan, i, 360).unwrap();
        let sched = calc_amort_sched(loan, i, pmt, 360);
        for r in &sched {
            for v in [r.beg_bal, r.pmt, r.int_pmt, r.princ_pmt, r.end_bal] {
                assert_eq!(v, v.round_dp(2), "row {} not in cents: {v}", r.period);
            }
        }
        assert_eq!(sched[0].pmt, d("1374.08"));
        assert_eq!(sched.last().unwrap().end_bal, Decimal::ZERO);
        let princ: Decimal = sched.iter().map(|r| r.princ_pmt).sum();
        assert_eq!(princ, loan);
    }

    #[test]
    fn rounded_up_payment_never_overpays() {
        // 0.05 / 10 = 0.005 rounds up to 0.01, which would pay the loan off
        // after 5 payments and then drive the balance negative.
        let pmt = checked_monthly_pmt(d("0.05"), Decimal::ZERO, 10).unwrap();
        let sched = calc_amort_sched(d("0.05"), Decimal::ZERO, pmt, 10);
        assert!(sched.iter().all(|r| r.end_bal >= Decimal::ZERO));
        assert!(sched.iter().all(|r| r.pmt >= Decimal::ZERO));
        assert_eq!(
            sched.iter().map(|r| r.princ_pmt).sum::<Decimal>(),
            d("0.05")
        );
        assert_eq!(sched.last().unwrap().end_bal, Decimal::ZERO);
    }

    #[test]
    fn high_rate_long_term_does_not_overflow() {
        // 99% APR over 1200 months: (1 + i)^n overflows Decimal, but the
        // payment is well defined (~ L * i).
        let i = d("99") / d("1200");
        let pmt = checked_monthly_pmt(d("100000"), i, 1200).unwrap();
        assert_eq!(round_cents(pmt), d("8250.00"));
    }
}
