use std::error::Error;

use regex::{Regex, RegexBuilder};
use clap::{App, Arg};

const PATTERN: &'static str = "pattern";
const FILE: &'static str = "file";
const RECURSIVE: &'static str = "recursive";
const INVERT_MATCH: &'static str = "invert-match";
const COUNT: &'static str = "count";
const INSENSITIVE: &'static str = "insensitive";

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    pattern: Regex,
    files: Vec<String>,
    recursive: bool,
    count: bool,
    invert_match: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("grepr")
        .version("0.1.0")
        .author("Myron Lioz <liozmyron@gmail.com>")
        .about("Rust grep")
        .arg(
            Arg::with_name(PATTERN).value_name("PATTERN").help("Search pattern").required(true)
        ).arg(
            Arg::with_name(FILE).value_name("FILE").help("Input file(s)").default_value("-").multiple(true)
        ).arg(
            Arg::with_name(COUNT).help("Count occurences.").short("c").long("count").takes_value(false)
        ).arg(
            Arg::with_name(INSENSITIVE).help("Case-insensitive").short("i").long("insensitive").takes_value(false)
        ).arg(
            Arg::with_name(INVERT_MATCH).help("Invert match").short("v").long("invert-match").takes_value(false)
        ).arg(
            Arg::with_name(RECURSIVE).help("Recursive search").short("r").long("recursive").takes_value(false)
        )
        .get_matches();

    let pattern_str = matches.value_of(PATTERN).unwrap();
    let pattern = RegexBuilder::new(pattern_str)
        .case_insensitive(matches.is_present(INSENSITIVE))
        .build()
        .map_err(|_| -> String { From::from(format!("Invalid pattern: \"{}\"", pattern_str)) })?;

    Ok(Config { 
        pattern, 
        files: matches.values_of_lossy(FILE).unwrap(), 
        recursive: matches.is_present(RECURSIVE), 
        count: matches.is_present(COUNT), 
        invert_match: matches.is_present(INVERT_MATCH) 
    })
}

pub fn run(config: Config) -> MyResult<()> {
    println!("{:#?}", config);
    Ok(())
}