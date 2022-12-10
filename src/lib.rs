use std::io::{self, BufRead, BufReader};
use std::{error::Error, vec};

use clap::{App, Arg};
use regex::{Regex, RegexBuilder};
use walkdir::WalkDir;

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
            Arg::with_name(PATTERN)
                .value_name("PATTERN")
                .help("Search pattern")
                .required(true),
        )
        .arg(
            Arg::with_name(FILE)
                .value_name("FILE")
                .help("Input file(s)")
                .default_value("-")
                .multiple(true),
        )
        .arg(
            Arg::with_name(COUNT)
                .help("Count occurences.")
                .short("c")
                .long("count")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(INSENSITIVE)
                .help("Case-insensitive")
                .short("i")
                .long("insensitive")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(INVERT_MATCH)
                .help("Invert match")
                .short("v")
                .long("invert-match")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(RECURSIVE)
                .help("Recursive search")
                .short("r")
                .long("recursive")
                .takes_value(false),
        )
        .get_matches();

    let pattern_str = matches.value_of(PATTERN).unwrap();
    let pattern = RegexBuilder::new(pattern_str)
        .case_insensitive(matches.is_present(INSENSITIVE))
        .build()
        .map_err(|_| format!("Invalid pattern \"{}\"", pattern_str))?;

    Ok(Config {
        pattern,
        files: matches.values_of_lossy(FILE).unwrap(),
        recursive: matches.is_present(RECURSIVE),
        count: matches.is_present(COUNT),
        invert_match: matches.is_present(INVERT_MATCH),
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let file_paths = find_files(&config.files, config.recursive);
    let many_files = file_paths.len() > 1;

    for path in file_paths {
        match path {
            Err(e) => eprintln!("{}", e),
            Ok(path) => match open(&path) {
                Err(e) => eprintln!("{}: {}", path, e),
                Ok(file) => {
                    let matches = find_lines(file, &config.pattern, config.invert_match)?;

                    if config.count {
                        if many_files {
                            print!("{}:", path);
                        }

                        println!("{}", matches.len());
                        continue;
                    }

                    for match_ in &matches {
                        if many_files {
                            print!("{}:", path);
                        }

                        print!("{}", match_);
                    }
                }
            },
        }
    }

    Ok(())
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(std::fs::File::open(filename)?))),
    }
}

fn find_files(paths: &[String], recursive: bool) -> Vec<MyResult<String>> {
    let mut res = vec![];
    for path in paths {
        if path == "-" {
            res.push(Ok(path.to_owned()));
            continue;
        }

        for dir_entry in WalkDir::new(path) {
            match dir_entry {
                Err(e) => res.push(Err(From::from(format!(
                    "{}: {}",
                    path,
                    e.io_error().unwrap().to_string()
                )))),
                Ok(dir) => {
                    if dir.file_type().is_dir() && recursive == false {
                        res.push(Err(From::from(format!("{} is a directory", path))));
                        break;
                    }

                    if dir.file_type().is_file() {
                        res.push(Ok(dir.path().display().to_string()));
                    }
                }
            };
        }
    }

    res
}

fn find_lines<T: BufRead>(
    mut file: T,
    pattern: &Regex,
    invert_match: bool,
) -> MyResult<Vec<String>> {
    let mut res = vec![];

    let mut buffer = String::new();
    while let Ok(bytes) = file.read_line(&mut buffer) {
        if bytes == 0 {
            break;
        }

        if invert_match ^ pattern.is_match(&buffer) {
            res.push(buffer.to_string());
        }

        buffer.clear();
    }

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::{find_files, find_lines};
    use rand::{distributions::Alphanumeric, Rng};
    use regex::{Regex, RegexBuilder};
    use std::io::Cursor;

    #[test]
    fn test_find_files() {
        // Verify that the function finds a file known to exist
        let files = find_files(&["./tests/inputs/fox.txt".to_string()], false);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].as_ref().unwrap(), "./tests/inputs/fox.txt");

        // The function should reject a directory without the recursive option
        let files = find_files(&["./tests/inputs/".to_string()], false);
        assert_eq!(files.len(), 1);
        if let Err(e) = &files[0] {
            assert_eq!(e.to_string(), "./tests/inputs/ is a directory");
        }

        // Verify the function recurses to find four files in the directory
        let res = find_files(&["./tests/inputs/".to_string()], true);
        let files = res
            .iter()
            .map(|r| r.as_ref().unwrap().replace("\\", "/"))
            .collect::<Vec<String>>();

        assert_eq!(files.len(), 4);

        // Generate a random string to represent a nonexistent file
        let bad: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        // Verify that the function returns the bad file as an error
        let files = find_files(&[bad], false);
        assert_eq!(files.len(), 1);
        assert!(files[0].is_err());
    }

    #[test]
    fn test_find_lines() {
        let text = b"Lorem\nIpsum\r\nDOLOR";

        // Pattern _or_ should match the one line, "Lorem"
        let re1 = Regex::new("or").unwrap();
        let matches = find_lines(Cursor::new(&text), &re1, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);

        // When inverted, the function should match the other two lines
        let matches = find_lines(Cursor::new(&text), &re1, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);

        // This regex will be case-insensitive
        let re2 = RegexBuilder::new("or")
            .case_insensitive(true)
            .build()
            .unwrap();

        // The two lines "Lorem" and "DOLOR" should match
        let matches = find_lines(Cursor::new(&*text), &re2, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);

        // When inverted, the one remaining line should match
        let matches = find_lines(Cursor::new(&text), &re2, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);
    }
}
