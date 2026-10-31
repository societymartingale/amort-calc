//! Parsing and validation of the raw form inputs.

use rust_decimal::prelude::*;

use crate::fin;

pub const MAX_LOAN_AMT: i64 = 1_000_000_000_000;
pub const MAX_RATE_PCT: i64 = 100;
pub const MAX_NUM_PMTS: u64 = 1200;

/// Validated loan terms.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LoanInput {
    pub loan_amt: Decimal,
    pub int_yearly_pct: Decimal,
    pub num_pmts: u64,
}

/// Commas are only accepted as thousands separators ("1,234,567.89"), so a
/// locale decimal comma like "6,5" is rejected instead of becoming 65.
fn valid_commas(s: &str) -> bool {
    let int_part = s.split('.').next().unwrap_or_default();
    if s[int_part.len()..].contains(',') {
        return false;
    }
    let digits = int_part.trim_start_matches(['+', '-']);
    let mut groups = digits.split(',');
    let first = groups.next().unwrap_or_default();
    (1..=3).contains(&first.len()) && groups.all(|g| g.len() == 3)
}

fn parse_decimal(s: &str, field: &str) -> Result<Decimal, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err(format!("{field} is required."));
    }
    let not_a_number = || format!("{field} must be a number.");
    if s.contains(',') && !valid_commas(s) {
        return Err(not_a_number());
    }
    Decimal::from_str(&s.replace(',', "")).map_err(|_| not_a_number())
}

/// Parses and validates the three numeric form fields.
pub fn parse_loan_input(
    loan_amt: &str,
    rate_pct: &str,
    num_pmts: &str,
) -> Result<LoanInput, String> {
    let loan_amt = parse_decimal(loan_amt, "Loan amount")?;
    if loan_amt <= Decimal::ZERO || loan_amt > Decimal::from(MAX_LOAN_AMT) {
        return Err(format!(
            "Loan amount must be greater than 0 and at most {MAX_LOAN_AMT}."
        ));
    }

    if loan_amt.normalize().scale() > 2 {
        return Err("Loan amount must be in whole cents (at most 2 decimal places).".into());
    }

    let int_yearly_pct = parse_decimal(rate_pct, "Annual percentage rate")?;
    if int_yearly_pct < Decimal::ZERO || int_yearly_pct > Decimal::from(MAX_RATE_PCT) {
        return Err(format!(
            "Annual percentage rate must be between 0 and {MAX_RATE_PCT}."
        ));
    }

    let n = num_pmts.trim();
    if n.is_empty() {
        return Err("Number of monthly payments is required.".into());
    }
    let num_pmts = match n.parse::<i64>() {
        Ok(v) if (1..=MAX_NUM_PMTS as i64).contains(&v) => v as u64,
        Ok(_) => {
            return Err(format!(
                "Number of monthly payments must be between 1 and {MAX_NUM_PMTS}."
            ));
        }
        Err(_) => return Err("Number of monthly payments must be a whole number.".into()),
    };

    let input = LoanInput {
        loan_amt,
        int_yearly_pct,
        num_pmts,
    };
    match fin::checked_monthly_pmt(loan_amt, input.int_monthly(), num_pmts) {
        None => return Err("These loan terms are too large to calculate.".into()),
        Some(p) if fin::round_cents(p).is_zero() => {
            return Err(
                "Loan amount is too small for this many payments (payment rounds to $0.00).".into(),
            );
        }
        Some(_) => {}
    }
    Ok(input)
}

impl LoanInput {
    pub fn int_monthly(&self) -> Decimal {
        self.int_yearly_pct / Decimal::from(1200)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn valid_input() {
        assert_eq!(
            parse_loan_input("500,000", " 6.5 ", "360"),
            Ok(LoanInput {
                loan_amt: d("500000"),
                int_yearly_pct: d("6.5"),
                num_pmts: 360
            })
        );
        assert!(parse_loan_input("1000", "0", "1").is_ok());
    }

    #[test]
    fn empty_fields() {
        assert!(
            parse_loan_input("", "6", "360")
                .unwrap_err()
                .contains("required")
        );
        assert!(
            parse_loan_input("1000", "  ", "360")
                .unwrap_err()
                .contains("required")
        );
        assert!(
            parse_loan_input("1000", "6", "")
                .unwrap_err()
                .contains("required")
        );
    }

    #[test]
    fn non_numeric_fields() {
        assert!(
            parse_loan_input("abc", "6", "360")
                .unwrap_err()
                .contains("number")
        );
        assert!(
            parse_loan_input("1000", "6%", "360")
                .unwrap_err()
                .contains("number")
        );
        assert!(
            parse_loan_input("1000", "6", "12.5")
                .unwrap_err()
                .contains("whole number")
        );
        assert!(
            parse_loan_input("1000", "6", "ten")
                .unwrap_err()
                .contains("whole number")
        );
    }

    #[test]
    fn zero_negative_or_absurd_payments() {
        for n in ["0", "-12", "1201", "99999999999999999999"] {
            assert!(parse_loan_input("1000", "6", n).is_err(), "{n}");
        }
    }

    #[test]
    fn bad_loan_amounts() {
        for amt in ["0", "-5", "1000000000001"] {
            assert!(
                parse_loan_input(amt, "6", "360")
                    .unwrap_err()
                    .contains("Loan amount"),
                "{amt}"
            );
        }
    }

    #[test]
    fn negative_or_excessive_rate() {
        assert!(
            parse_loan_input("1000", "-1", "360")
                .unwrap_err()
                .contains("rate")
        );
        assert!(
            parse_loan_input("1000", "100.1", "360")
                .unwrap_err()
                .contains("rate")
        );
    }

    #[test]
    fn loan_amount_with_fractional_cents_is_rejected() {
        assert!(
            parse_loan_input("1000.005", "6", "12")
                .unwrap_err()
                .contains("cents")
        );
        assert!(parse_loan_input("1000.50", "6", "12").is_ok());
        assert!(parse_loan_input("1000.5", "6", "12").is_ok());
    }

    #[test]
    fn misplaced_commas_are_rejected() {
        // locale decimal comma must not silently become 65%
        assert!(parse_loan_input("1000", "6,5", "12").is_err());
        // "1.000,50" (European format) must not become 1.0005
        assert!(parse_loan_input("1.000,50", "6", "12").is_err());
        assert!(parse_loan_input("1,00", "6", "12").is_err());
        assert!(parse_loan_input("1,,000", "6", "12").is_err());
        assert_eq!(
            parse_loan_input("1,234,567.89", "6", "12")
                .unwrap()
                .loan_amt,
            d("1234567.89")
        );
    }

    #[test]
    fn loan_too_small_for_term_is_rejected() {
        // $1 over 1200 months at 0% rounds to a $0.00 payment
        assert!(
            parse_loan_input("1", "0", "1200")
                .unwrap_err()
                .contains("too small")
        );
        assert!(parse_loan_input("12", "0", "1200").is_ok());
    }

    #[test]
    fn high_rate_long_term_is_accepted() {
        assert!(parse_loan_input("100000", "99", "1200").is_ok());
        assert!(parse_loan_input("1000000000000", "100", "1200").is_ok());
    }
}
