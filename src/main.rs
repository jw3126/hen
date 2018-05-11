extern crate num_cpus;
extern crate rand;
extern crate rayon;
extern crate regex;
extern crate sha3;

#[allow(unused_imports)]
#[macro_use]
extern crate assert_cli;

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
