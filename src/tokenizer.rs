use std::io::{BufRead, BufReader};
use std::option::Option;
use std::iter::Iterator;
use simulation::Seed;
use errors::*;

use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Start(String),
    Stop(String),
    KeyValue(String, String),
}

impl Token {
    pub fn parse(s: &str) -> Result<Token> {
        let kv = Token::parse_key_value(s);
        if kv != None {
            return Ok(kv.unwrap());
        };
        let st = Token::parse_start(s);
        if st != None {
            return Ok(st.unwrap());
        };
        let sp = Token::parse_stop(s);
        if sp != None {
            return Ok(sp.unwrap());
        };
        bail!("Cannot parse {}", s);
    }

    fn parse_start(s: &str) -> Option<Token> {
        let re = Regex::new("^:start (.*):$").unwrap();
        let caps = re.captures(&s)?;
        let tok = Token::Start(caps.get(1).unwrap().as_str().trim().to_string());
        Some(tok)
    }

    fn parse_stop(s: &str) -> Option<Token> {
        let re = Regex::new("^:stop (.*):$").unwrap();
        let caps = re.captures(&s)?;
        let tok = Token::Stop(caps.get(1).unwrap().as_str().trim().to_string());
        Some(tok)
    }

    fn parse_key_value(s: &str) -> Option<Token> {
        let re = Regex::new("^(.*)=(.*)$").unwrap();
        let caps = re.captures(&s)?;
        let cap_key = caps.get(1)?;
        let cap_val = caps.get(2)?;
        let tok = Token::KeyValue(
            cap_key.as_str().trim().to_string(),
            cap_val.as_str().trim().to_string(),
        );
        Some(tok)
    }

    pub fn read_next(reader: &mut BufRead) -> Option<Result<Token>> {
        match read_token_raw(reader) {
            Ok(s) => Some(Token::parse(&s)),
            Err(_) => None // this is fragile!,
        }
    }

    fn _to_string(self) -> String {
        let s = match self {
            Token::Start(s) => format!(":start {}:", s),
            Token::Stop(s) => format!(":stop {}:", s),
            Token::KeyValue(k, v) => format!("{} = {}", k, v),
        };
        s
    }

    fn soft_dec(x: usize) -> usize {
        if x > 0 {
            x - 1
        } else {
            x
        }
    }

    fn soft_inc(x: usize) -> usize {
        x + 1
    }

    pub fn to_string_indent(self: Token, indent: usize) -> (String, usize) {
        let (i_current, i_next) = match self {
            Token::Start(_) => (indent, Token::soft_inc(indent)),
            Token::Stop(_) => (Token::soft_dec(indent), Token::soft_dec(indent)),
            Token::KeyValue(_, _) => (indent, indent),
        };
        let ws = "    ".repeat(i_current);
        let s = format!("{}{}", ws, self._to_string());
        (s, i_next)
    }

    pub fn value(&self) -> Option<&String> {
        match *self {
            Token::KeyValue(_, ref val) => Some(val),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TokenStream {
    tokens: Vec<Token>,
}

impl TokenStream {
    pub fn to_string(&self) -> String {
        let mut lines = Vec::new();
        let mut indent = 0;
        for tok in self.tokens.clone() {
            let (line, i) = tok.to_string_indent(indent);
            indent = i;
            lines.push(line);
        }
        lines.join("\n")
    }

    pub fn parse_reader(reader: &mut BufRead) -> Result<TokenStream> {
        let mut tokens = Vec::new();
        loop {
            let ot = Token::read_next(reader);
            if ot.is_none() {
                break;
            } // end of file
            let t = ot.unwrap()?;
            tokens.push(t);
        }
        let stream = TokenStream { tokens };
        Ok(stream)
    }
    pub fn parse_string(s: &str) -> Result<TokenStream> {
        let mut reader = BufReader::new(s.as_bytes());
        let result = TokenStream::parse_reader(&mut reader);
        result
    }

    // pub fn get_index(&self, index:usize) -> Token {};

    fn find_index(&self, key: &str) -> Vec<usize> {
        let mut ret: Vec<usize> = Vec::new();
        for (i, token) in self.tokens.iter().enumerate() {
            match *token {
                Token::KeyValue(ref k, _) => {
                    if k == key {
                        ret.push(i);
                    }
                }
                _ => {}
            }
        }
        ret
    }

    fn find_index_single(&self, key: &str) -> Option<usize> {
        let xs = self.find_index(key);
        let x = single(&xs)?;
        Some(*x)
    }

    pub fn generate_seeds(&self, n: usize) -> Result<Vec<Seed>> {
        let mut seeds = Vec::new();
        for i in 1..(n + 1) {
            seeds.push((42, i));
        }
        assert_eq!(seeds.len(), n);
        Ok(seeds)
    }

    pub fn generate_ncases(&self, n: usize) -> Result<Vec<u64>> {
        let n64 = n as u64;
        let ncase = self.get_ncase()?;
        let ncase_new = ncase / n64;
        assert!(ncase_new > 0);
        let ncases: Vec<u64> = vec![ncase_new; n]; // TODO missing cases
        Ok(ncases)
    }

    fn get_index(&self, index: usize) -> &Token {
        let t = &self.tokens[index];
        t
    }

    pub fn get_ncase(&self) -> Result<u64> {
        let index_ncase = self.find_index_single("ncase").ok_or("Cannot find ncase")?;
        let sncase = self.get_index(index_ncase)
            .value()
            .ok_or("Cannor parse ncase")?;
        let ncase: u64 =
            str::parse(&sncase).chain_err(|| format!("Cannot parse ncase from {:?}", sncase))?;
        Ok(ncase)
    }

    pub fn split(&self, seeds: &Vec<Seed>, ncases: &Vec<u64>) -> Result<Vec<TokenStream>> {
        let ret = ncases
            .iter()
            .zip(seeds)
            .map(|(ncase, seed)| self.with_seed_and_ncase(seed, *ncase).unwrap())
            .collect();
        Ok(ret)
    }

    fn with_seed_and_ncase(&self, seed: &Seed, ncase_new: u64) -> Result<TokenStream> {
        let mut ret = self.clone();
        let index_ncase = self.find_index_single("ncase").ok_or("Cannot find ncase")?;
        let index_seed = self.find_index_single("initial seeds")
            .ok_or("Cannot find initial seeds")?;
        ret.tokens[index_ncase] = Token::KeyValue("ncase".to_string(), format!("{}", ncase_new));
        let &(s1, s2) = seed;
        ret.tokens[index_seed] =
            Token::KeyValue("initial seeds".to_string(), format!("{} {}", s1, s2));
        Ok(ret)
    }
}

fn single<T>(v: &[T]) -> Option<&T> {
    if v.len() == 1 {
        let x = v.first();
        x
    } else {
        None
    }
}

fn read_clean_line(reader: &mut BufRead) -> Result<String> {
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap() == 0 {
        bail!("End of file");
    }
    let line = line.split('#').next().unwrap();
    let line = line.trim();
    if line == "" {
        read_clean_line(reader)
    } else {
        Ok(line.to_string())
    }
}

fn read_token_raw(reader: &mut BufRead) -> Result<String> {
    let mut line = read_clean_line(reader)?;
    if let Some('\\') = line.chars().last() {
        line = format!(
            "{}{}",
            {
                line.pop();
                line
            },
            read_token_raw(reader).unwrap()
        );
    }
    Ok(line)
}

#[test]
fn test_parse_single_token() {
    let s_start = ":start rng definition:";
    let t_start = Ok(Token::Start("rng definition".to_string()));
    assert_eq!(Token::parse(s_start).into_stub(), t_start);

    let s_stop = ":stop rng definition:";
    let t_stop = Ok(Token::Stop("rng definition".to_string()));
    assert_eq!(Token::parse(s_stop).into_stub(), t_stop);

    let s1 = "initial seeds  =  20 1";
    let t1 = Ok(Token::KeyValue(
        "initial seeds".to_string(),
        "20 1".to_string(),
    ));
    assert_eq!(Token::parse(s1).into_stub(), t1);

    let s2 = "type  =  ranmar";
    let t2 = Ok(Token::KeyValue("type".to_string(), "ranmar".to_string()));
    assert_eq!(Token::parse(s2).into_stub(), t2);

    let s_garbage = "garbage";
    let t_garbage = Err("Cannot parse garbage".to_string());
    assert_eq!(Token::parse(s_garbage).into_stub(), t_garbage);
}

#[test]
fn test_parse_tokenstream() {
    let s = "
:start source definition:
    :start source:
        library = egs_collimated_source
        name = the_source
        :start source shape:
            type = point
            position = 0 0 -110
        :stop source shape:
        :start target shape:
            library   = egs_rectangle # some comment 
            rectangle = -2 -2 \
 2 2
        :stop target shape:
        distance = 110

        # co
        # mmen 

        charge = 0
        :start spectrum:
             type = monoenergetic
	energy = 13.75
        :stop spectrum:
    :stop source:

    simulation source = the_source

:stop source definition:
    ";
    let result = TokenStream::parse_string(&s);

    fn start(s: &str) -> Token {
        Token::Start(s.to_string())
    }
    fn stop(s: &str) -> Token {
        Token::Stop(s.to_string())
    }
    fn key_value(k: &str, v: &str) -> Token {
        Token::KeyValue(k.to_string(), v.to_string())
    }
    let tokens = [
        start("source definition"),
        start("source"),
        key_value("library", "egs_collimated_source"),
        key_value("name", "the_source"),
        start("source shape"),
        key_value("type", "point"),
        key_value("position", "0 0 -110"),
        stop("source shape"),
        start("target shape"),
        key_value("library", "egs_rectangle"),
        key_value("rectangle", "-2 -2 2 2"),
        stop("target shape"),
        key_value("distance", "110"),
        key_value("charge", "0"),
        start("spectrum"),
        key_value("type", "monoenergetic"),
        key_value("energy", "13.75"),
        stop("spectrum"),
        stop("source"),
        key_value("simulation source", "the_source"),
        stop("source definition"),
    ].to_vec();
    let stream = TokenStream { tokens };
    assert_eq!(result.unwrap(), stream);
    assert_eq!(
        TokenStream::parse_string(&stream.to_string()).unwrap(),
        stream
    );
}
