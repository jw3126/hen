#![recursion_limit = "1024"]

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
mod app;
mod tests;
mod errors;

use app::app_main;
use std::io::Write;
use error_chain::ChainedError;

use errors::*;

pub fn handle_error(msg: &str) {
    writeln!(&mut std::io::stderr(), "Error:\n{}", msg).expect("Failed writeln! to stderr");
    std::process::exit(1);
    //panic!();
}

fn run() -> Result<()> {
    use std::fs::File;

    // This operation will fail
    File::open("tretrete")
        .chain_err(|| "unable to open tretrete file")?;

    Ok(())
}

fn main() {
    let res : Result<()> = app_main()
        .map_err(|e|e.into());
    let res : Result<()> = res.map_err(|e|e.into());
    let res = res.chain_err(||"oh");
    if let Err(e) = res {
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";
        writeln!(stderr, "{}", e.display_chain()).expect(errmsg);
        std::process::exit(1);
    }
}
