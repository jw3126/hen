extern crate num_cpus;
extern crate rand;
extern crate rayon;
extern crate regex;
extern crate sha3;

#[cfg(test)]
extern crate assert_cli;

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

// is there a cleaner way to do these tests?
mod tests;

use app::app_main;

fn main() {
    match app_main() {
        Ok(_) => {}
        Err(err) => {
            let msg = format!("Error {:?}. Try hen --help", err);
            panic!(msg)
        }
    }
}
