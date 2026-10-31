use chrono::{Months, NaiveDate};

pub fn get_date_vector(mut dt: NaiveDate, length: u64) -> Vec<NaiveDate> {
    let mut res: Vec<NaiveDate> = vec![];
    res.push(dt);
    for _ in 0..length - 1 {
        let next = dt.checked_add_months(Months::new(1)).unwrap();
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
}
