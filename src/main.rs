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

#[macro_use]
extern crate error_chain;

mod tokenizer;
mod simulation;
mod output_parser;
mod util;
mod uncertain;
mod omittable;
mod app;
mod errors;

#[cfg(test)]
mod tests;

use app::app_main;
use std::io::Write;
use errors::*;
use error_chain::ChainedError;

fn main() {    
    let res : Result<()> = app_main()
        .chain_err(||"Hen failed");
    if let Err(e) = res {
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";
        writeln!(stderr, "{}", e.display_chain()).expect(errmsg);
        std::process::exit(1);
    };
}
