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


fn main() {
    let (ping_times, timeout, filename, ips_vec) = detect_cli_input();
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
    pinger.run_pinger(ping_times);

    // receive result segment
    let target_ips = pinger.get_target_count() as u64;
    let mut count = target_ips * ping_times as u64;
    println!("{}", count);
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

fn detect_cli_input() -> (u32, u32, Option<String>, Vec<String>) {
    use prettytable::{Table, Row, Cell};
    // Add a row per time

    let mut help_table = Table::new();
    help_table.add_row(row![" ", " ",  "ping number of times", "ping timeout(ms)", "filename", "cidr or range or ip", "..."]);
    // A more complicated way to add a row:
    help_table.add_row(Row::new(vec![
        Cell::new("sudo"),
        Cell::new("pingmu"),
        Cell::new("4"),
        Cell::new("2000"),
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
        println!("example:\nsudo pingmu 10 2000 out.csv 192.168.1.1/30 10.0.0.1-10.0.0.5 127.0.0.1");
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

    if (&args[sub_v_flag]).ends_with(".csv") {
        filename = Some(args[sub_v_flag].to_string());
        sub_v_flag += 1;
    } else if  (&args[sub_v_flag]).contains(".") {

    } else {
        help_table.printstd();
        println!("example:\nsudo pingmu 4 2000 out.csv 192.168.1.1/30 10.0.0.1-10.0.0.5 127.0.0.1");
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
            let ip_vec: Vec<&str> = ip_string.split("-").collect();
            if ip_vec.len() != 2 {
                panic!("wrong input")
            }
            let ip1 = Ipv4Addr::from_str(ip_vec[0]).unwrap_or_else(move |e| {
                panic!("{}", e)
            }).octets();
            let ip2 = Ipv4Addr::from_str(ip_vec[1]).unwrap_or_else(move |e| {
                panic!("{}", e)
            }).octets();
            //
            if ip1[0] != ip2[0] || ip1[1] != ip2[1] {
                panic!("wrong input")
            } else if ip1[2] != ip2[2] {
                for j in ip1[2]..=ip2[2] {
                    for i in ip1[3]..=ip2[3] {
                        let ip_ji = Ipv4Addr::new(ip1[0], ip1[1], j, i);
                        ips_vec.push(ip_ji.to_string());
                        // pinger.add_ipaddr(&ip_ji.to_string())
                    }
                }
            } else if ip1[2] == ip2[2] {
                for i in ip1[3]..=ip2[3] {
                    let ip_ji = Ipv4Addr::new(ip1[0], ip1[1], ip2[2], i);
                    ips_vec.push(ip_ji.to_string());
                    // pinger.add_ipaddr(&ip_ji.to_string())
                }
            }
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
    return (times, timeout, filename, ips_vec)
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
