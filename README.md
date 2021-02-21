# cron-exp  &emsp; [![Build Status]][ghactions] [![Latest Version]][crates.io]

[Build Status]: https://github.com/rust-playground/cron-exp/actions/workflows/rust.yml/badge.svg
[ghactions]: https://github.com/rust-playground/cron-exp/actions/workflows/rust.yml/badge.svg
[Latest Version]: https://img.shields.io/crates/v/cron_exp.svg
[crates.io]: https://crates.io/crates/cron_exp

A CRON expression parser and explorer.
It is designed for space efficiency for caching and storage purposes such as in a CRON Scheduler.

### Example
```rust
use chrono::{DateTime, TimeZone, Utc};
use cron_rs::schedule::Schedule;
use std::str::FromStr;

fn main() {
    //               sec  min   hour   day of month   month   day of week   year
    let expression = "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2";
    let schedule = Schedule::from_str(expression).unwrap();

    let mut last: Option<DateTime<Utc>> = None;
    let from_date = Utc.ymd(2022, 6, 1).and_hms(8, 40, 1);

    println!("Upcoming fire times:");
    for datetime in schedule.iter_from(&from_date).take(10) {
        last = Some(datetime);
        println!("next -> {:?}", datetime);
    }

    println!("\nPrevious fire times:");
    for datetime in schedule.iter_from(&last.unwrap()).rev().take(10) {
        println!("prev -> {:?}", datetime);
    }
}

/*
Upcoming fire times:
next -> 2022-06-01T09:30:00Z
next -> 2022-06-01T12:30:00Z
next -> 2022-06-01T15:30:00Z
next -> 2022-06-15T09:30:00Z
next -> 2022-06-15T12:30:00Z
next -> 2022-06-15T15:30:00Z
next -> 2022-07-01T09:30:00Z
next -> 2022-07-01T12:30:00Z
next -> 2022-07-01T15:30:00Z
next -> 2022-07-15T09:30:00Z

Previous fire times:
prev -> 2022-07-01T15:30:00Z
prev -> 2022-07-01T12:30:00Z
prev -> 2022-07-01T09:30:00Z
prev -> 2022-06-15T15:30:00Z
prev -> 2022-06-15T12:30:00Z
prev -> 2022-06-15T09:30:00Z
prev -> 2022-06-01T15:30:00Z
prev -> 2022-06-01T12:30:00Z
prev -> 2022-06-01T09:30:00Z
prev -> 2020-07-15T15:30:00Z
*/

```

### Crontab:

```
# ┌─────────────────────  minute (0 - 59)
# │ ┌───────────────────  hour   (0 - 23)
# │ │ ┌─────────────────  dom    (1 - 31) day of month
# │ │ │ ┌───────────────  month  (1 - 12 or Jan-Dec)
# │ │ │ │ ┌─────────────  dow    ((0 or 7) - 6 or Sun - Sat)  day of week (Sunday to Saturday)
# │ │ │ │ │
# │ │ │ │ │
# │ │ │ │ │
# * * * * * <command to execute>
```

| Field        | Required | Allowed values        | Allowed special characters |
| ------------ | -------- | ----------------------| -------------------------- |
| Minutes      | Yes      | 0–59                  | \* , - /                   |
| Hours        | Yes      | 0–23                  | \* , - /                   |
| Day of month | Yes      | 1–31                  | \* , - /                   |
| Month        | Yes      | 1–12 or Jan-Dec       | \* , - /                   |
| Day of week  | Yes      | (0 or 7)–6 or Sun-Sat | \* , - /                   |

### Vixie CRON:

```
# ┌───────────────────────  seconds (0 - 59)
# │ ┌─────────────────────  minute  (0 - 59)
# │ │ ┌───────────────────  hour    (0 - 23)
# │ │ │ ┌─────────────────  dom     (1 - 31) day of month
# │ │ │ │ ┌───────────────  month   (1 - 12 or Jan-Dec)
# │ │ │ │ │ ┌─────────────  dow     (1-7 or Sun-Sat)  day of week (Sunday to Saturday)
# │ │ │ │ │ │ ┌──────────── year    (1970-2099 Optional)
# │ │ │ │ │ │ │
# │ │ │ │ │ │ │
# * * * * * * *
```

| Field        | Required | Allowed values  | Allowed special characters |
| ------------ | -------- | --------------- | -------------------------- |
| Seconds      | Yes      | 0–59            | \* , - /                   |
| Minutes      | Yes      | 0–59            | \* , - /                   |
| Hours        | Yes      | 0–23            | \* , - /                   |
| Day of month | Yes      | 1–31            | \* , - /                   |
| Month        | Yes      | 1–12 or Jan-Dec | \* , - /                   |
| Day of week  | Yes      | 1–7 or Sun-Sat  | \* , - /                   |
| Years        | No       | 1970-2099       | \* , - /                   |

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Proteus by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
</sub>
