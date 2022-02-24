// extern crate pingmu;
// extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate prettytable;
// use chrono::{DateTime, Utc};
// use crate::chrono::prelude::{};
// use fastping_rs::PingResult::{Idle, Receive};
use pingmu::{Pinger, PingRecord, Delay, PingResult};
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::{Duration};
use chrono::{DateTime, Utc};
use std::sync::mpsc::channel;
use pingmu::save;
use pingmu::PingResult::{Receive, Idle};
use std::num::ParseIntError;
use std::convert::TryInto;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;



fn main() {
    let (ping_times, timeout,interval, filename, ips_vec) = detect_cli_input();
    // let m = PingRecord {
    //     ipaddress: "192.168.1.1".to_string(),
    //     delay: vec![Delay::Idle, Delay::DelayTime(Duration::from_millis(100)), Delay::DelayTime(Duration::from_millis(200)),]
    // };
    // println!("{}", m);
    // pretty_env_logger::init();
    let (pinger, results) = match Pinger::new(Some(timeout as u64), Some(64)) {
        Ok((pinger, results)) => (pinger, results),
        Err(e) => panic!("Error creating pinger: {}", e),
    };

    for x in ips_vec.iter() {
        pinger.add_ipaddr(x);
    }

    // pinger.add_ipaddr("127.0.0.1");
    // pinger.add_ipaddr("8.8.8.8");
    // pinger.add_ipaddr("1.1.1.1");
    // pinger.add_ipaddr("7.7.7.7");
    // pinger.add_ipaddr("2001:4860:4860::8888");
    println!("add ips completed!");
    // let ping_times: u32 = 4;
    let save_csv_path = "/Users/fly/workspace/Rust/pingmu/fff.csv";
    pinger.run_pinger(ping_times, interval);

    // receive result segment
    let target_ips = pinger.get_target_count() as u64;
    let mut count = target_ips * ping_times as u64;
    println!("ip address numbers: {}", count);
    let mut epoch_reset_count: u64 = 0;
    let (tx, rx) = channel();
    ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))
        .expect("Error setting Ctrl-C handler");
    println!("Waiting for Ctrl-C...");

    let mut ping_record_result: Vec<PingResult> = vec![];
    loop {
        match results.recv() {
            Ok(result) => {
                match result {
                    Idle { addr } => {
                        // println!("Idle Address {}.", addr);
                    }
                    Receive { addr, rtt , recv_duration} => {
                        let now: DateTime<Utc> = Utc::now();
                        println!("[{}]Receive from Address {} in {:?}.",now.format("%H:%M:%S"), addr, recv_duration);
                    }
                }
                ping_record_result.push(result);
            },
            Err(_) => panic!("Worker threads disconnected before the solution was found!"),
        }


        epoch_reset_count += 1;
        if epoch_reset_count == target_ips {
            epoch_reset_count = 0;
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(a) => {
                    println!("Exit by Ctrl+C");
                    // todo save ping result
                    save::save_result(ping_record_result, filename);
                    break;
                }
                Err(e) => {
                    println!("info: {}", e);
                }
            }
        }
        if ping_times == 0 {
            continue;
        } else if count == 1 { // 因为是先判断，后-
            println!("stop");
            // todo save
            save::save_result(ping_record_result, filename);
            break;
        } else {
            count -= 1;
        }
    }
}

fn detect_cli_input() -> (u32, u32, u64, Option<String>, Vec<String>) {
    use prettytable::{Table, Row, Cell};
    // Add a row per time

    let mut help_table = Table::new();
    help_table.add_row(row![" ", " ",  "ping number of times", "ping timeout(ms)", " send interval(us)", "input.text","filename", "cidr or range or ip", "..."]);
    // A more complicated way to add a row:
    help_table.add_row(Row::new(vec![
        Cell::new("sudo"),
        Cell::new("pingmu"),
        Cell::new("4"),
        Cell::new("2000"),
        Cell::new("100"),
        Cell::new("input.text"),
        Cell::new("out.csv"),
        Cell::new("192.168.1.1/30"),
        Cell::new("192.168.2.1-192.168.3.255")]));
    // Print the table to stdout
    // println!("{}", table);

    let args: Vec<String> = std::env::args().collect();
    println!("{:?}", args);
    if args.len() <= 1 || args[1].as_str() == "-h" {
        println!("do not > 4w ips");
        help_table.printstd();
        println!("example:\nsudo pingmu 10 2000 100 input.text out.csv 192.168.1.1/30 10.0.0.1-10.0.0.5 127.0.0.1");
        let mut help_table = Table::new();
        // help_table.add_row(row!["ip", "loss(%)", "min(ms)", "avg(ms)", "max(ms)", "stddev(ms)"]);
        println!("\nout.csv: value example");
        help_table.add_row(Row::new(vec![
            Cell::new("ip"),
            Cell::new("loss(%)"),
            Cell::new("min(ms)"),
            Cell::new("avg(ms)"),
            Cell::new("max(ms)"),
            Cell::new("stddev"),
            Cell::new("...(ms)")]));
        help_table.add_row(Row::new(vec![
            Cell::new("192.168.10.x"),
            Cell::new("0%"),
            Cell::new("5.93"),
            Cell::new("15.37"),
            Cell::new("20.00"),
            Cell::new("32"),
            Cell::new("10")]));
        help_table.printstd();
        std::process::exit(1);
    }
    let times =  args[1].parse::<u32>().unwrap_or_else(move|e| {
        panic!("{}", e)
    });
    let mut filename: Option<String> = None;
    let mut ips_vec: Vec<String> = vec![];

    let mut sub_v_flag: usize = 2;
    // let mut timeout: u32;
    let timeout =  match args[sub_v_flag].parse::<u32>() {
        Ok(a) => {
            sub_v_flag += 1;
            a
        },
        _ => 2000 // default timeout = 2000ms
    };
    let interval =  match args[sub_v_flag].parse::<u32>() {
        Ok(a) => {
            sub_v_flag += 1;
            a as u64
        },
        _ => 100 // default timeout = 2000ms
    };


    if (&args[sub_v_flag]).contains(".text") || (&args[sub_v_flag]).starts_with("input") {
        println!("detect input file");
        if let Ok(lines) = read_lines(args[sub_v_flag].to_string()) {
            // 使用迭代器，返回一个（可选）字符串
            for line in lines {
                if let Ok(ip) = line {
                    let ip = ip.trim().to_string();
                    let ip = Ipv4Addr::from_str(&ip).unwrap_or_else(move |e| {
                        panic!("convert ip error: {}", e)
                    });
                    ips_vec.push(ip.to_string());
                }
            }
        }

        sub_v_flag += 1;
    }

    if (&args[sub_v_flag]).ends_with(".csv") {
        // check_file(args[sub_v_flag])
        let x = check_file(&args[sub_v_flag].to_string());
        println!("x{}", x);
        filename = Some(x);
        sub_v_flag += 1;
    } else if  (&args[sub_v_flag]).contains(".") {

    } else {
        help_table.printstd();
        println!("example:\nsudo ./pingmu 4 2000 1000 input.text out.csv 192.168.1.1/30 10.0.0.1-10.0.0.5 127.0.0.1");
        std::process::exit(1);
    }

    // else {
    //     // === before
    //     if (&args[2]).contains("csv") {
    //         filename = Some(args[2].to_string())
    //     } else if  (&args[2]).contains(".") {
    //
    //     } else {
    //         help_table.printstd();
    //         println!("example:\nsudo pingmu 10 out.csv 192.168.1.1/30 10.0.0.1-10.0.0.5 127.0.0.1");
    //         std::process::exit(1);
    //     }
    // }

    // let sub_v: usize;
    // if let Some(_) = filename {
    //     sub_v = 3;
    // } else {
    //     sub_v = 2;
    // }

    for i in sub_v_flag..args.len() {
        let ip_string = (&args[i]).trim();
        if ip_string.contains("-") {
            ips_vec.append(&mut ip_range_to_list(ip_string));
        } else if ip_string.contains("/") {
            let ips = ipnetwork::IpNetwork::from_str(ip_string).unwrap_or_else(move |e| {
                panic!("{}", e)
            });
            for x in ips.iter() {
                ips_vec.push(x.to_string());
            }
        } else if ip_string.contains(".") {
            let ip = Ipv4Addr::from_str(ip_string).unwrap_or_else(move |e| {
                panic!("{}", e)
            });
            ips_vec.push(ip.to_string());
        } else {
            panic!("error input.")
        }
    }

    println!("{:?}", ips_vec);
    return (times, timeout, interval, filename, ips_vec)
}

// fn parse_ipaddress(ipdes: &str) -> Vec<String> {
//
//     // let ip_string = "192.168.1.1-192.168.4.255";
//     let ip_string = "21.239.50.1-21.239.50.2";
//     // if ip_vec
//
//
//
//
// }



fn ip_range_to_list(ip_range: &str) -> Vec<String> {
    // let ip_range =
    let x: Vec<&str> = ip_range.split("-").collect();
    // println!("{:?}", x);
    let ip1 = x[0];
    let ip2 = x[1];
    let ip1_string = ip_str_to_hex(ip1);
    // println!("{}", ip1_string);
    let ip2_string = ip_str_to_hex(ip2);
    // println!("{}", ip2_string);
    let ip1_int = u64::from_str_radix( ip1_string.as_str(), 16).unwrap();
    let ip2_int = u64::from_str_radix(ip2_string.as_str(), 16).unwrap();
    // println!("{} {}", ip1_int, ip2_int);
    // let ip2_int = ip_str_to_hex(ip2).parse::<u64>().unwrap();
    let mut ip_vec: Vec<String> = vec![];
    for ip in ip1_int..=ip2_int {
        let ip_str = format!("{:0>8x}", ip);
        // println!("{}", ip_str);
        // println!("{}-{}-{}-{}", &ip_str[0..2], &ip_str[2..4], &ip_str[4..6], &ip_str[6..8]);
        let ip_str_arr = [&ip_str[0..2], &ip_str[2..4], &ip_str[4..6], &ip_str[6..8]];
        let ip_u8_arr = ip_str_arr.map(move |a| {
            u8::from_str_radix(a, 16).unwrap().to_string()
        });
        ip_vec.push(ip_u8_arr.join("."));
        // println!("{}", ip_u8_arr.join("."));
    }
    ip_vec
}



fn ip_str_to_hex(s: &str) -> String {
    let ip1_vec: Vec<&str> = s.split(".").collect();
    let ip1_arr: [&str;4] = ip1_vec.try_into().unwrap();
    let ip1_str_arr = ip1_arr.map(move|a| {
        format!("{:0>2x}", a.parse::<u8>().unwrap())
    });

    ip1_str_arr.join("")
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
    where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}


fn check_file(path: &str) -> String {
    // std::fs::metadata(path).is_ok();
    if Path::new(path.trim()).is_file() {
        let now: DateTime<Utc> = Utc::now();
        let s_vec: Vec<&str> = path.trim().split(".").collect();
        let mut a: String = (&s_vec[0..(s_vec.len()-1)]).join("");
        // let f_now = time::strftime("%Y%m%d_%H%M%S", &now).unwrap();
        // let b = &s_vec[s_vec.len()-1];
        let time_s = now.format("%Y%m%d_%H%M%S").to_string();
        a.push_str(&time_s);
        a.push_str(".csv");
        // if Path::new(&a).is_file() {
        //     a = "1".to_string() + &a;
        // }
        return a;
    } else {
        return String::from(path);

    }
    // let file =  std::fs::try_exists(path);
}