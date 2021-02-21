use crate::errors::ParseScheduleError;
use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike};
use once_cell::sync::Lazy;
use std::collections::BTreeSet;
use std::collections::Bound::Included;
use std::str::FromStr;

static EMPTY: Lazy<BTreeSet<u32>> = Lazy::new(BTreeSet::new);
static MONTHS: Lazy<BTreeSet<u32>> = Lazy::new(|| (1..=12).into_iter().collect());
static DAYS: Lazy<BTreeSet<u32>> = Lazy::new(|| (1..=31).into_iter().collect());
static HOURS: Lazy<BTreeSet<u32>> = Lazy::new(|| (0..=23).into_iter().collect());
static MINUTES_OR_SECONDS: Lazy<BTreeSet<u32>> = Lazy::new(|| (0..=59).into_iter().collect());
static DAYS_OF_WEEK: Lazy<BTreeSet<u32>> = Lazy::new(|| (1..=7).into_iter().collect());

const MIN_YEAR: u32 = 1970;
const MAX_YEAR: u32 = 2099;

enum Direction {
    Forward,
    Back,
}

struct ResetHelper<'a, Z>
where
    Z: TimeZone,
{
    initial_datetime: &'a DateTime<Z>,
    initial_seconds: bool,
    initial_minutes: bool,
    initial_hours: bool,
    initial_days: bool,
    initial_months: bool,
    reset_seconds: u32,
    reset_minutes: u32,
    reset_hours: u32,
    reset_days: u32,
    reset_months: u32,
}

impl<'a, Z> ResetHelper<'a, Z>
where
    Z: TimeZone,
{
    fn new(dt: &'a DateTime<Z>, direction: Direction) -> Self {
        match direction {
            Direction::Forward => Self {
                initial_datetime: dt,
                initial_seconds: true,
                initial_minutes: true,
                initial_hours: true,
                initial_days: true,
                initial_months: true,
                reset_seconds: 0,
                reset_minutes: 0,
                reset_hours: 0,
                reset_days: 1,
                reset_months: 1,
            },
            Direction::Back => Self {
                initial_datetime: dt,
                initial_seconds: true,
                initial_minutes: true,
                initial_hours: true,
                initial_days: true,
                initial_months: true,
                reset_seconds: 59,
                reset_minutes: 59,
                reset_hours: 23,
                reset_days: 31,
                reset_months: 12,
            },
        }
    }

    fn reset_seconds(&mut self) {
        if self.initial_seconds {
            self.initial_seconds = false;
        }
    }

    fn reset_minutes(&mut self) {
        if self.initial_minutes {
            self.initial_minutes = false;
            self.reset_seconds();
        }
    }

    fn reset_hours(&mut self) {
        if self.initial_hours {
            self.initial_hours = false;
            self.reset_minutes();
        }
    }

    fn reset_days(&mut self) {
        if self.initial_days {
            self.initial_days = false;
            self.reset_hours();
        }
    }

    fn reset_months(&mut self) {
        if self.initial_months {
            self.initial_months = false;
            self.reset_days();
        }
    }

    fn seconds(&self) -> u32 {
        if self.initial_seconds {
            return self.initial_datetime.second();
        }
        self.reset_seconds
    }

    fn minutes(&self) -> u32 {
        if self.initial_minutes {
            return self.initial_datetime.minute();
        }
        self.reset_minutes
    }

    fn hours(&self) -> u32 {
        if self.initial_hours {
            return self.initial_datetime.hour();
        }
        self.reset_hours
    }

    fn days(&self) -> u32 {
        if self.initial_days {
            return self.initial_datetime.day();
        }
        self.reset_days
    }

    fn months(&self) -> u32 {
        if self.initial_months {
            return self.initial_datetime.month();
        }
        self.reset_months
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Years {
    All,
    Constrained(BTreeSet<u32>),
    Unbound,
}

#[derive(Debug, PartialEq, Clone)]
enum Seconds {
    Ignore,
    All,
    Constrained(BTreeSet<u32>),
}

#[derive(Debug, PartialEq, Clone)]
enum TimeRange {
    All,
    Constrained(BTreeSet<u32>),
}

/// Represents a parsed CRON schedule.
/// It is designed for space efficiency for caching and storage purposes such as in a CRON Scheduler.
#[derive(Debug, PartialEq, Clone)]
pub struct Schedule {
    seconds: Seconds,
    minutes: TimeRange,
    hours: TimeRange,
    days_of_month: TimeRange,
    months: TimeRange,
    days_of_week: TimeRange,
    years: Years,
}

impl Schedule {
    /// Accepts a DateTime as a placeholder to iterate forwards or backwards for the next time the
    /// CRON expression is to run or should have ran.
    /// ```rust
    /// use chrono::{DateTime, TimeZone, Utc};
    /// use cron_rs::schedule::Schedule;
    /// use std::str::FromStr;
    ///
    /// //                sec  min   hour   day of month   month   day of week   year
    /// let expression = "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2";
    /// let schedule = Schedule::from_str(expression).unwrap();
    /// let mut last: Option<DateTime<Utc>> = None;
    /// let from_date = Utc.ymd(2022, 6, 1).and_hms(8, 40, 1);
    ///
    /// // upcoming
    /// for datetime in schedule.iter_from(&from_date).take(10) {
    ///     last = Some(datetime);
    ///     println!("next -> {:?}", datetime);
    /// }
    ///
    /// // previous
    /// for datetime in schedule.iter_from(&last.unwrap()).rev().take(10) {
    ///     println!("prev -> {:?}", datetime);
    /// }
    /// ```
    pub fn iter_from<'a, Z: 'a>(
        &'a self,
        dt: &DateTime<Z>,
    ) -> impl DoubleEndedIterator<Item = DateTime<Z>> + 'a
    where
        Z: TimeZone,
    {
        ScheduleIterator::new(self, dt)
    }

    fn date<Z>(&self, dt: &DateTime<Z>, direction: Direction) -> DateTime<Z>
    where
        Z: TimeZone,
    {
        match direction {
            Direction::Forward => {
                dt.clone()
                    + if let Seconds::Ignore = self.seconds {
                        Duration::minutes(1)
                    } else {
                        Duration::seconds(1)
                    }
            }
            Direction::Back => {
                dt.clone()
                    - if let Seconds::Ignore = self.seconds {
                        Duration::minutes(1)
                    } else {
                        Duration::seconds(1)
                    }
            }
        }
    }

    fn years<Z>(&self, dt: &DateTime<Z>, direction: Direction) -> Box<dyn Iterator<Item = u32> + '_>
    where
        Z: TimeZone,
    {
        let from_year = dt.year() as u32;

        match direction {
            Direction::Forward => match &self.years {
                Years::All => Box::new(from_year.max(MIN_YEAR)..=MAX_YEAR),
                Years::Constrained(btree) => Box::new(
                    btree
                        .range(from_year.max(MIN_YEAR) as u32..=MAX_YEAR)
                        .cloned(),
                ),
                Years::Unbound => Box::new(from_year..),
            },
            Direction::Back => match &self.years {
                Years::All => Box::new((MIN_YEAR..=from_year.min(MAX_YEAR)).rev()),
                Years::Constrained(btree) => Box::new(
                    btree
                        .range(MIN_YEAR..=from_year.min(MAX_YEAR))
                        .rev()
                        .cloned(),
                ),
                Years::Unbound => Box::new((u32::MIN..=from_year).rev()),
            },
        }
    }

    fn months(&self) -> &BTreeSet<u32> {
        match &self.months {
            TimeRange::All => &MONTHS,
            TimeRange::Constrained(m) => m,
        }
    }

    fn days_of_month(&self) -> &BTreeSet<u32> {
        match &self.days_of_month {
            TimeRange::All => &DAYS,
            TimeRange::Constrained(m) => m,
        }
    }

    fn hours(&self) -> &BTreeSet<u32> {
        match &self.hours {
            TimeRange::All => &HOURS,
            TimeRange::Constrained(m) => m,
        }
    }

    fn minutes(&self) -> &BTreeSet<u32> {
        match &self.minutes {
            TimeRange::All => &MINUTES_OR_SECONDS,
            TimeRange::Constrained(m) => m,
        }
    }

    fn seconds(&self) -> &BTreeSet<u32> {
        match &self.seconds {
            Seconds::All => &MINUTES_OR_SECONDS,
            Seconds::Constrained(s) => s,
            Seconds::Ignore => &EMPTY,
        }
    }

    fn days_of_week(&self) -> &BTreeSet<u32> {
        match &self.days_of_week {
            TimeRange::All => &DAYS_OF_WEEK,
            TimeRange::Constrained(dow) => dow,
        }
    }

    fn before<Z>(&self, dt: &DateTime<Z>) -> Option<DateTime<Z>>
    where
        Z: TimeZone,
    {
        let timezone = dt.timezone();
        let dt = self.date(dt, Direction::Back);

        let mut helper = ResetHelper::new(&dt, Direction::Back);

        let months = self.months();
        let days_of_month = self.days_of_month();
        let hours = self.hours();
        let minutes = self.minutes();
        let seconds = self.seconds();
        let ignore_seconds = seconds.is_empty();
        let days_of_week = self.days_of_week();

        for year in self.years(&dt, Direction::Back) {
            let month_end = helper.months();
            if !months.contains(&month_end) {
                helper.reset_months();
            }

            for month in months.range(1..=month_end).rev().cloned() {
                let days_end = helper.days();
                if !days_of_month.contains(&days_end) {
                    helper.reset_days();
                }

                'days_loop: for day_of_month in days_of_month
                    .range((
                        Included(1),
                        Included(days_in_month(month, year).min(days_end)),
                    ))
                    .rev()
                    .cloned()
                {
                    let hours_end = helper.hours();
                    if !hours.contains(&hours_end) {
                        helper.reset_hours();
                    }

                    for hour in hours.range(0..=hours_end).rev().cloned() {
                        let minutes_end = helper.minutes();
                        if !minutes.contains(&minutes_end) {
                            helper.reset_minutes();
                        }

                        for minute in minutes.range(0..=minutes_end).rev().cloned() {
                            if ignore_seconds {
                                let candidate = if let Some(candidate) = timezone
                                    .ymd(year as i32, month, day_of_month)
                                    .and_hms_opt(hour, minute, 0)
                                {
                                    candidate
                                } else {
                                    continue;
                                };
                                if !days_of_week.contains(&candidate.weekday().number_from_sunday())
                                {
                                    helper.reset_days();
                                    continue 'days_loop;
                                }
                                return Some(candidate);
                            } else {
                                let seconds_end = helper.seconds();
                                if !seconds.contains(&seconds_end) {
                                    helper.reset_seconds();
                                }

                                for second in seconds.range(0..=seconds_end).rev().cloned() {
                                    let candidate = if let Some(candidate) = timezone
                                        .ymd(year as i32, month, day_of_month)
                                        .and_hms_opt(hour, minute, second)
                                    {
                                        candidate
                                    } else {
                                        continue;
                                    };
                                    if !days_of_week
                                        .contains(&candidate.weekday().number_from_sunday())
                                    {
                                        helper.reset_days();
                                        continue 'days_loop;
                                    }
                                    return Some(candidate);
                                }
                                helper.reset_seconds();
                            }
                        }
                        helper.reset_minutes();
                    }
                    helper.reset_hours();
                }
                helper.reset_days();
            }
            helper.reset_months();
        }
        None
    }

    fn after<Z>(&self, dt: &DateTime<Z>) -> Option<DateTime<Z>>
    where
        Z: TimeZone,
    {
        let timezone = dt.timezone();
        let dt = self.date(dt, Direction::Forward);

        let mut helper = ResetHelper::new(&dt, Direction::Forward);

        let months = self.months();
        let days_of_month = self.days_of_month();
        let hours = self.hours();
        let minutes = self.minutes();
        let seconds = self.seconds();
        let ignore_seconds = seconds.is_empty();
        let days_of_week = self.days_of_week();

        for year in self.years(&dt, Direction::Forward) {
            let month_start = helper.months();
            if !months.contains(&month_start) {
                helper.reset_months();
            }

            for month in months.range(month_start..=12).cloned() {
                let day_start = helper.days();
                if !days_of_month.contains(&day_start) {
                    helper.reset_days();
                }

                'days_loop: for day_of_month in days_of_month
                    .range((Included(day_start), Included(days_in_month(month, year))))
                    .cloned()
                {
                    let hour_start = helper.hours();
                    if !hours.contains(&hour_start) {
                        helper.reset_hours();
                    }

                    for hour in hours.range(hour_start..=23).cloned() {
                        let minutes_start = helper.minutes();
                        if !minutes.contains(&minutes_start) {
                            helper.reset_minutes();
                        }

                        for minute in minutes.range(minutes_start..=59).cloned() {
                            if ignore_seconds {
                                let candidate = if let Some(candidate) = timezone
                                    .ymd(year as i32, month, day_of_month)
                                    .and_hms_opt(hour, minute, 0)
                                {
                                    candidate
                                } else {
                                    continue;
                                };
                                if !days_of_week.contains(&candidate.weekday().number_from_sunday())
                                {
                                    helper.reset_days();
                                    continue 'days_loop;
                                }
                                return Some(candidate);
                            } else {
                                let seconds_start = helper.seconds();
                                if !seconds.contains(&seconds_start) {
                                    helper.reset_seconds();
                                }

                                for second in seconds.range(seconds_start..=59).cloned() {
                                    let candidate = if let Some(candidate) = timezone
                                        .ymd(year as i32, month, day_of_month)
                                        .and_hms_opt(hour, minute, second)
                                    {
                                        candidate
                                    } else {
                                        continue;
                                    };
                                    if !days_of_week
                                        .contains(&candidate.weekday().number_from_sunday())
                                    {
                                        helper.reset_days();
                                        continue 'days_loop;
                                    }
                                    return Some(candidate);
                                }
                                helper.reset_seconds();
                            }
                        }
                        helper.reset_minutes();
                    }
                    helper.reset_hours();
                }
                helper.reset_days();
            }
            helper.reset_months();
        }
        None
    }
}

fn is_leap_year(year: u32) -> bool {
    let by_four = year % 4 == 0;
    let by_hundred = year % 100 == 0;
    let by_four_hundred = year % 400 == 0;
    by_four && ((!by_hundred) || by_four_hundred)
}

fn days_in_month(month: u32, year: u32) -> u32 {
    let is_leap_year = is_leap_year(year);
    match month {
        9 | 4 | 6 | 11 => 30,
        2 if is_leap_year => 29,
        2 => 28,
        _ => 31,
    }
}

impl FromStr for Schedule {
    type Err = ParseScheduleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fields: Vec<&str> = s.split_whitespace().collect();
        match fields.len() {
            5 => Ok(Schedule {
                seconds: Seconds::Ignore,
                minutes: parse_field(fields[0], 0, 59, false, false, false)?,
                hours: parse_field(fields[1], 0, 23, false, false, false)?,
                days_of_month: parse_field(fields[2], 1, 31, false, false, false)?,
                months: parse_field(fields[3], 1, 12, false, false, true)?,
                days_of_week: parse_field(fields[4], 1, 7, false, true, false)?,
                years: Years::Unbound,
            }),
            6 => Ok(Schedule {
                seconds: match parse_field(fields[0], 0, 59, true, false, false)? {
                    TimeRange::All => Seconds::All,
                    TimeRange::Constrained(set) => Seconds::Constrained(set),
                },
                minutes: parse_field(fields[1], 0, 59, true, false, false)?,
                hours: parse_field(fields[2], 0, 23, true, false, false)?,
                days_of_month: parse_field(fields[3], 1, 31, true, false, false)?,
                months: parse_field(fields[4], 1, 12, true, false, true)?,
                days_of_week: parse_field(fields[5], 1, 7, true, true, false)?,
                years: Years::All,
            }),
            7 => Ok(Schedule {
                seconds: match parse_field(fields[0], 0, 59, true, false, false)? {
                    TimeRange::All => Seconds::All,
                    TimeRange::Constrained(set) => Seconds::Constrained(set),
                },
                minutes: parse_field(fields[1], 0, 59, true, false, false)?,
                hours: parse_field(fields[2], 0, 23, true, false, false)?,
                days_of_month: parse_field(fields[3], 1, 31, true, false, false)?,
                months: parse_field(fields[4], 1, 12, true, false, true)?,
                days_of_week: parse_field(fields[5], 1, 7, true, true, false)?,
                years: match parse_field(fields[6], MIN_YEAR, MAX_YEAR, true, false, false)? {
                    TimeRange::All => Years::All,
                    TimeRange::Constrained(f) => Years::Constrained(f),
                },
            }),
            _ => Err(ParseScheduleError::ArgumentCount),
        }
    }
}

fn parse_range(
    left_range: &str,
    right_range: &str,
    is_vixie: bool,
    is_dom: bool,
    is_dow: bool,
) -> Result<(u32, u32), ParseScheduleError> {
    let l = parse_time_unit(left_range, is_vixie, is_dom, is_dow)?;
    let r = parse_time_unit(right_range, is_vixie, is_dom, is_dow)?;
    Ok((l, r))
}

fn parse_time_unit(
    s: &str,
    is_vixie: bool,
    is_dom: bool,
    is_dow: bool,
) -> Result<u32, ParseScheduleError> {
    let num;
    if is_dom {
        num = month(s)?;
    } else if is_dow {
        num = day_of_week(s, is_vixie)?;
    } else {
        num = s.parse()?;
    }
    Ok(num)
}

fn parse_field(
    value: &str,
    min: u32,
    max: u32,
    is_vixie: bool,
    is_dow: bool,
    is_dom: bool,
) -> Result<TimeRange, ParseScheduleError> {
    let mut set = BTreeSet::<u32>::new();

    for v in value.split(',') {
        let mut step_iter = v.splitn(2, '/');
        let left_step = step_iter.next().unwrap();
        let right_step = step_iter.next();

        let mut dash_iter = left_step.splitn(2, '-');
        let left_dash = dash_iter.next().unwrap();
        let right_dash = dash_iter.next();

        match (left_dash, right_dash, right_step) {
            (left_range, Some(right_range), Some(step_value)) => {
                let (l, r) = parse_range(left_range, right_range, is_vixie, is_dom, is_dow)?;

                if l < min || l > max || r < min || r > max || l > r {
                    return Err(ParseScheduleError::InvalidRange(v.into()));
                }

                for i in (l..=r).step_by(step_value.parse()?) {
                    set.insert(i);
                }
            }
            (left_range, Some(right_range), None) => {
                let (l, r) = parse_range(left_range, right_range, is_vixie, is_dom, is_dow)?;

                if l < min || l > max || r < min || r > max || l > r {
                    return Err(ParseScheduleError::InvalidRange(v.into()));
                }

                if l == min && r == max {
                    return Ok(TimeRange::All);
                }

                for i in l..=r {
                    set.insert(i);
                }
            }
            (left_most, None, Some(step_value)) => match left_most {
                "*" => {
                    for i in (min..=max).step_by(step_value.parse()?) {
                        set.insert(i);
                    }
                }
                _ => {
                    let left = parse_time_unit(left_most, is_vixie, is_dom, is_dow)?;

                    for i in (left..=max).step_by(step_value.parse()?) {
                        set.insert(i);
                    }
                }
            },
            (left_most, None, None) => match left_most {
                "*" => {
                    return Ok(TimeRange::All);
                }
                _ => {
                    let i = parse_time_unit(left_most, is_vixie, is_dom, is_dow)?;
                    set.insert(i);
                }
            },
        };
    }

    Ok(TimeRange::Constrained(set))
}

fn month(value: &str) -> Result<u32, ParseScheduleError> {
    match value.to_uppercase().as_ref() {
        "JAN" | "1" => Ok(1),
        "FEB" | "2" => Ok(2),
        "MAR" | "3" => Ok(3),
        "APR" | "4" => Ok(4),
        "MAY" | "5" => Ok(5),
        "JUN" | "6" => Ok(6),
        "JUL" | "7" => Ok(7),
        "AUG" | "8" => Ok(8),
        "SEP" | "9" => Ok(9),
        "OCT" | "10" => Ok(10),
        "NOV" | "11" => Ok(11),
        "DEC" | "12" => Ok(12),
        _ => Err(ParseScheduleError::InvalidMonthIndicator(value.into())),
    }
}

fn day_of_week(value: &str, is_vixie: bool) -> Result<u32, ParseScheduleError> {
    if is_vixie {
        match value.to_uppercase().as_ref() {
            "SUN" | "1" => Ok(1),
            "MON" | "2" => Ok(2),
            "TUE" | "3" => Ok(3),
            "WED" | "4" => Ok(4),
            "THU" | "5" => Ok(5),
            "FRI" | "6" => Ok(6),
            "SAT" | "7" => Ok(7),
            _ => Err(ParseScheduleError::InvalidDayOfWeekIndicator(value.into())),
        }
    } else {
        match value.to_uppercase().as_ref() {
            "SUN" | "0" | "7" => Ok(1),
            "MON" | "1" => Ok(2),
            "TUE" | "2" => Ok(3),
            "WED" | "3" => Ok(4),
            "THU" | "4" => Ok(5),
            "FRI" | "5" => Ok(6),
            "SAT" | "6" => Ok(7),
            _ => Err(ParseScheduleError::InvalidDayOfWeekIndicator(value.into())),
        }
    }
}

struct ScheduleIterator<'a, Z>
where
    Z: TimeZone,
{
    is_done: bool,
    schedule: &'a Schedule,
    previous_datetime: DateTime<Z>,
}

impl<'a, Z> ScheduleIterator<'a, Z>
where
    Z: TimeZone,
{
    fn new(schedule: &'a Schedule, starting_datetime: &DateTime<Z>) -> ScheduleIterator<'a, Z> {
        ScheduleIterator {
            is_done: false,
            schedule,
            previous_datetime: starting_datetime.clone(),
        }
    }
}

impl<'a, Z> Iterator for ScheduleIterator<'a, Z>
where
    Z: TimeZone,
{
    type Item = DateTime<Z>;

    fn next(&mut self) -> Option<DateTime<Z>> {
        if self.is_done {
            return None;
        }
        if let Some(next_datetime) = self.schedule.after(&self.previous_datetime) {
            self.previous_datetime = next_datetime.clone();
            Some(next_datetime)
        } else {
            self.is_done = true;
            None
        }
    }
}

impl<'a, Z> DoubleEndedIterator for ScheduleIterator<'a, Z>
where
    Z: TimeZone,
{
    fn next_back(&mut self) -> Option<DateTime<Z>> {
        if self.is_done {
            return None;
        }
        if let Some(next_datetime) = self.schedule.before(&self.previous_datetime) {
            self.previous_datetime = next_datetime.clone();
            Some(next_datetime)
        } else {
            self.is_done = true;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn parse_invalid() {
        assert_eq!(
            Err(ParseScheduleError::ParseIntError(
                "invalid".parse::<u32>().err().unwrap()
            )),
            parse_field("invalid", 0, 59, true, false, false)
        );
    }

    #[test]
    fn parse_seconds_minutes() {
        let expected = TimeRange::Constrained((0..=58).into_iter().collect());
        assert_eq!(Ok(expected), parse_field("0-58", 0, 59, true, false, false));
        assert_eq!(
            Ok(TimeRange::All),
            parse_field("*", 0, 59, true, false, false)
        );
        assert_eq!(
            Ok(TimeRange::All),
            parse_field("0-59", 0, 59, true, false, false)
        );
    }

    #[test]
    fn parse_seconds_minutes_step_2() {
        let expected = TimeRange::Constrained((0..=59).into_iter().step_by(2).collect());
        assert_eq!(
            Ok(expected.clone()),
            parse_field("*/2", 0, 59, true, false, false)
        );
        assert_eq!(
            Ok(expected),
            parse_field("0-59/2", 0, 59, true, false, false)
        );
    }

    #[test]
    fn parse_hours() {
        let expected = TimeRange::Constrained((0..=22).into_iter().collect());
        assert_eq!(Ok(expected), parse_field("0-22", 0, 23, true, false, false));
        assert_eq!(
            Ok(TimeRange::All),
            parse_field("*", 0, 23, true, false, false)
        );
        assert_eq!(
            Ok(TimeRange::All),
            parse_field("0-23", 0, 23, true, false, false)
        );
    }

    #[test]
    fn parse_days_of_month() {
        let expected = TimeRange::Constrained((1..=30).into_iter().collect());
        assert_eq!(Ok(expected), parse_field("1-30", 1, 31, true, false, false));
        assert_eq!(
            Ok(TimeRange::All),
            parse_field("*", 1, 31, true, false, false)
        );
    }

    #[test]
    fn parse_months() {
        assert_eq!(
            Ok(TimeRange::All),
            parse_field("*", 1, 12, true, false, true)
        );
        assert_eq!(
            Ok(TimeRange::All),
            parse_field("1-12", 1, 12, true, false, true)
        );
        assert_eq!(
            Ok(TimeRange::All),
            parse_field("JAN-DEC", 1, 12, true, false, true)
        );
        assert_eq!(
            Ok(TimeRange::All),
            parse_field("1-DEC", 1, 12, true, false, true)
        );
        assert_eq!(
            Ok(TimeRange::All),
            parse_field("JAN-12", 1, 12, true, false, true)
        );
        assert_eq!(
            Ok(TimeRange::Constrained((2..=4).into_iter().collect())),
            parse_field("FEB-APR", 1, 12, true, false, true)
        );
        assert_eq!(
            Ok(TimeRange::Constrained((2..=4).into_iter().collect())),
            parse_field("2-APR", 1, 12, true, false, true)
        );
        assert_eq!(
            Ok(TimeRange::Constrained((2..=4).into_iter().collect())),
            parse_field("FEB-4", 1, 12, true, false, true)
        );
        assert_eq!(
            Ok(TimeRange::Constrained((2..=4).into_iter().collect())),
            parse_field("2-4", 1, 12, true, false, true)
        );
        assert_eq!(
            Ok(TimeRange::Constrained({
                let mut b = (2..=4).into_iter().step_by(2).collect::<BTreeSet<u32>>();
                b.insert(11);
                b
            })),
            parse_field("FEB-APR/2,NOV", 1, 12, true, false, true)
        );
        assert_eq!(
            Ok(TimeRange::Constrained({
                let mut b = (2..=4).into_iter().step_by(2).collect::<BTreeSet<u32>>();
                b.insert(11);
                b
            })),
            parse_field("feb-APR/2,nOv", 1, 12, true, false, true)
        );
    }

    #[test]
    fn parse_years() {
        let expected = TimeRange::Constrained((1980..=2000).into_iter().collect());
        assert_eq!(
            Ok(expected),
            parse_field("1980-2000", MIN_YEAR, MAX_YEAR, true, false, false)
        );
        assert_eq!(
            Ok(TimeRange::All),
            parse_field("*", MIN_YEAR, MAX_YEAR, true, false, false)
        );
        assert_eq!(
            Ok(TimeRange::Constrained(
                (MIN_YEAR..=MAX_YEAR).step_by(2).into_iter().collect()
            )),
            parse_field("*/2", MIN_YEAR, MAX_YEAR, true, false, false)
        );
    }

    #[test]
    fn parse_vixie() {
        let expected = Schedule {
            seconds: Seconds::Constrained((0..=59).into_iter().step_by(5).collect()),
            minutes: TimeRange::All,
            hours: TimeRange::All,
            days_of_month: TimeRange::All,
            months: TimeRange::All,
            days_of_week: TimeRange::All,
            years: Years::All,
        };
        let parsed = Schedule::from_str("*/5 * * * * * *");
        assert_eq!(Ok(expected), parsed);
    }

    #[test]
    fn parse_vixie_optional_year() {
        let expected = Schedule {
            seconds: Seconds::Constrained((0..=59).into_iter().step_by(5).collect()),
            minutes: TimeRange::All,
            hours: TimeRange::All,
            days_of_month: TimeRange::All,
            months: TimeRange::All,
            days_of_week: TimeRange::All,
            years: Years::All,
        };
        let parsed = Schedule::from_str("*/5 * * * * *");
        assert_eq!(Ok(expected), parsed);
    }

    #[test]
    fn parse_linux_crontab() {
        let expected = Schedule {
            seconds: Seconds::Ignore,
            minutes: TimeRange::Constrained((0..=59).into_iter().step_by(5).collect()),
            hours: TimeRange::All,
            days_of_month: TimeRange::All,
            months: TimeRange::All,
            days_of_week: TimeRange::All,
            years: Years::Unbound,
        };
        let parsed = Schedule::from_str("*/5 * * * *");
        assert_eq!(Ok(expected), parsed);
    }

    #[test]
    fn schedule_every_5_seconds() {
        let from_date = Utc.ymd(2021, 2, 1).and_hms_opt(1, 1, 40).unwrap();
        let parsed = Schedule::from_str("*/5 * * * * *").unwrap();
        let mut iter = parsed.iter_from(&from_date);
        assert_eq!(
            iter.next().unwrap(),
            "2021-02-01T01:01:45Z".parse::<DateTime<Utc>>().unwrap()
        );
        assert_eq!(
            iter.next().unwrap(),
            "2021-02-01T01:01:50Z".parse::<DateTime<Utc>>().unwrap()
        );
        assert_eq!(
            iter.next().unwrap(),
            "2021-02-01T01:01:55Z".parse::<DateTime<Utc>>().unwrap()
        );
        assert_eq!(
            iter.next().unwrap(),
            "2021-02-01T01:02:00Z".parse::<DateTime<Utc>>().unwrap()
        );
    }

    #[test]
    fn schedule_every_5_minutes_vixie() {
        let from_date = Utc.ymd(2021, 2, 1).and_hms_opt(1, 1, 1).unwrap();
        let parsed = Schedule::from_str("0 */5 * * * *").unwrap();
        let mut iter = parsed.iter_from(&from_date);
        assert_eq!(
            iter.next().unwrap(),
            "2021-02-01T01:05:00Z".parse::<DateTime<Utc>>().unwrap()
        );
        assert_eq!(
            iter.next().unwrap(),
            "2021-02-01T01:10:00Z".parse::<DateTime<Utc>>().unwrap()
        );
    }

    #[test]
    fn schedule_every_5_minutes_crontab() {
        let from_date = Utc.ymd(2021, 2, 1).and_hms_opt(1, 1, 1).unwrap();
        let parsed = Schedule::from_str("*/5 * * * *").unwrap();
        let mut iter = parsed.iter_from(&from_date);
        assert_eq!(
            iter.next().unwrap(),
            "2021-02-01T01:05:00Z".parse::<DateTime<Utc>>().unwrap()
        );
        assert_eq!(
            iter.next().unwrap(),
            "2021-02-01T01:10:00Z".parse::<DateTime<Utc>>().unwrap()
        );
    }

    #[test]
    fn test_no_panic_on_nonexistent_time_after() {
        use chrono::offset::TimeZone;
        use chrono_tz::Tz;

        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz
            .ymd(2019, 10, 27)
            .and_hms(0, 3, 29)
            .checked_add_signed(chrono::Duration::hours(1)) // puts it in the middle of the DST transition
            .unwrap();
        let schedule = Schedule::from_str("* * * * * Sat,Sun *").unwrap();
        let next = schedule.iter_from(&dt).next().unwrap();
        assert!(next > dt); // test is ensuring line above does not panic
        assert_eq!(
            next,
            "2019-10-27T02:00:00Z".parse::<DateTime<Utc>>().unwrap()
        );
    }

    #[test]
    fn test_no_panic_on_nonexistent_time_before() {
        use chrono::offset::TimeZone;
        use chrono_tz::Tz;

        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz
            .ymd(2019, 10, 27)
            .and_hms(0, 3, 29)
            .checked_add_signed(chrono::Duration::hours(1)) // puts it in the middle of the DST transition
            .unwrap();
        let schedule = Schedule::from_str("* * * * * Sat,Sun *").unwrap();
        let prev = schedule.iter_from(&dt).rev().next().unwrap();
        assert!(prev < dt); // test is ensuring line above does not panic
        assert_eq!(
            prev,
            "2019-10-26T23:59:59Z".parse::<DateTime<Utc>>().unwrap()
        );
    }

    #[test]
    fn test_next_and_prev_from() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule: Schedule = expression.parse().unwrap();

        let from_date = Utc.ymd(2021, 2, 14).and_hms(23, 49, 55);

        let next = schedule.iter_from(&from_date).next();
        assert!(next.is_some());
        assert_eq!(
            next.unwrap(),
            "2022-01-01T17:05:00Z".parse::<DateTime<Utc>>().unwrap()
        );

        let next2 = schedule.iter_from(&next.unwrap()).next();
        assert!(next2.is_some());
        assert_eq!(
            next2.unwrap(),
            "2022-01-01T17:13:00Z".parse::<DateTime<Utc>>().unwrap()
        );

        let prev = schedule.iter_from(&next2.unwrap()).rev().next();
        assert!(prev.is_some());
        assert_eq!(
            prev.unwrap(),
            "2022-01-01T17:05:00Z".parse::<DateTime<Utc>>().unwrap()
        );
        assert_eq!(prev, next);
    }
}
