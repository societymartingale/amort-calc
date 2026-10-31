use chrono::{Months, NaiveDate};

pub fn get_date_vector(mut dt: NaiveDate, length: u64) -> Vec<NaiveDate> {
    let mut res: Vec<NaiveDate> = vec![];
    if length == 0 {
        return res;
    }
    res.push(dt);
    for _ in 0..length - 1 {
        let Some(next) = dt.checked_add_months(Months::new(1)) else {
            break;
        };
        res.push(next);
        dt = next;
    }
    res
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
        assert_eq!(res[2], NaiveDate::from_ymd_opt(2024, 3, 29).unwrap());
    }

    #[test]
    fn zero_and_one_length() {
        let dt = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
        assert!(get_date_vector(dt, 0).is_empty());
        assert_eq!(get_date_vector(dt, 1), vec![dt]);
    }
}
