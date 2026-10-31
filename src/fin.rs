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

pub fn calc_monthly_pmt(loan_amt: Decimal, int_monthly: Decimal, num_pmts: u64) -> Decimal {
    if int_monthly.is_zero() {
        return loan_amt / Decimal::from(num_pmts);
    }
    let n = num_pmts;
    let i = int_monthly;
    let num = i * (Decimal::one() + i).powu(n);
    let den = (Decimal::one() + i).powu(n) - Decimal::one();
    loan_amt * num / den
}

pub fn calc_amort_sched(
    loan_amt: Decimal,
    int_monthly: Decimal,
    pmt_monthly: Decimal,
    num_pmts: u64,
) -> Vec<AmortRow> {
    let mut res: Vec<AmortRow> = vec![];
    let mut beg_bal = loan_amt;
    for p in 1..num_pmts + 1 {
        let int_pmt = beg_bal * int_monthly;
        let princ_pmt = pmt_monthly - int_pmt;
        let end_bal = beg_bal - princ_pmt;
        res.push(AmortRow {
            period: p,
            beg_bal,
            pmt: pmt_monthly,
            int_pmt,
            princ_pmt,
            end_bal,
        });
        beg_bal = end_bal;
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(res.last().unwrap().end_bal.round_dp(9), Decimal::zero());
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
        assert_eq!(sched.last().unwrap().end_bal.round_dp(6), Decimal::ZERO);
        let total_princ: Decimal = sched.iter().map(|r| r.princ_pmt).sum();
        let total_int: Decimal = sched.iter().map(|r| r.int_pmt).sum();
        let total_pmt: Decimal = sched.iter().map(|r| r.pmt).sum();
        assert_eq!(total_princ.round_dp(6), loan);
        assert_eq!(
            (total_int + total_princ - total_pmt).round_dp(10),
            Decimal::ZERO
        );
        assert_eq!(
            (pmt * Decimal::from(n) - loan - total_int).round_dp(6),
            Decimal::ZERO
        );
    }

    #[test]
    fn schedule_rows_are_consistent() {
        let loan = d("50000");
        let i = d("0.004");
        let pmt = calc_monthly_pmt(loan, i, 60);
        let sched = calc_amort_sched(loan, i, pmt, 60);
        for (idx, r) in sched.iter().enumerate() {
            assert_eq!(r.period, idx as u64 + 1);
            assert_eq!(r.int_pmt, r.beg_bal * i);
            assert_eq!(r.int_pmt + r.princ_pmt, r.pmt);
            assert_eq!(r.end_bal, r.beg_bal - r.princ_pmt);
            if idx > 0 {
                assert_eq!(r.beg_bal, sched[idx - 1].end_bal);
                // interest shrinks and principal grows each period
                assert!(r.int_pmt < sched[idx - 1].int_pmt);
                assert!(r.princ_pmt > sched[idx - 1].princ_pmt);
            }
        }
    }

    #[test]
    fn empty_schedule_for_zero_payments() {
        assert!(calc_amort_sched(d("1000"), d("0.01"), d("100"), 0).is_empty());
    }
}
