//! The template calendar: scheduled dates and the due date a rule gives one (ADR-0022, Q118).

use chrono::{Datelike, Days, Months, NaiveDate};

use crate::facts::{DueRule, Frequency, FrequencyUnit};

/// The `k`th scheduled date of a template counting from its first, negative before it: the
/// schedule extends before its first date and past its end (Q149). `None` past the calendar.
pub fn occurrence(start: NaiveDate, frequency: Frequency, k: i64) -> Option<NaiveDate> {
    let n = k.checked_mul(i64::from(frequency.interval))?;
    let months = |m: i64| {
        let m = u32::try_from(m.unsigned_abs()).ok()?;
        if k < 0 {
            start.checked_sub_months(Months::new(m))
        } else {
            start.checked_add_months(Months::new(m))
        }
    };
    let days = |d: i64| {
        if k < 0 {
            start.checked_sub_days(Days::new(d.unsigned_abs()))
        } else {
            start.checked_add_days(Days::new(d.unsigned_abs()))
        }
    };
    match frequency.unit {
        FrequencyUnit::Day => days(n),
        FrequencyUnit::Week => days(n.checked_mul(7)?),
        FrequencyUnit::Month => months(n),
        FrequencyUnit::Year => months(n.checked_mul(12)?),
    }
}

/// The index of the last scheduled date on or before a date.
pub fn index_at_or_before(start: NaiveDate, frequency: Frequency, date: NaiveDate) -> Option<i64> {
    let per_step = match frequency.unit {
        FrequencyUnit::Day => 1,
        FrequencyUnit::Week => 7,
        FrequencyUnit::Month => 30,
        FrequencyUnit::Year => 365,
    } * i64::from(frequency.interval.max(1));
    let mut k = (date - start).num_days().div_euclid(per_step);
    // The estimate is out by at most a step or two per decade; the loops are bounded regardless.
    for _ in 0..1000 {
        if occurrence(start, frequency, k + 1).is_some_and(|d| d <= date) {
            k += 1;
        } else if occurrence(start, frequency, k).is_some_and(|d| d > date) {
            k -= 1;
        } else {
            return occurrence(start, frequency, k).map(|_| k);
        }
    }
    None
}

/// The scheduled date nearest a document's date; exactly halfway covers the earlier one (Q149).
pub fn nearest_occurrence(
    start: NaiveDate,
    frequency: Frequency,
    date: NaiveDate,
) -> Option<NaiveDate> {
    let k = index_at_or_before(start, frequency, date)?;
    let before = occurrence(start, frequency, k)?;
    let Some(after) = occurrence(start, frequency, k + 1) else {
        return Some(before);
    };
    Some(if after - date < date - before {
        after
    } else {
        before
    })
}

/// The due date a due rule gives an invoice dated on a date.
pub fn due_by(rule: DueRule, date: NaiveDate) -> Option<NaiveDate> {
    let day_of = |month_start: NaiveDate, day: u32| {
        let last = month_start
            .checked_add_months(Months::new(1))?
            .pred_opt()?
            .day();
        month_start.with_day(day.clamp(1, last))
    };
    match rule {
        DueRule::DaysAfterDate(n) => date.checked_add_days(Days::new(n.into())),
        DueRule::DaysAfterMonthEnd(n) => date
            .with_day(1)?
            .checked_add_months(Months::new(1))?
            .pred_opt()?
            .checked_add_days(Days::new(n.into())),
        DueRule::DayOfCurrentMonth(n) => day_of(date.with_day(1)?, n),
        DueRule::DayOfFollowingMonth(n) => {
            day_of(date.with_day(1)?.checked_add_months(Months::new(1))?, n)
        }
    }
}
