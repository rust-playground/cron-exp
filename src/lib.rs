//! # Schedule
//!
//! Is a CRON expression parser and explorer.
//!
//! It Supports both Crantab and Vixie CRON. If 5 arguments are provided Crantab is assumed,
//! otherwise Vixie CRON.
//!
//! The following Syntax is supported:
//! - \* any value
//! - , value list
//! - \- range values
//! - / step values
//!
//! ```rust
//! use chrono::{DateTime, TimeZone, Utc};
//! use cron_exp::Schedule;
//! use std::str::FromStr;
//! //               sec  min   hour   day of month   month   day of week   year
//! let expression = "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2";
//!
//! let schedule = Schedule::from_str(expression).unwrap();
//! let mut last: Option<DateTime<Utc>> = None;
//! let from_date = Utc.ymd(2022, 6, 1).and_hms(8, 40, 1);
//!
//! println!("Upcoming fire times:");
//! for datetime in schedule.iter_from(&from_date).take(10) {
//!     last = Some(datetime);
//!     println!("next -> {:?}", datetime);
//! }
//!
//! println!("Previous fire times:");
//! for datetime in schedule.iter_from(&last.unwrap()).rev().take(40) {
//!     println!("prev -> {:?}", datetime);
//! }
//! /*
//! Upcoming fire times:
//! next -> 2022-06-01T09:30:00Z
//! next -> 2022-06-01T12:30:00Z
//! next -> 2022-06-01T15:30:00Z
//! next -> 2022-06-15T09:30:00Z
//! next -> 2022-06-15T12:30:00Z
//! next -> 2022-06-15T15:30:00Z
//! next -> 2022-07-01T09:30:00Z
//! next -> 2022-07-01T12:30:00Z
//! next -> 2022-07-01T15:30:00Z
//! next -> 2022-07-15T09:30:00Z
//!
//! Previous fire times:
//! prev -> 2022-07-01T15:30:00Z
//! prev -> 2022-07-01T12:30:00Z
//! prev -> 2022-07-01T09:30:00Z
//! prev -> 2022-06-15T15:30:00Z
//! prev -> 2022-06-15T12:30:00Z
//! prev -> 2022-06-15T09:30:00Z
//! prev -> 2022-06-01T15:30:00Z
//! prev -> 2022-06-01T12:30:00Z
//! prev -> 2022-06-01T09:30:00Z
//! prev -> 2020-07-15T15:30:00Z
//! */
//! ```
mod errors;
mod schedule;

#[doc(inline)]
pub use errors::ParseScheduleError;

#[doc(inline)]
pub use schedule::Schedule;
