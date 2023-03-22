use crate::save::Delay::DelayTime;
use crate::PingResult;
use csv;
use csv::Writer;
use std::collections::HashMap;
use std::fs::File;
use std::time::Duration;
use std::{fmt, io};
// use std::path::Path;
#[allow(unused_imports)]
use log::{debug, error, info, log, warn};

pub struct PingRecord {
    pub ipaddress: String,
    pub delay: Vec<Delay>,
}

pub enum Delay {
    Idle,
    DelayTime(Duration),
}

impl fmt::Display for Delay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Delay::Idle => write!(f, "TMOut"),
            Delay::DelayTime(d) => write!(f, "{:.2}ms", d.as_micros() as f64 / 1000.0),
        }
    }
}

impl Delay {
    fn to_csv(&self) -> String {
        match self {
            Delay::Idle => "TMOut".to_string(), // format!("TMOut")
            Delay::DelayTime(d) => format!("{:.2}", d.as_micros() as f64 / 1000.0),
        }
    }
}

impl fmt::Display for PingRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\t  [", self.ipaddress)?;
        for x in self.delay.iter() {
            write!(f, " {},", x)?;
        }
        writeln!(f, "]",)?;
        Ok(())
    }
}

pub fn save_result(
    ping_result_vec: Vec<PingResult>,
    filename: Option<String>,
    is_log: bool,
    ips_vec: &[String],
) -> Result<(), io::Error> {
    let mut map: HashMap<String, Vec<Delay>> = HashMap::new();
    for x in ping_result_vec.iter() {
        match x {
            PingResult::Idle { addr } => match map.get_mut(&*addr.to_string()) {
                Some(ipv) => ipv.push(Delay::Idle),
                _ => {
                    let v1 = vec![Delay::Idle];
                    map.insert(addr.to_string(), v1);
                }
            },
            PingResult::Receive {
                addr,
                rtt: _rtt,
                recv_duration,
            } => {
                match map.get_mut(&*addr.to_string()) {
                    Some(ipv) => ipv.push(DelayTime(*recv_duration)),
                    _ => {
                        // let mut v1 = vec!;
                        map.insert(addr.to_string(), Vec::from([DelayTime(*recv_duration)]));
                    }
                }
            }
        }
    }
    let mut wtr: Writer<File>;
    let mut can_write_to_file = false;
    let mut total_ip_loss = 0u64;
    let mut total_icmp_loss = 0u64;
    let mut total_icmp_echo = 0u64;
    let mut global_max_ms: f32 = 0.0;
    let mut global_avg_ms: f64 = 0.0;
    if let Some(path) = filename {
        wtr = csv::Writer::from_path(path.as_str()).unwrap();
        can_write_to_file = true
    } else {
        wtr = csv::Writer::from_path("/var/tmp/tmkggjfuftrdtfy547688.csv").unwrap();
    }
    // let map = map.sort_by_key(|a| a.0);
    for k in ips_vec.iter() {
        let v = map.get(k).unwrap();
        let mut line: Vec<String> = vec![];
        // de log
        if is_log {
            print!("{:width$}[", k, width = 26);
        }
        line.push(k.to_string());
        let mut not_idle_array: Vec<f32> = vec![];
        let mut min: f32 = 100000000.0;
        let mut max: f32 = 0.0;
        let mut sum: f64 = 0.0;
        for x in v.iter() {
            match x {
                DelayTime(d) => {
                    let time_ms = d.as_micros() as f32 / 1000.0;
                    if time_ms > max {
                        max = time_ms
                    }
                    if time_ms < min {
                        min = time_ms
                    }
                    sum += time_ms as f64;
                    not_idle_array.push(time_ms);
                    total_icmp_echo += 1;
                }
                _ => {
                    total_icmp_loss += 1;
                }
            }
        }
        if not_idle_array.is_empty() {
            total_ip_loss += 1;
            line.push("100%".to_string()); // loss
            line.push("NaN".to_string()); // min
            line.push("NaN".to_string()); // avg
            line.push("NaN".to_string()); // max
            line.push("NaN".to_string()); // stddev
        } else {
            line.push(format!(
                "{:.2}%",
                (1.0 - not_idle_array.len() as f32 / v.len() as f32) * 100.0
            ));
            line.push(format!("{:.2}", min));
            let avg = sum / (not_idle_array.len() as f64);
            line.push(format!("{:.2}", avg));
            if global_max_ms < max {
                global_max_ms = max;
            }
            global_avg_ms += avg;
            line.push(format!("{:.2}", max));
            let variance = not_idle_array
                .iter()
                .map(|value| {
                    let diff = avg - (*value as f64);
                    diff * diff
                })
                .sum::<f64>()
                / not_idle_array.len() as f64;
            line.push(format!("{:.2}", variance.sqrt()));
        }
        for x in v.iter() {
            // de log
            if is_log {
                print!(" {},", x);
            }
            line.push(x.to_csv());
        }
        // de log
        if is_log {
            println!("]");
        }
        // let mut file = File::create("text.csv").unwrap();
        // let mut wtr = csv::Writer::from_writer(io::stdout());
        if can_write_to_file {
            wtr.write_record(&line)?;
        }
    }
    if can_write_to_file {
        wtr.flush()?;
    }
    // else {
    // for (k, v) in map.iter() {
    //     print!("{:width$}[", k, width=26);
    //     for x in v.iter() {
    //         print!(" {},", x);
    //     }
    //     print!("]\n");
    // }

    use prettytable::{Cell, Row, Table};
    // Add a row per time

    let mut help_table = Table::new();
    // help_table.add_row(row!["ip", "loss(%)", "min(ms)", "avg(ms)", "max(ms)", "stddev(ms)"]);

    help_table.add_row(Row::new(vec![
        Cell::new("loss ip/total ip"),
        Cell::new("loss_packets/all_ping_packets"),
        Cell::new("total_loss(%)"),
        Cell::new("max delay(ex TMOut)"),
        Cell::new("avg delay(ex TMOut)"),
    ]));
    help_table.add_row(Row::new(vec![
        Cell::new(format!("{}/{}", total_ip_loss, map.len()).as_str()),
        Cell::new(format!("{}/{}", total_icmp_loss, total_icmp_echo + total_icmp_loss).as_str()),
        Cell::new(
            format!(
                "{:.4}%",
                total_icmp_loss as f64 / (total_icmp_echo + total_icmp_loss) as f64 * 100.0
            )
            .as_str(),
        ),
        Cell::new(format!("{:.2}ms", global_max_ms).as_str()),
        Cell::new(format!("{:.2}ms", global_avg_ms / (map.len() as f64)).as_str()),
    ]));
    help_table.printstd();
    Ok(())
}

#[cfg(test)]
mod tests {

    // #[test]
    // fn test_ping() {
    //     let m = PingRecord {
    //         ipaddress: "192.168.1.1".to_string(),
    //         delay: vec![Delay::Idle, Delay::DelayTime(Duration::from_millis(100)), Delay::DelayTime(Duration::from_millis(200)),]
    //     };
    //     println!("{}", m);
    //     assert_eq!(p.get_sequence_number(), 0);
    //     assert!(p.get_identifier() > 0);

    //     p.increment_sequence_number();
    //     assert_eq!(p.get_sequence_number(), 1);
    // }
    #[test]
    fn test_str() {
        let a111 = "fsfsf.csvsa.csv";
        if a111.ends_with(".csv") {
            let r1 = a111.rsplit_once(".csv").unwrap();
            println!("---{:}  {:}", r1.0, r1.1);
        }
        let a = a111.split(".csv").collect::<Vec<&str>>();
        println!("{:?}", a);
    }
}
