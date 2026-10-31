use chrono::{Months, NaiveDate};

/// Due dates for `n` monthly payments on a loan starting `loan_start`.
///
/// Payment k (1-based) is due `loan_start + k months`, so the first payment
/// is one month after the loan starts (the usual convention for mortgages
/// and installment loans: interest accrues in arrears). Each date is computed
/// from `loan_start` and clamped to the end of shorter months, so a loan that
/// starts on the 31st stays on the last day of each month instead of drifting.
pub fn payment_dates(loan_start: NaiveDate, n: u64) -> Vec<NaiveDate> {
    (1..=n)
        .map_while(|k| {
            let k = u32::try_from(k).ok()?;
            loan_start.checked_add_months(Months::new(k))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn length_and_last_date() {
        let res = payment_dates(ymd(2024, 3, 12), 11);
        assert_eq!(res.len(), 11);
        assert_eq!(res.first(), Some(&ymd(2024, 4, 12)));
        assert_eq!(res.last(), Some(&ymd(2025, 2, 12)));
    }

    #[test]
    fn month_end_dates_clamp() {
        // computed from the start date, so no drift to the 29th
        let res = payment_dates(ymd(2023, 12, 31), 13);
        assert_eq!(res[0], ymd(2024, 1, 31));
        assert_eq!(res[1], ymd(2024, 2, 29));
        assert_eq!(res[2], ymd(2024, 3, 31));
        assert_eq!(res[3], ymd(2024, 4, 30));
        assert_eq!(res[11], ymd(2024, 12, 31));
        assert_eq!(res[12], ymd(2025, 1, 31));
    }

    #[test]
    fn zero_and_one_length() {
        let dt = ymd(2024, 3, 1);
        assert!(payment_dates(dt, 0).is_empty());
        assert_eq!(payment_dates(dt, 1), vec![ymd(2024, 4, 1)]);
    }

    #[test]
    fn first_payment_is_one_month_after_loan_start() {
        let res = payment_dates(ymd(2024, 3, 1), 360);
        assert_eq!(res[0], ymd(2024, 4, 1));
        assert_eq!(res[359], ymd(2054, 3, 1));
    }

    #[test]
    fn leap_day_and_year_rollover() {
        // Feb 29 start: Feb 28 in non-leap years, Feb 29 again in 2028
        let res = payment_dates(ymd(2024, 2, 29), 48);
        assert_eq!(res[0], ymd(2024, 3, 29));
        assert_eq!(res[11], ymd(2025, 2, 28));
        assert_eq!(res[47], ymd(2028, 2, 29));
        // Dec -> Jan rollover
        let res = payment_dates(ymd(2024, 11, 30), 3);
        assert_eq!(
            res,
            vec![ymd(2024, 12, 30), ymd(2025, 1, 30), ymd(2025, 2, 28)]
        );
    }

    #[test]
    fn stops_at_the_end_of_the_calendar() {
        assert!(payment_dates(NaiveDate::MAX, 3).is_empty());
    }
}
