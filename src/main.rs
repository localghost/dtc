#[macro_use]
extern crate lazy_static;

use std::{collections::HashMap, process::exit};

use chrono::{
    DateTime, FixedOffset, Local, NaiveDateTime, NaiveTime, Offset, ParseResult, TimeZone, Utc,
};
use chrono_tz::OffsetName;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg()]
    datetime: String,

    #[arg(default_value_t = String::from("gmt"), help="Timezone to convert to")]
    dest_tz: String,

    #[arg(short, long)]
    verbose: bool,
}

const FORMATS: &[&str] = &["%s", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"];

static mut VERBOSE: bool = false;

lazy_static! {
    static ref TIMEZONES_DB: HashMap<String, chrono_tz::Tz> = build_timezone_db();
}

fn verbose(message: &str) {
    // This is set at the start of the program and never modified again.
    if unsafe { VERBOSE } {
        println!("{message}");
    }
}

fn parse(datetime: &str) -> Result<DateTime<FixedOffset>, ()> {
    // If only time is provided then prefix it with local date.
    let datetime = if NaiveTime::parse_and_remainder(datetime, "%H:%M:%S").is_ok() {
        verbose("Date not provieded, assuming today.");
        format!(
            "{} {datetime}",
            Local::now().date_naive().format("%Y-%m-%d")
        )
        .into()
    } else {
        std::borrow::Cow::from(datetime)
    };

    parse_datetime(&datetime)
}

// First tries to parse timezone using standard format. If that fails it tries to match the timezone
// against the timezone database.
// The `datetime` is needed to establish if e.g. for the timezone a daylight saving time should be
// applied.
fn parse_timezone(datetime: DateTime<Utc>, timezone: &str) -> Result<FixedOffset, ()> {
    let datetime_str = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
    for format in &["%#z", "%:z", "%::z", "%Z"] {
        verbose(&format!("Trying out format {format}"));
        let format = format!("%Y-%m-%d %H:%M:%S {format}");
        let datetime_str = format!("{} {}", datetime_str, timezone.to_uppercase());
        match DateTime::parse_from_str(&datetime_str, &format) {
            ParseResult::Ok(datetime) => {
                return Ok(*datetime.offset());
            }
            ParseResult::Err(e) => {
                verbose(&("Error: ".to_string() + &e.to_string()));
            }
        }
    }

    if let Some(timezone) = TIMEZONES_DB.get(&timezone.to_lowercase()) {
        return Ok(datetime.with_timezone(timezone).offset().fix());
    }

    Err(())
}

fn parse_datetime(datetime: &str) -> Result<DateTime<FixedOffset>, ()> {
    for format in FORMATS {
        verbose(&format!("Trying out format {format}"));
        match NaiveDateTime::parse_and_remainder(datetime, format) {
            ParseResult::Ok((datetime, remainder)) => {
                let remainder = remainder.trim();
                // Assume UTC if no timezone is provided
                let datetime = if remainder.is_empty() {
                    datetime.and_utc().fixed_offset()
                } else if remainder.to_lowercase() == "local" {
                    datetime.and_local_timezone(Local).unwrap().fixed_offset()
                } else if let Ok(offset) = parse_timezone(datetime.and_utc(), remainder) {
                    datetime.and_local_timezone(offset).unwrap().fixed_offset()
                } else {
                    continue;
                };
                return Ok(datetime);
            }
            ParseResult::Err(e) => {
                verbose(&("Error: ".to_string() + &e.to_string()));
            }
        }
    }
    Err(())
}

fn build_timezone_db() -> HashMap<String, chrono_tz::Tz> {
    let mut timezones =
        HashMap::<String, chrono_tz::Tz>::with_capacity(chrono_tz::TZ_VARIANTS.len());
    let utc_now = Utc::now().naive_utc();
    for tz in chrono_tz::TZ_VARIANTS {
        timezones.insert(
            tz.from_utc_datetime(&utc_now)
                .offset()
                .abbreviation()
                .to_lowercase(),
            tz,
        );
    }
    timezones
}

fn convert(datetime: &DateTime<FixedOffset>, timezone: &chrono_tz::Tz) -> DateTime<chrono_tz::Tz> {
    datetime.fixed_offset().with_timezone(timezone)
}

fn main() {
    let args = Args::parse();
    unsafe {
        VERBOSE = args.verbose;
    }

    let dest_tz = match TIMEZONES_DB.get(&args.dest_tz.to_lowercase()) {
        Some(tz) => tz,
        None => {
            eprintln!(
                "Destination timezone {} could not be found in the timezone database",
                args.dest_tz
            );
            exit(1);
        }
    };

    let datetime_parsed =
        parse(&args.datetime).unwrap_or_else(|()| panic!("Could not parse {}", args.datetime));
    verbose(&datetime_parsed.to_string());

    println!("{}", convert(&datetime_parsed, dest_tz));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timestamp() {
        assert_eq!(
            "2023-10-22T21:38:20+00:00",
            parse("1698010700").unwrap().to_rfc3339()
        );
    }

    #[test]
    fn parse_rfc3339() {
        let datetime = "2023-10-22T10:34:16+01:00";
        assert_eq!(datetime, parse(datetime).unwrap().to_rfc3339());
    }

    #[test]
    fn parse_timezone_abbreviation() {
        let datetime = "2023-10-22 10:34:16 jst";
        assert_eq!(
            "2023-10-22 10:34:16 +09:00",
            parse(datetime).unwrap().to_string()
        )
    }

    #[test]
    fn parse_without_timezone() {
        let datetime = "2023-10-22 10:34:16";
        assert_eq!(
            "2023-10-22T10:34:16+00:00",
            parse(datetime).unwrap().to_rfc3339()
        )
    }

    #[test]
    fn convert_datetime_to_utc() {
        let datetime = DateTime::parse_from_rfc3339("2023-10-22T10:34:16+02:00").unwrap();
        assert_eq!(
            Into::<DateTime<Utc>>::into(datetime).to_rfc3339(),
            convert(&datetime, &chrono_tz::Tz::UTC).to_rfc3339()
        );
    }
}
