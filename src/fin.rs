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
}
