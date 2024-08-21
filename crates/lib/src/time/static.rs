use std::fmt;
use std::str::FromStr;

use chrono::{offset::Utc, DateTime, NaiveDate, NaiveTime};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::traits::Api;
use crate::Error;

static STATIC_DATE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?<year>\d\d\d\d)(-(?<month>\d\d))?(-(?<day>\d\d))?$").unwrap());

#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone, PartialEq, Eq)]
pub struct TimeStatic {
    raw: String,
    value: DateTime<Utc>,
}

impl FromStr for TimeStatic {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let value = if s == "now" {
            Utc::now()
        } else if let Some(captures) = STATIC_DATE_RE.captures(s) {
            let year = captures.name("year").map(|m| m.as_str()).unwrap();
            let year = year
                .parse()
                .map_err(|e| Error::InvalidValue(format!("invalid year: {year}: {e}")))?;
            let month = captures.name("month").map_or("1", |m| m.as_str());
            let month = month
                .parse()
                .map_err(|e| Error::InvalidValue(format!("invalid month: {month}: {e}")))?;
            let day = captures.name("day").map_or("1", |m| m.as_str());
            let day = day
                .parse()
                .map_err(|e| Error::InvalidValue(format!("invalid day: {day}: {e}")))?;
            NaiveDate::from_ymd_opt(year, month, day)
                .ok_or_else(|| Error::InvalidValue(format!("invalid date: {s}")))?
                .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .and_utc()
        } else {
            DateTime::from_str(s)
                .map_err(|e| Error::InvalidValue(format!("invalid datetime format: {s}: {e}")))?
        };

        Ok(Self {
            raw: s.to_string(),
            value,
        })
    }
}

impl fmt::Display for TimeStatic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl AsRef<str> for TimeStatic {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

impl Api for TimeStatic {
    fn api(&self) -> String {
        self.value.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}
