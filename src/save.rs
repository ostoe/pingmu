use std::{fmt, io};
use std::time::Duration;
use crate::PingResult;
use std::collections::HashMap;
use crate::save::Delay::{Idle, DelayTime};
use std::fs::File;
use csv;
use std::io::Error;
use csv::Writer;
// use std::path::Path;

pub struct PingRecord {
    pub ipaddress: String,
    pub delay: Vec<Delay>
}

pub enum Delay {
    Idle,
    DelayTime(Duration)
}

impl fmt::Display for Delay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Delay::Idle => write!(f, "idle"),
            Delay::DelayTime(d) => write!(f, "{:.2}ms", d.as_micros() as f64 / 1000.0)
        };
        Ok(())
    }
}

impl Delay {
    fn to_csv(&self) -> String {
        match self {
            Delay::Idle => format!("idle").to_string(),
            Delay::DelayTime(d) => format!("{:.2}", d.as_micros() as f64 / 1000.0).to_string(),
        }
    }
}

impl fmt::Display for PingRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\t  [", self.ipaddress);
        for x in self.delay.iter() {
            write!(f, " {},", x);
        }
        write!(f, "]\n", );
        Ok(())
    }
}

pub fn save_result(v: Vec<PingResult>, filename: Option<String>) -> Result<(), io::Error > {
    let mut map : HashMap<String, Vec<Delay>> = HashMap::new();
    for x in v.iter() {
        match x {
            PingResult::Idle{ addr} => {
                match map.get_mut(&*addr.to_string()) {
                    Some(ipv) => ipv.push(Delay::Idle),
                    _ => {
                        let mut v1 = vec![Delay::Idle];
                        map.insert(addr.to_string(), v1);
                    }
                }
            },
            PingResult::Receive{ addr, rtt, recv_duration} => {
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
    let mut total_ip_loss = 0u64;
    let mut total_icmp_loss = 0u64;
    let mut total_icmp_echo = 0u64;
    let mut global_max_ms: f32 = 0.0;
    let mut global_avs_ms: f32 = 0.0;
    if let Some(path) = filename {
        wtr = csv::Writer::from_path(path.as_str()).unwrap();

        for (k, v) in map.iter() {
            let mut line:Vec<String> = vec![];
            print!("{:width$}[", k, width=26);
            line.push(k.to_string());
            let mut not_idle_array: Vec<f32> = vec![];
            let mut min: f32 = 100000000.0;
            let mut max: f32 = 0.0;
            let mut sum: f64 = 0.0;
            for x in v.iter() {
                match x {
                    DelayTime(d) => {
                        let time_ms = d.as_micros() as f32 / 1000.0;
                        if time_ms > max { max = time_ms }
                        if time_ms < min { min = time_ms }
                        sum += time_ms as f64;
                        not_idle_array.push(time_ms);
                        total_icmp_echo += 1;
                    },
                    _ => { total_icmp_loss += 1; },
                }
            }
            if not_idle_array.len() == 0 {
                total_ip_loss += 1;
                line.push("100%".to_string()); // loss
                line.push("NaN".to_string());  // min
                line.push("NaN".to_string());  // avg
                line.push("NaN".to_string());  // max
                line.push("NaN".to_string());  // stddev
            } else {
                line.push(format!("{:.2}%", (1.0 - not_idle_array.len() as f32 / v.len() as f32) * 100.0));
                line.push(format!("{:.2}", min));
                let avg = sum / (not_idle_array.len() as f64);
                line.push(format!("{:.2}", avg));
                if max > global_max_ms as f32 {
                    global_max_ms = max;
                }
                global_avs_ms += avg;
                line.push(format!("{:.2}", max));
                let variance = not_idle_array.iter().map(|value| {
                    let diff = avg - (*value as f64);
                    diff * diff
                }).sum::<f64>() / not_idle_array.len() as f64;
                line.push(format!("{:.2}", variance.sqrt()));
            }
            for x in v.iter() {
                print!(" {},", x);
                line.push(x.to_csv());
            }
            print!("]\n");
            // let mut file = File::create("text.csv").unwrap();
            // let mut wtr = csv::Writer::from_writer(io::stdout());
            wtr.write_record(&line)?;
        }
        wtr.flush()?;
    } else {
        for (k, v) in map.iter() {
            print!("{:width$}[", k, width=26);
            for x in v.iter() {
                print!(" {},", x);
            }
            print!("]\n");
        }
    }
    use prettytable::{Table, Row, Cell};
    // Add a row per time


    let mut help_table = Table::new();
    // help_table.add_row(row!["ip", "loss(%)", "min(ms)", "avg(ms)", "max(ms)", "stddev(ms)"]);

    help_table.add_row(Row::new(vec![
        Cell::new("loss ip/total ip"),
        Cell::new("ping_loss_packets/all_ping_packets"),
        Cell::new("total_loss(%)"),
        Cell::new("avg delay(exclude timeout packets)"),
        Cell::new("max delay(exclude timeout packets)"),
        ]));
    help_table.add_row(Row::new(vec![
        Cell::new(format!("{}/{}", total_ip_loss, map.len()).as_str()),
        Cell::new(format!("{}/{}", total_icmp_loss, total_icmp_echo + total_icmp_loss).as_str()),
        Cell::new(format!("{:.4}%", total_icmp_loss as f64 /
            (total_icmp_echo + total_icmp_loss) as f64 * 100.0).as_str()),
        Cell::new(format!("{:.2}ms", global_max_ms).as_str()),
        Cell::new(format!("{:.2}ms", global_avg_ms / map.len()).as_str()),
        ]));
    help_table.printstd();
    Ok(())

}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping() {
        let m = PingRecord {
            ipaddress: "192.168.1.1".to_string(),
            delay: vec![Delay::Idle, Delay::DelayTime(Duration::from_millis(100)), Delay::DelayTime(Duration::from_millis(200)),]
        };
        println!("{}", m);
        assert_eq!(p.get_sequence_number(), 0);
        assert!(p.get_identifier() > 0);

        p.increment_sequence_number();
        assert_eq!(p.get_sequence_number(), 1);
    }
}