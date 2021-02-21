use std::num::ParseIntError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ParseScheduleError {
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),

    #[error("Invalid number of arguments, 5 for Crontab 6 or 7 for Vixie CRON")]
    ArgumentCount,

    #[error("Invalid Step Range {0}")]
    InvalidStepRange(String),

    #[error("Invalid Range {0}")]
    InvalidRange(String),

    #[error("Invalid Month {0}")]
    InvalidMonthIndicator(String),

    #[error("Invalid Day of Week {0}")]
    InvalidDayOfWeekIndicator(String),
}
