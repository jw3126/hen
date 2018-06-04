use std::io::BufRead;
use regex::Regex;
use uncertain::UncertainF64;
use simulation::SingSimParsedOutput;
use util::Result;
use std::path::Path;

fn parse_dot_separated_key_value(s: &str) -> Option<(String, String)> {
    let re = Regex::new(r"^(.*[^\.])\.\.\.*(.*)$").unwrap();
    let caps = re.captures(&s)?;
    let cap_key = caps.get(1)?;
    let cap_val = caps.get(2)?;
    let key = cap_key.as_str().to_string();
    let val = cap_val.as_str().to_string();
    return Some((key, val));
}

fn read_line_until(reader: &mut BufRead, re: &Regex) -> Option<String> {
    let line = read_line(reader)?;
    if re.is_match(&line) {
        return Some(line);
    } else {
        return read_line_until(reader, re);
    }
}

fn read_line(reader: &mut BufRead) -> Option<String> {
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap() == 0 {
        return None;
    } else {
        return Some(line);
    }
}

fn parse_total_cpu_time(line: &str) -> Result<f64> {
    let re = Regex::new(r"^Total cpu time for this run:\s*(.*) \(sec.\)").unwrap();
    let err = format!("Cannot parse total cpu time from {}", line).to_string();
    let s = re.captures(&line)
        .ok_or_else(|| err.clone())?
        .get(1)
        .ok_or_else(|| err.clone())?
        .as_str();
    let ret: f64 = s.parse::<f64>().map_err(|_| err.clone())?;
    return Ok(ret);
}

fn parse_geometry_dose(line: &str) -> Result<(String, UncertainF64)> {
    let re = Regex::new(r"^\s*(.*)\s\s*(.*) \+/\- (.*)%").unwrap();
    let caps = re.captures(&line)
        .ok_or(format!("Cannot match {:?} on {:?}.", re, line))?;
    let name = caps.get(1)
        .ok_or(format!("Cannot parse geometry from {:?}", line))?
        .as_str()
        .trim()
        .to_string();
    let svalue = caps.get(2)
        .ok_or(format!("Cannot parse dose value from {:?}", line))?
        .as_str();
    let value = svalue
        .trim()
        .parse::<f64>()
        .map_err(|err| format!("Cannot parse f64 from {:?} {:?}", svalue, err))?;
    let srstd = caps.get(3)
        .ok_or(format!("Cannot parse dose rstd from {:?}", line))?
        .as_str();
    let rstd_percent = srstd
        .trim()
        .parse::<f64>()
        .map_err(|err| format!("Cannot parse f64 from {:?} {:?}", srstd, err))?;
    let rstd = rstd_percent / 100.;
    let score = UncertainF64::from_value_rstd(value, rstd);
    return Ok((name, score));
}

pub fn parse_simulation_output(reader: &mut BufRead) -> Result<SingSimParsedOutput> {
    let re = Regex::new("^==(=*)").unwrap();
    read_line_until(reader, &re);
    read_line_until(reader, &re);
    let mut line = read_line(reader).ok_or("Unexpected end of file".to_string())?;
    while !(re.is_match(&line)) {
        let _kv = parse_dot_separated_key_value(line.trim()).unwrap();
        line = read_line(reader).ok_or("Unexpected end of file".to_string())?;
    }
    read_line_until(reader, &Regex::new("^Finished simulation").unwrap());

    let mut mline = read_line_until(reader, &Regex::new("^Total cpu time for this run").unwrap());
    let total_cpu_time = match mline {
        None => Err("Cannot find Total cpu time for this run".to_string()),
        Some(l) => parse_total_cpu_time(&l),
    };

    // // parse number of histories etc.
    // while !(line.trim() == "".to_string()) {
    //     line = read_line(reader).unwrap();
    //     println!("{}", line);
    // }

    let re_many_minus = Regex::new("^---*").unwrap();
    mline = read_line_until(reader, &re_many_minus);
    let dose = match mline {
        None => Err("Cannot find dose".to_string()),
        Some(_) => {
            let mut v = Vec::new();
            loop {
                mline = read_line(reader);
                if mline == None {
                    break Ok(v);
                }
                line = mline.unwrap();
                if line.trim().is_empty() {
                    break Ok(v);
                }
                let edose1 = parse_geometry_dose(&line);
                match edose1 {
                    Ok(dose1) => {
                        v.push(dose1);
                    }
                    Err(e) => {
                        break Err(e);
                    }
                }
            }
        }
    };

    // parse_geometry_dose
    mline = read_line_until(reader, &Regex::new("finishSimulation").unwrap());
    let simulation_finished = match mline {
        None => Err("Cannot find SingSimFinished".to_string()),
        Some(_) => {
            if read_line(reader) == None {
                Ok(true)
            } else {
                Ok(false)
            }
        }
    };
    let ret = SingSimParsedOutput {
        dose,
        total_cpu_time,
        simulation_finished,
    };
    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;
    use util::asset_path;
    use uncertain::UncertainF64;
    fn parse_simulation_output_from_file(path: &Path) -> SingSimParsedOutput {
        use std::fs::File;
        use std::io::BufReader;
        let f = File::open(path).unwrap();
        let mut r = BufReader::new(f);
        let out = parse_simulation_output(&mut r).unwrap();
        out
    }

    #[test]
    fn test_parse_dot_separated_key_value() {
        let s = "configuration...linux64";
        assert_eq!(
            parse_dot_separated_key_value(s),
            Some(("configuration".to_string(), "linux64".to_string()))
        );
    }

    #[test]
    fn test_parse_geometry_dose() {
        let line = "Block_                    0.0000e+00 +/- 100.000% \n";
        assert_eq!(
            parse_geometry_dose(&line),
            Ok(("Block_".to_string(), UncertainF64::from_value_rstd(0., 1.)))
        );

        let line = "Block_                    2.1867e-16 +/- 54.499 % \n";
        let score = UncertainF64::from_value_rstd(0.00000000000000021867, 0.54499);
        assert_eq!(
            parse_geometry_dose(&line),
            Ok(("Block_".to_string(), score))
        );
    }

    #[test]
    fn test_parse_loop_simulation_output() {
        let path = asset_path().join("Wasservoxel.log");
        let out = parse_simulation_output_from_file(&path);
        assert_eq!(out.total_cpu_time.unwrap(), 1997.04);
        assert_eq!(out.simulation_finished.unwrap(), true);
        let dose = out.dose.unwrap();
        let dose0 = (
            "PSS_Box".to_string(),
            UncertainF64::from_value_rstd(0.0, 1.0),
        );
        let dose1 = (
            "Messwelt_0".to_string(),
            UncertainF64::from_value_rstd(5.6425e-13, 0.955e-2),
        );
        let dose81 = (
            "Messwelt_4".to_string(),
            UncertainF64::from_value_rstd(2.1412e-12, 1.359e-2),
        );
        assert_eq!(dose[0], dose0);
        assert_eq!(dose[1], dose1);
        assert_eq!(dose[81], dose81);
        assert_eq!(dose.len(), 82);
    }

    #[test]
    fn test_parse_timeout_simulation_output() {
        let path1 = asset_path().join("timeout.log");
        let path2 = asset_path().join("statistical_accuracy_reached.log");
        for path in [path1, path2].iter() {
            let out = parse_simulation_output_from_file(&path);
            assert!(out.simulation_finished.unwrap());
            let dose = out.dose.unwrap();
            assert_eq!(dose.len(), 1);
            assert_eq!(
                dose[0],
                (
                    "geo".to_string(),
                    UncertainF64::from_value_rstd(5.3408e-10, 0.124e-2)
                )
            );
        }
    }
}
