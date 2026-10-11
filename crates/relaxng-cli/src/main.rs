use std::env;
use std::fs;
use std::process::ExitCode;

fn usage() {
    eprintln!(
        "usage: relaxng validate schema.rng document.xml\n       relaxng check-schema schema.rng"
    );
}

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let Some(command) = arguments.next() else {
        usage();
        return ExitCode::from(2);
    };
    let Some(schema_path) = arguments.next() else {
        usage();
        return ExitCode::from(2);
    };
    match command.as_str() {
        "check-schema" => match relaxng_syntax::parse_file(&schema_path) {
            Ok(_) => {
                println!("schema is valid");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("schema error: {error}");
                ExitCode::from(1)
            }
        },
        "validate" => {
            let Some(document_path) = arguments.next() else {
                usage();
                return ExitCode::from(2);
            };
            let schema = match relaxng_syntax::parse_file(&schema_path) {
                Ok(schema) => schema,
                Err(error) => {
                    eprintln!("schema error: {error}");
                    return ExitCode::from(1);
                }
            };
            let document = match fs::read_to_string(&document_path) {
                Ok(document) => document,
                Err(error) => {
                    eprintln!("document error: {error}");
                    return ExitCode::from(1);
                }
            };
            match relaxng_validator::Validator::new().validate_str(&schema, &document) {
                Ok(()) => {
                    println!("document is valid");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}
