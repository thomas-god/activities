use std::env;

use activities::{parse_activity, parse_records};

enum Mode {
    Raw,
    Activity,
}

fn parse_args(args: &[String]) -> Result<(String, Mode), String> {
    if args.len() < 4 {
        return Err("Missing required parameters".to_string());
    }

    let mut file: Option<String> = None;
    let mut mode = Mode::Activity;

    let mut iter = args.iter();
    let _ = iter.next(); // rust bin
    let _ = iter.next(); // main.rs

    for arg in iter {
        match arg.as_str() {
            "--activity" => mode = Mode::Activity,
            "--raw" => mode = Mode::Raw,
            arg if arg.starts_with('-') => return Err(format!("Unknown flag: {}", arg)),
            _ => {
                if file.is_some() {
                    return Err("Multiple input parameters provided".to_string());
                }
                file = Some(arg.clone());
            }
        }
    }

    if file.is_none() {
        return Err("Missing required file input parameter".to_string());
    }

    Ok((file.unwrap(), mode))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (file, mode) = match parse_args(&args) {
        Ok((file, mode)) => (file, mode),
        Err(err) => {
            println!("{err:?}");
            return;
        }
    };

    match mode {
        Mode::Raw => {
            let records = parse_records(&file).unwrap();
            for record in records.iter() {
                println!("{record:?}");
            }
        }
        Mode::Activity => {
            let activity = parse_activity(&file).unwrap();
            println!("{activity:?}");
        }
    }
}
