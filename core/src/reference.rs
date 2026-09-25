//! Statutory calendars and holiday lists (ADR-0007). Versioned Reference Data; which calendar
//! applies to an Entity is a Setting. M1 carries Canada federal only (Q122); provinces and the UK
//! take GAP-TAX-04's "no calendar" path (Q228).

use chrono::{Datelike, NaiveDate, Weekday};

/// A tax or remittance jurisdiction whose calendar may or may not be on file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Jurisdiction {
    CanadaFederal,
    /// Named for GAP-TAX-04; no calendar in M1 (Q122, Q228).
    BritishColumbia,
    UnitedStatesFederal,
    UnitedKingdom,
}

/// Whether Reference Data holds a calendar for this jurisdiction.
pub fn has_calendar(jurisdiction: Jurisdiction) -> bool {
    matches!(
        jurisdiction,
        Jurisdiction::CanadaFederal | Jurisdiction::UnitedStatesFederal
    )
}

/// How a due date that falls on a weekend or holiday is moved (Q227).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DueDateMove {
    /// CRA and IRS: later, to the next business day `[gap: Part 8]`.
    NextBusinessDay,
}

pub(crate) fn due_date_move(jurisdiction: Jurisdiction) -> Option<DueDateMove> {
    match jurisdiction {
        Jurisdiction::CanadaFederal | Jurisdiction::UnitedStatesFederal => {
            Some(DueDateMove::NextBusinessDay)
        }
        Jurisdiction::BritishColumbia | Jurisdiction::UnitedKingdom => None,
    }
}

/// CRA-recognised holidays that fall inside or next to the Scenario horizon (2026 into early
/// 2027). Extend when a Scenario needs another year or another day; the list is Reference Data,
/// not a Setting.
fn cra_holidays_2026(date: NaiveDate) -> bool {
    matches!(
        (date.year(), date.month(), date.day()),
        (2026, 1, 1)   // New Year's Day
            | (2026, 2, 16) // Family Day (observed federal for CRA purposes where applicable)
            | (2026, 4, 3) // Good Friday
            | (2026, 5, 18) // Victoria Day
            | (2026, 7, 1) // Canada Day
            | (2026, 9, 7) // Labour Day
            | (2026, 10, 12) // Thanksgiving
            | (2026, 11, 11) // Remembrance Day
            | (2026, 12, 25) // Christmas
            | (2026, 12, 26) // Boxing Day
            | (2027, 1, 1) // New Year's Day (GAP-S11 AT2 band after 12-31)
    )
}

fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

fn is_non_business_day(date: NaiveDate, jurisdiction: Jurisdiction) -> bool {
    match jurisdiction {
        Jurisdiction::CanadaFederal => is_weekend(date) || cra_holidays_2026(date),
        Jurisdiction::UnitedStatesFederal => is_weekend(date), // holiday list when a Scenario needs it
        Jurisdiction::BritishColumbia | Jurisdiction::UnitedKingdom => true,
    }
}

/// Move a statutory due date for weekends and holidays in the calendar's direction. Returns the
/// date unchanged when it is already a business day. `None` when the jurisdiction has no calendar.
pub fn move_due_date(due: NaiveDate, jurisdiction: Jurisdiction) -> Option<NaiveDate> {
    match due_date_move(jurisdiction)? {
        DueDateMove::NextBusinessDay => {
            let mut d = due;
            while is_non_business_day(d, jurisdiction) {
                d = d.succ_opt()?;
            }
            Some(d)
        }
    }
}

/// GST/HST reporting-period lengths the owner selects (ADR-0007's Setting side).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GstHstPeriod {
    Monthly,
    Quarterly,
    Annual,
}

/// The reporting period of one length that contains a date, as its first and last day. Periods are
/// calendar periods: a month, a calendar quarter ending March, June, September or December (the
/// boundaries Xero's `3MONTHLY` names), or the calendar year.
pub fn sales_tax_period(on: NaiveDate, period: GstHstPeriod) -> Option<(NaiveDate, NaiveDate)> {
    let (first_month, months) = match period {
        GstHstPeriod::Monthly => (on.month(), 1),
        GstHstPeriod::Quarterly => ((on.month() - 1) / 3 * 3 + 1, 3),
        GstHstPeriod::Annual => (1, 12),
    };
    let start = NaiveDate::from_ymd_opt(on.year(), first_month, 1)?;
    let end = start
        .checked_add_months(chrono::Months::new(months))?
        .pred_opt()?;
    Some((start, end))
}

/// The statutory due date of a sales tax remittance covering a period that ended on `period_end`:
/// the last day of the month one month after a monthly or quarterly period end, or three months
/// after an annual one `[gap: Part 2]` (September → 10-31, not chrono's 10-30 from adding a month
/// to the 30th). Before any calendar moves it, which is the date an obligation states beside the
/// moved one (`due 10-31, moved to 11-02`).
pub fn sales_tax_due_date(period_end: NaiveDate, period: GstHstPeriod) -> Option<NaiveDate> {
    let months = match period {
        GstHstPeriod::Monthly | GstHstPeriod::Quarterly => 1,
        GstHstPeriod::Annual => 3,
    };
    let due_month_start = period_end
        .with_day(1)?
        .checked_add_months(chrono::Months::new(months))?;
    let next_month = due_month_start.checked_add_months(chrono::Months::new(1))?;
    next_month.pred_opt()
}

/// The name a jurisdiction's sales tax goes by, which the Rules use to name an obligation — never
/// a provider account title (ADR-0013).
pub fn sales_tax_name(jurisdiction: Jurisdiction) -> &'static str {
    match jurisdiction {
        Jurisdiction::CanadaFederal => "GST/HST",
        Jurisdiction::BritishColumbia => "PST",
        Jurisdiction::UnitedStatesFederal => "sales tax",
        Jurisdiction::UnitedKingdom => "VAT",
    }
}

/// The jurisdiction's own name, which GAP-TAX-04's Decision Item states.
pub fn jurisdiction_name(jurisdiction: Jurisdiction) -> &'static str {
    match jurisdiction {
        Jurisdiction::CanadaFederal => "Canada federal",
        Jurisdiction::BritishColumbia => "British Columbia",
        Jurisdiction::UnitedStatesFederal => "United States federal",
        Jurisdiction::UnitedKingdom => "United Kingdom",
    }
}

/// CRA payroll remitter types (GAP-PAYROLL-03, Q285, ADR-0007). Period boundaries and due dates
/// come from the remitter's calendar (Q227), not from the pay cadence: biweekly net pay under a
/// Regular remitter still remits one month of runs on the 15th.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemitterType {
    /// One remittance per calendar month of pay; due the 15th of the following month `[gap: Part 1]`.
    Regular,
    /// Two remittances per calendar month: pay on the 1st–15th due the 25th of that month; pay on
    /// the 16th–end due the 10th of the next month `[gap: Part 1]`.
    AcceleratedThreshold1,
    /// Four remittances per calendar month: pay on the 1st–7th, 8th–14th, 15th–21st, and 22nd–end;
    /// each due the third working day after that period ends `[gap: Part 1]`.
    AcceleratedThreshold2,
    /// One remittance per calendar quarter of pay; due the 15th of the month after the quarter
    /// (15 April, 15 July, 15 October, 15 January) `[gap: Part 1]`.
    Quarterly,
}

fn calendar_month_end(on: NaiveDate) -> Option<NaiveDate> {
    on.with_day(1)?
        .checked_add_months(chrono::Months::new(1))?
        .pred_opt()
}

/// The nth Canada-federal business day strictly after `on` (Accelerated threshold 2 due dates).
fn nth_business_day_after(on: NaiveDate, n: u32) -> Option<NaiveDate> {
    let mut d = on;
    let mut count = 0u32;
    while count < n {
        d = d.succ_opt()?;
        if !is_non_business_day(d, Jurisdiction::CanadaFederal) {
            count += 1;
        }
    }
    Some(d)
}

/// The remittance period that contains `on` for this remitter type: first and last day. Regular
/// and Quarterly periods end on a month-end or quarter-end; Accelerated periods end mid-month
/// (Q227 — the period need not end at a month-end).
pub fn payroll_remittance_period(
    remitter: RemitterType,
    on: NaiveDate,
) -> Option<(NaiveDate, NaiveDate)> {
    match remitter {
        RemitterType::Regular => {
            let start = on.with_day(1)?;
            let end = calendar_month_end(start)?;
            Some((start, end))
        }
        RemitterType::AcceleratedThreshold1 => {
            if on.day() <= 15 {
                Some((on.with_day(1)?, on.with_day(15)?))
            } else {
                Some((on.with_day(16)?, calendar_month_end(on)?))
            }
        }
        RemitterType::AcceleratedThreshold2 => {
            let day = on.day();
            if day <= 7 {
                Some((on.with_day(1)?, on.with_day(7)?))
            } else if day <= 14 {
                Some((on.with_day(8)?, on.with_day(14)?))
            } else if day <= 21 {
                Some((on.with_day(15)?, on.with_day(21)?))
            } else {
                Some((on.with_day(22)?, calendar_month_end(on)?))
            }
        }
        RemitterType::Quarterly => sales_tax_period(on, GstHstPeriod::Quarterly),
    }
}

/// Statutory due date for a remittance whose covered period ended on `period_end`, before weekend
/// and holiday moves where the statute names a calendar day. Accelerated threshold 2's due is
/// already a working-day count, so the raw date is that working day.
pub fn payroll_remittance_due_raw(
    remitter: RemitterType,
    period_end: NaiveDate,
) -> Option<NaiveDate> {
    match remitter {
        RemitterType::Regular => {
            let next_month = period_end
                .with_day(1)?
                .checked_add_months(chrono::Months::new(1))?;
            next_month.with_day(15)
        }
        RemitterType::AcceleratedThreshold1 => {
            if period_end.day() == 15 {
                period_end.with_day(25)
            } else {
                let next_month = period_end
                    .with_day(1)?
                    .checked_add_months(chrono::Months::new(1))?;
                next_month.with_day(10)
            }
        }
        RemitterType::AcceleratedThreshold2 => nth_business_day_after(period_end, 3),
        RemitterType::Quarterly => {
            let next_month = period_end
                .with_day(1)?
                .checked_add_months(chrono::Months::new(1))?;
            next_month.with_day(15)
        }
    }
}

/// Due date for a remittance covering the period that ended on `period_end`, moved for weekends
/// and holidays on the CRA calendar when the statutory due is a calendar day.
pub fn payroll_remittance_due(remitter: RemitterType, period_end: NaiveDate) -> Option<NaiveDate> {
    let raw = payroll_remittance_due_raw(remitter, period_end)?;
    match remitter {
        // Already counted on working days.
        RemitterType::AcceleratedThreshold2 => Some(raw),
        RemitterType::Regular | RemitterType::AcceleratedThreshold1 | RemitterType::Quarterly => {
            move_due_date(raw, Jurisdiction::CanadaFederal)
        }
    }
}

/// Corporate tax instalment frequency used with an owner-entered amount (GAP-INCOME-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstalmentCadence {
    Monthly,
}

/// Statutory corporate instalment due date (last day of the month `k` months after the tax-year
/// start month), before weekend/holiday moves `[gap: Part 3]`.
pub fn corporate_instalment_raw(
    tax_year_start: NaiveDate,
    cadence: InstalmentCadence,
    k: u32,
) -> Option<NaiveDate> {
    match cadence {
        InstalmentCadence::Monthly => {
            let month_start = tax_year_start
                .with_day(1)?
                .checked_add_months(chrono::Months::new(k))?;
            let next = month_start.checked_add_months(chrono::Months::new(1))?;
            next.pred_opt()
        }
    }
}

/// Statutory balance-due date for a completed tax year: the last day of the month
/// `months_after` months after year-end (2 or 3 in Canada), before calendar moves.
pub fn corporate_balance_due_raw(year_end: NaiveDate, months_after: u32) -> Option<NaiveDate> {
    let due_month_start = year_end
        .with_day(1)?
        .checked_add_months(chrono::Months::new(months_after))?;
    let next = due_month_start.checked_add_months(chrono::Months::new(1))?;
    next.pred_opt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn cra_moves_saturday_due_date_to_monday() {
        // GAP-S06 / GAP-S07: GST/HST due 10-31 is a Saturday → 11-02.
        let due = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        assert_eq!(
            move_due_date(due, Jurisdiction::CanadaFederal),
            Some(NaiveDate::from_ymd_opt(2026, 11, 2).unwrap())
        );
    }

    #[test]
    fn a_quarterly_period_ends_with_the_calendar_quarter() {
        // GAP-S07's `3MONTHLY`: the run date 10-07 sits in October to December, and the period
        // before it, July to September, is the one whose remittance falls due inside the horizon.
        let on = NaiveDate::from_ymd_opt(2026, 10, 7).unwrap();
        let (start, end) = sales_tax_period(on, GstHstPeriod::Quarterly).unwrap();
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 12, 31).unwrap());
        let (start, end) =
            sales_tax_period(start.pred_opt().unwrap(), GstHstPeriod::Quarterly).unwrap();
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 7, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 9, 30).unwrap());
        // One month after the period, moved off the Saturday (GAP-S06, GAP-S07, CASH-S03).
        assert_eq!(
            move_due_date(
                sales_tax_due_date(end, GstHstPeriod::Quarterly).unwrap(),
                Jurisdiction::CanadaFederal
            ),
            Some(NaiveDate::from_ymd_opt(2026, 11, 2).unwrap())
        );
    }

    #[test]
    fn bc_has_no_calendar() {
        assert!(!has_calendar(Jurisdiction::BritishColumbia));
        assert_eq!(
            move_due_date(
                NaiveDate::from_ymd_opt(2026, 10, 31).unwrap(),
                Jurisdiction::BritishColumbia
            ),
            None
        );
    }

    #[test]
    fn regular_remittance_period_ends_on_the_month_end() {
        let on = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let (start, end) = payroll_remittance_period(RemitterType::Regular, on).unwrap();
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 10, 31).unwrap());
        assert_eq!(
            payroll_remittance_due_raw(RemitterType::Regular, end),
            Some(NaiveDate::from_ymd_opt(2026, 11, 15).unwrap())
        );
        // 2026-11-15 is a Sunday → Monday 11-16 (GAP-S03).
        assert_eq!(
            payroll_remittance_due(RemitterType::Regular, end),
            Some(NaiveDate::from_ymd_opt(2026, 11, 16).unwrap())
        );
    }

    #[test]
    fn accelerated_threshold_1_periods_end_mid_month() {
        // Q227 / GAP-PAYROLL-03: the remittance period need not end at a month-end.
        let early = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let (start, end) =
            payroll_remittance_period(RemitterType::AcceleratedThreshold1, early).unwrap();
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 10, 15).unwrap());
        assert_eq!(
            payroll_remittance_due_raw(RemitterType::AcceleratedThreshold1, end),
            Some(NaiveDate::from_ymd_opt(2026, 10, 25).unwrap())
        );

        let late = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();
        let (start, end) =
            payroll_remittance_period(RemitterType::AcceleratedThreshold1, late).unwrap();
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 10, 16).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 10, 31).unwrap());
        assert_eq!(
            payroll_remittance_due_raw(RemitterType::AcceleratedThreshold1, end),
            Some(NaiveDate::from_ymd_opt(2026, 11, 10).unwrap())
        );
    }

    #[test]
    fn accelerated_threshold_2_periods_are_weekly_bands() {
        let on = NaiveDate::from_ymd_opt(2026, 10, 10).unwrap();
        let (start, end) =
            payroll_remittance_period(RemitterType::AcceleratedThreshold2, on).unwrap();
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 10, 8).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 10, 14).unwrap());
        // 14 Oct 2026 is Wednesday → 3rd working day after is Monday 19 Oct.
        assert_eq!(
            payroll_remittance_due(RemitterType::AcceleratedThreshold2, end),
            Some(NaiveDate::from_ymd_opt(2026, 10, 19).unwrap())
        );
    }

    #[test]
    fn quarterly_remitter_follows_the_calendar_quarter() {
        let on = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let (start, end) = payroll_remittance_period(RemitterType::Quarterly, on).unwrap();
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 12, 31).unwrap());
        assert_eq!(
            payroll_remittance_due_raw(RemitterType::Quarterly, end),
            Some(NaiveDate::from_ymd_opt(2027, 1, 15).unwrap())
        );
    }
}
