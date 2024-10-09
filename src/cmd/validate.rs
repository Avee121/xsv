use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

use crate::config::Delimiter;
use crate::util;
use crate::CliResult;

static USAGE: &'static str = "
Validate a CSV file for common errors.

Errors are reported in the format <line no> <expected delimiters> <actual delimiters> <data>

Usage:
    xsv val [options] [<input>]

input options:
    --quote <arg>          The quote character to use. [default: \"]
    --no-quoting           Disable quoting completely.

Common options:
    -h, --help             Display this message
    -o, --output <file>    Write output to <file> instead of stdout.
    -d, --delimiter <arg>  The field delimiter for reading CSV data.
                           Must be a single character. (default: ,)
";

#[derive(Deserialize)]
struct Args {
    arg_input: String,
    flag_output: Option<String>,
    flag_delimiter: Option<Delimiter>,
    flag_quote: Option<Delimiter>,
    flag_no_quoting: bool,
}

pub fn run(argv: &[&str]) -> CliResult<()> {
    let args: Args = util::get_args(USAGE, argv).unwrap();

    let delim_arg = if let Some(delim) = args.flag_delimiter {
        delim.as_byte()
    } else {
        b','
    };

    let qual_char = if let Some(qual) = args.flag_quote {
        qual.as_byte()
    } else {
        b'"'
    };

    let qual = if args.flag_no_quoting {
        None
    } else {
        Some(qual_char)
    };

    let res = validate_file(delim_arg, qual, !args.flag_no_quoting, &args.arg_input);

    match res {
        Ok(_) => {
            println!("File is valid");
            Ok(())
        }
        Err(e) => {
            if let Some(output) = args.flag_output {
                let _ = File::create(output).map(move |mut f| {
                    writeln!(f, "Line_Number,Expected_Delimiters,Actual_Delimiters,Data")
                        .expect("Error writing to file");
                    e.into_iter()
                        .for_each(|s| writeln!(f, "{}", s).expect("Error writing to file"))
                });
            } else {
                println!("Line_Number,Expected_Delimiters,Actual_Delimiters,Data");
                e.into_iter().for_each(|s| println!("{}", s));
            }
            Err("File is invalid".into())
        }
    }
}

fn validate_file(
    delim: u8,
    qual: Option<u8>,
    is_quoted: bool,
    file_path: &str,
) -> Result<(), Vec<String>> {
    let filepath = validate_path(file_path);

    if let Err(e) = filepath {
        return Err(vec![e]);
    }

    let file: File = File::open::<&Path>(filepath.unwrap())
        .map_err(|e| Vec::from([format!("Error opening file: {}", e)]))?;

    let mut reader = BufReader::new(file);

    if is_quoted {
        validate_quoted(&mut reader, delim, qual.unwrap())
    } else {
        validate_unquoted(&mut reader, delim)
    }
}

fn validate_path(path: &str) -> Result<&Path, String> {
    let path = Path::new(path);
    if path.exists() && path.is_file() {
        Ok(path)
    } else {
        Err(format!(
            "Path not found or filepath is not a file: {}",
            path.display()
        ))
    }
}

fn validate_quoted(reader: &mut BufReader<File>, delim: u8, qual: u8) -> Result<(), Vec<String>> {
    let mut qual_flag: bool = false;
    let mut delim_count: usize = 0;

    let mut errs = Vec::new();
    //set expected delims

    let mut line = String::new();
    reader.read_line(&mut line).expect("Error reading line");
    let iter = line.bytes();

    for ch in iter {
        if ch == delim && !qual_flag {
            delim_count += 1;
        } else if ch == qual {
            qual_flag = !qual_flag;
        }
    }

    let expected_delims = delim_count;
    delim_count = 0;
    qual_flag = false;

    for (i, line_result) in reader.lines().enumerate() {
        let line = line_result.expect("Error reading line");

        for ch in line.bytes() {
            match ch {
                _ if ch == delim => {
                    if !qual_flag {
                        delim_count += 1;
                    }
                }
                _ if ch == qual => {
                    qual_flag = !qual_flag;
                }
                _ => {}
            }
        }
        if delim_count != expected_delims {
            errs.push(fmt_error(i + 1, expected_delims, delim_count, &line));
        }
        delim_count = 0;
        qual_flag = false;
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

fn validate_unquoted(reader: &mut BufReader<File>, delim: u8) -> Result<(), Vec<String>> {
    let mut delim_count: usize = 0;

    let mut errs = Vec::new();

    //set expected delims

    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("Error reading line from file");
    let iter = line.bytes();

    for ch in iter {
        if ch == delim {
            delim_count += 1;
        }
    }

    let expected_delims = delim_count;
    delim_count = 0;

    for (i, line_result) in reader.lines().enumerate() {
        let line = line_result.expect("Error reading line");

        for ch in line.bytes() {
            match ch {
                _ if ch == delim => {
                    delim_count += 1;
                }

                _ => {}
            }
        }
        if delim_count != expected_delims {
            errs.push(fmt_error(i + 1, expected_delims, delim_count, &line));
        }
        delim_count = 0;
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

fn fmt_error(line_no: usize, expected: usize, actual: usize, data: &str) -> String {
    format!("{},{},{},\"{}\"", line_no, expected, actual, data)
}
