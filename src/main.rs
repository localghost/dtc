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
    let timezones = build_timezone_db();
    if let Some(timezone) = timezones.get(&timezone.to_lowercase()) {
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
        let datetime = tz.from_utc_datetime(&utc_now);
        timezones.insert(datetime.offset().abbreviation().to_lowercase(), tz);
    }
    timezones
}

fn main() {
    let args = Args::parse();
    unsafe {
        VERBOSE = args.verbose;
    }

    let timezones = build_timezone_db();
    let dest_tz = match timezones.get(&args.dest_tz.to_lowercase()) {
        Some(tz) => tz,
        None => {
            eprintln!(
                "Destination timezone {} could not be found in the timezone database",
                args.dest_tz
            );
            exit(1);
        }
    };

    let datetime_parsed = parse(&args.datetime)
        .unwrap_or_else(|()| panic!("Could not parse {}", args.datetime));
    verbose(&datetime_parsed.to_string());
    println!("{datetime_parsed}");

    let datetime_converted = datetime_parsed.fixed_offset().with_timezone(dest_tz);
    println!("{datetime_converted}");
}
