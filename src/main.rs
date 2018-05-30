extern crate itertools;
extern crate num_cpus;
extern crate rand;
extern crate rayon;
extern crate regex;
extern crate sha3;

#[cfg(test)]
extern crate assert_cli;

#[cfg(test)]
extern crate tempfile;

#[cfg(test)]
#[macro_use]
extern crate approx;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

#[macro_use]
extern crate clap;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

mod tokenizer;
mod simulation;
mod output_parser;
mod util;
mod uncertain;
mod app;

#[cfg(test)]
mod tests;

use app::app_main;
use std::io::Write;

pub fn error(msg: &str) {
    writeln!(&mut std::io::stderr(), "Error:\n{}", msg).expect("Failed writeln! to stderr");
    std::process::exit(1);
    //panic!();
}

fn main() {
    match app_main() {
        Ok(_) => {}
        Err(err) => error(&err),
    }
}
