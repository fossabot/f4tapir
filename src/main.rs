mod args;
mod detect;
mod find;
mod merge;
mod paths;
mod split;
mod timestamp;
mod transcript;

use args::{Invocation, TopLevel};

fn main() {
    stderrlog::new().verbosity(1).init().unwrap();
    match run(argh::from_env()) {
        Ok(_) => (),
        Err(msg) => {
            eprintln!("error: {}", msg);
            std::process::exit(1);
        }
    }
}

fn run(invocation: TopLevel) -> Result<(), String> {
    match invocation.invocation {
        Invocation::Split(opts) => split::split(opts).map_err(|e| format!("{}", e)),
        Invocation::Merge(opts) => merge::merge(opts).map_err(|e| format!("{}", e)),
    }
}
