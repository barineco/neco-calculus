#![forbid(unsafe_code)]

use std::{
    env, fs,
    io::{Read, Write},
    process::ExitCode,
};

use calculus_observation_producer::{
    describe, generate_request_batch, self_check_request_batch, AdapterFailure,
};
use calculus_observation_types::{canonical_bytes, canonical_value_bytes};

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(output) => std::io::stdout()
            .lock()
            .write_all(&output)
            .map_or(ExitCode::from(1), |_| ExitCode::SUCCESS),
        Err(error) => match canonical_value_bytes(&error.canonical_value()) {
            Ok(output) => std::io::stderr()
                .lock()
                .write_all(&output)
                .map_or(ExitCode::from(1), |_| ExitCode::from(2)),
            Err(_) => ExitCode::from(1),
        },
    }
}

fn run(arguments: Vec<String>) -> Result<Vec<u8>, AdapterFailure> {
    match arguments.as_slice() {
        [command] if command == "describe" => describe().and_then(|value| {
            canonical_value_bytes(&value)
                .map_err(|failure| AdapterFailure::canonical_serialization("describe", failure))
        }),
        [command, flag, request] if command == "generate" && flag == "--request" => {
            let request = read_input(request)?;
            generate_request_batch(&request).and_then(|batch| {
                canonical_bytes(&batch).map_err(|failure| {
                    AdapterFailure::canonical_serialization("generate", failure)
                })
            })
        }
        [command, request_flag, request, batch_flag, batch]
            if command == "self-check"
                && request_flag == "--request"
                && batch_flag == "--batch" =>
        {
            if request == "-" && batch == "-" {
                return Err(AdapterFailure::command_failure(
                    "self-check",
                    "stdin can provide one input only",
                ));
            }
            let request = read_input(request)?;
            let batch = read_input(batch)?;
            self_check_request_batch(&request, &batch).and_then(|result| {
                canonical_value_bytes(&result).map_err(|failure| {
                    AdapterFailure::canonical_serialization("self-check", failure)
                })
            })
        }
        _ => Err(AdapterFailure::command_failure(
            "command",
            "usage is describe | generate --request <file|-> | self-check --request <file|-> --batch <file|->",
        )),
    }
}

fn read_input(location: &str) -> Result<Vec<u8>, AdapterFailure> {
    if location == "-" {
        let mut bytes = Vec::new();
        std::io::stdin()
            .lock()
            .read_to_end(&mut bytes)
            .map_err(|error| AdapterFailure::process_io_failure("readInput", "stdin", error))?;
        Ok(bytes)
    } else {
        fs::read(location)
            .map_err(|error| AdapterFailure::process_io_failure("readInput", location, error))
    }
}
