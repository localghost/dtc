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

const FORMATS: &[&str] = &["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"];

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

// fn parse_with_forced_timezone(
//     datetime: &str,
//     timezone: chrono_tz::Tz,
// ) -> Result<DateTime<FixedOffset>, ()> {
//     for format in FORMATS {
//         verbose(&format!("Trying out format {format}"));
//         match NaiveDateTime::parse_and_remainder(datetime, format) {
//             ParseResult::Ok((datetime, _)) => {
//                 return Ok(datetime
//                     .and_local_timezone(timezone)
//                     .unwrap()
//                     .fixed_offset());
//             }
//             ParseResult::Err(e) => {
//                 verbose(&("Error: ".to_string() + &e.to_string()));
//             }
//         }
//     }
//     Err(())
// }

// fn parse(datetime: &str) -> Result<DateTime<FixedOffset>, ()> {
//     for format in FORMATS {
//         verbose(&format!("Trying out format {format}"));
//         // Try parsing without timezone first and if it succeeds assume this is in UTC.
//         match NaiveDateTime::parse_from_str(datetime, format) {
//             ParseResult::Ok(datetime) => {
//                 verbose("Timezone not provided in the datetime string, assuming UTC.");
//                 return Ok(datetime.and_utc().into());
//             }
//             ParseResult::Err(e) => {
//                 verbose(&("Error: ".to_string() + &e.to_string()));
//             }
//         }
//
//         let format = format!("{format} %z");
//         verbose(&format!("Trying out format {format}"));
//         match DateTime::parse_from_str(datetime, &format) {
//             ParseResult::Ok(result) => {
//                 return Ok(result);
//             }
//             ParseResult::Err(e) => {
//                 verbose(&("Error: ".to_string() + &e.to_string()));
//             }
//         }
//     }
//     if let ParseResult::Ok(result) = DateTime::parse_from_rfc3339(datetime) {
//         return Ok(result);
//     }
//     if let ParseResult::Ok(result) = DateTime::parse_from_rfc2822(datetime) {
//         return Ok(result);
//     }
//
//     Err(())
// }

fn parse(datetime: &str) -> Result<DateTime<FixedOffset>, ()> {
    // Check if only time is provided, either with a timezone or not. If it is prefix it with local
    // date.
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

fn parse_timezone(datetime: DateTime<Utc>, timezone: &str) -> Result<FixedOffset, ()> {
    let datetime_str = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
    for format in &["%#z", "%:z", "%::z", "%Z"] {
        verbose(&format!("Trying out format {format}"));
        match DateTime::parse_from_str(
            &format!("{} {}", datetime_str, timezone.to_uppercase()),
            &format!("%Y-%m-%d %H:%M:%S {format}"),
        ) {
            ParseResult::Ok(datetime) => {
                return Ok(*datetime.offset());
            }
            ParseResult::Err(e) => {
                verbose(&("Error: ".to_string() + &e.to_string()));
            }
        }
    }
    if let Some(timezone) = TIMEZONES_DB.get(&timezone.to_lowercase()) {
        let datetime = datetime.with_timezone(timezone);
        return Ok(datetime.with_timezone(timezone).offset().fix());
    }

    Err(())
}

fn parse_datetime(datetime: &str) -> Result<DateTime<FixedOffset>, ()> {
    for format in FORMATS {
        verbose(&format!("Trying out format {format}"));
        match NaiveDateTime::parse_and_remainder(datetime, format) {
            ParseResult::Ok((datetime, remainder)) => {
                // TODO: Use local timezone if not provided.
                return Ok(datetime
                    .and_local_timezone(
                        parse_timezone(datetime.and_utc(), remainder.trim()).unwrap(),
                    )
                    .unwrap()
                    .fixed_offset());
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
