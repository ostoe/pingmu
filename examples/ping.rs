extern crate fastping_rs;
// extern crate pretty_env_logger;
#[macro_use]
extern crate log;

// use chrono::{DateTime, Utc};
// use crate::chrono::prelude::{};
use fastping_rs::PingResult::{Idle, Receive};
use fastping_rs::Pinger;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::{Duration};
use chrono::{DateTime, Utc};
use std::sync::mpsc::channel;

fn main() {
    // pretty_env_logger::init();
    let (pinger, results) = match Pinger::new(Some(2000 as u64), Some(64)) {
        Ok((pinger, results)) => (pinger, results),
        Err(e) => panic!("Error creating pinger: {}", e),
    };

    let ip_string = "192.168.1.1-192.168.4.255";
    let ip_string = "21.239.50.1-21.239.50.2";
    let ip_vec: Vec<&str> = ip_string.split("-").collect();
    if ip_vec.len() != 2 {
        panic!("wrong input")
    }
    // if ip_vec
    let ip1 = Ipv4Addr::from_str(ip_vec[0]).unwrap().octets();
    let ip2 = Ipv4Addr::from_str(ip_vec[1]).unwrap().octets();
    //
    if ip1[0] != ip2[0] || ip1[1] != ip2[1] {
        panic!("wrong input")
    } else if ip1[2] != ip2[2] {
        for j in ip1[2]..=ip2[2] {
            for i in ip1[3]..=ip2[3] {
                let ip_ji = Ipv4Addr::new(ip1[0], ip1[1], j, i);
                pinger.add_ipaddr(&ip_ji.to_string())
            }
        }
    } else if ip1[2] == ip2[2] {
        for i in ip1[3]..=ip2[3] {
            let ip_ji = Ipv4Addr::new(ip1[0], ip1[1], ip2[2], i);
            pinger.add_ipaddr(&ip_ji.to_string())
        }
    }
    pinger.add_ipaddr("127.0.0.1");

    pinger.add_ipaddr("8.8.8.8");
    pinger.add_ipaddr("1.1.1.1");
    pinger.add_ipaddr("7.7.7.7");
    pinger.add_ipaddr("2001:4860:4860::8888");
    println!("add ips completed!");
    let ping_times: u32 = 1;
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

    loop {
        match results.recv() {
            Ok(result) => match result {
                Idle { addr } => {
                    println!("Idle Address {}.", addr);
                }
                Receive { addr, rtt , recv_duration} => {
                    let now: DateTime<Utc> = Utc::now();
                    println!("[{}]Receive from Address {} in {:?}.",now.format("%H:%M:%S"), addr, recv_duration);
                }
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
                    break;
                }
                Err(e) => {
                    println!("error: {}", e);
                }
            }
        }
        if ping_times == 0 {
            continue;
        } else if count == 1 { // 因为是先判断，后-
            println!("stop");
            // todo save
            break;
        } else {
            count -= 1;
        }
    }
}
