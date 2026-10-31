use chrono::{Months, NaiveDate};

/// Monthly dates `start + k months` for k in 0..length. Each date is computed
/// from `start` (clamped to the end of shorter months), so a loan starting on
/// the 31st stays on the last day of each month instead of drifting.
pub fn get_date_vector(start: NaiveDate, length: u64) -> Vec<NaiveDate> {
    (0..length)
        .map_while(|k| {
            let k = u32::try_from(k).ok()?;
            start.checked_add_months(Months::new(k))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_date_vector() {
        let dt = NaiveDate::from_ymd_opt(2024, 3, 12).unwrap();
        let length: u64 = 11;
        let res = get_date_vector(dt, length);
        assert_eq!(res.len(), length as usize);
        assert_eq!(res.first().unwrap(), &dt);
        assert_eq!(
            res.last().unwrap(),
            &NaiveDate::from_ymd_opt(2025, 1, 12).unwrap()
        );
    }

    #[test]
    fn month_end_dates_clamp() {
        let dt = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let res = get_date_vector(dt, 3);
        assert_eq!(res[1], NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
        // computed from the start date, so no drift to the 29th
        assert_eq!(res[2], NaiveDate::from_ymd_opt(2024, 3, 31).unwrap());
        let res = get_date_vector(dt, 13);
        assert_eq!(res[3], NaiveDate::from_ymd_opt(2024, 4, 30).unwrap());
        assert_eq!(res[12], NaiveDate::from_ymd_opt(2025, 1, 31).unwrap());
    }

    #[test]
    fn zero_and_one_length() {
        let dt = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
        assert!(get_date_vector(dt, 0).is_empty());
        assert_eq!(get_date_vector(dt, 1), vec![dt]);
    }
}
