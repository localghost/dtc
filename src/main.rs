use chrono::{offset, DateTime, FixedOffset, Local, NaiveDateTime, NaiveTime, ParseResult};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg()]
    datetime: String,

    #[arg(long = "from", help = "Use this timezone if none found in DATETIME")]
    src_tz: Option<String>,

    #[arg(default_value_t = String::from("gmt"))]
    dest_tz: String,
}

const FORMATS: &[&str] = &["%Y-%m-%d %H:%M:%S"];

fn parse_datetime(datetime: &str, with_timezone: bool) -> Result<DateTime<FixedOffset>, ()> {
    let datetime = if NaiveTime::parse_and_remainder(datetime, "%H:%M:%S").is_ok() {
        format!(
            "{} {datetime}",
            Local::now().date_naive().format("%Y-%m-%d")
        )
        .into()
    } else {
        std::borrow::Cow::from(datetime)
    };

    for format in FORMATS {
        if with_timezone {
            match NaiveDateTime::parse_from_str(&datetime, format) {
                ParseResult::Ok(result) => {
                    // or .and_local_timezone() but it requires figuring out local timezone
                    return Ok(result.and_utc().into());
                }
                ParseResult::Err(e) => {
                    dbg!(e);
                }
            }
        }
        let format = if with_timezone {
            format!("{format} %z").into()
        } else {
            std::borrow::Cow::from(*format)
        };
        dbg!(&format);
        match DateTime::parse_from_str(&datetime, &format) {
            ParseResult::Ok(result) => {
                return Ok(result);
            }
            ParseResult::Err(e) => {
                dbg!(e);
            }
        }
    }
    if with_timezone {
        if let ParseResult::Ok(result) = DateTime::parse_from_rfc3339(&datetime) {
            return Ok(result);
        }
        return DateTime::parse_from_rfc2822(&datetime).map_err(|_e| ());
    }

    Err(())
}

fn main() {
    let args = Args::parse();
    dbg!(&args);

    let datetime_parsed = parse_datetime(&args.datetime, args.src_tz.is_none())
        .unwrap_or_else(|()| panic!("Could not parse {}", args.datetime));
    dbg!(datetime_parsed);

    let datetime_converted = datetime_parsed.with_timezone(&offset::Utc);
    println!("{datetime_converted}");
}
