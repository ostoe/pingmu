// extern crate pingmu;
// extern crate pretty_env_logger;

#[macro_use]
extern crate prettytable;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;
// use chrono::{DateTime, Utc};
// use crate::chrono::prelude::{};
// use fastping_rs::PingResult::{Idle, Receive};
use chrono::{DateTime, Utc};
use log::{debug, error, info, trace};
use log4rs;
use pingmu::save;
use pingmu::PingResult::{Idle, Receive};
use pingmu::{ PingResult, Pinger};
use std::collections::HashMap;
// use std::convert::TryInto;
use std::fs::File;
use std::io::{self, BufRead};
use std::net::Ipv4Addr;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc::channel;
use std::time::Duration;

use std::path::PathBuf;
use std::vec;
// use std::vec;
use clap::{CommandFactory};

use clap::{ Parser, Subcommand, ValueEnum};

const HELP_AND_LIMITED: &str = 
"eg. sudo ./pingmu  1.2.2.3/24 1.2.3.4-128 1.1.1.1-1.1.2.1
    sudo ./pingmu -c 1 1.2.3.4/30
    sudo ./pingmu -c 4 -input ips.txt -o output.csv 1.2.3.4/30\n
tips: 1. Need sudo.
      2. You can press ctrl+c anytime to stopped the program.
";

#[derive(Parser)]
#[command(author, version, about, 
    after_help = HELP_AND_LIMITED,
    color=clap::ColorChoice::Auto,
    long_about = "[pingmu] A tool can ping multi-ip addresses")]
struct Cli {
    /// The number of icmp echo packges `ping`;
    #[arg(short, long, default_value_t = 3,value_name = "count")]
    count: u32,

    /// The Timeout for each icmp echo packge (/ms)
    #[arg(short, long, default_value_t = 1000, value_name = "timeout")]
    timeout: u64,

    /// The interval between sending every two packets (/ns)
    #[arg(short, long, default_value_t = 100, value_name = "interval")]
    interval: u64,

    /// Which log level to run.
    #[arg(short, long, default_value_t = CliLevelFilter::Info ,value_name = "loglevel", value_enum)]
    loglevel: CliLevelFilter,

    /// ip list input file like this:
    /// cat ips.txt:
    /// 192.168.1.2 10.1.1.1/24 ... support cidr or range
    #[arg(long, value_name = "ip list file path")]
    input: Option<PathBuf>,

    /// output each ping result to csv file. eg. output-xxxx.csv
    #[arg(short, long, value_name = "output path")]
    outputpath: Option<PathBuf>,

    // #[command(subcommand)]
    // command: Option<Commands>,

    // /// What mode to run the program in
    // #[arg( value_enum)] // arg(short, long, 加上这两个就会变成Arguments
    // mode: Option<Mode>,

    // Range: Vec<String>,
    /// <cidr | range> (s)  ex.  10.0.0.0/24 1.1.1.1-2.5
    #[arg(value_name = "cidr|range")]
    cidr: Vec<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum CliLevelFilter {
    /// A level lower than all log levels.
    Off,
    /// Corresponds to the `Error` log level.
    Error,
    /// Corresponds to the `Warn` log level.
    Warn,
    /// Corresponds to the `Info` log level.
    Info,
    /// Corresponds to the `Debug` log level.
    Debug,
    /// Corresponds to the `Trace` log level.
    Trace,
}


// impl fmt::Display for CliLevelFilter {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             Delay::Idle => write!(f, "TMOut"),
//             Delay::DelayTime(d) => write!(f, "{:.2}ms", d.as_micros() as f64 / 1000.0)
//         };

//         Ok(())
//     }
// }

#[derive(Subcommand)]
enum Commands {
    /// does testing things
    Test {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    trace!(" -c {:?} -i {:?} -t {:?} -l {:?} -in {:?} -o {:?} -cidr {:?}", cli.count, cli.interval, cli.timeout,
     cli.loglevel, cli.input, cli.outputpath, cli.cidr);

    // You can see how many times a particular flag or argument occurred
    // Note, only flags can have multiple occurrences
    // match cli.debug {
    //     0 => println!("Debug mode is off"),
    //     1 => println!("Debug mode is kind of on"),
    //     2 => println!("Debug mode is on"),
    //     _ => println!("Don't be crazy"),
    // }

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    // match &cli.command {
    //     Some(Commands::Test { list }) => {
    //         if *list {
    //             println!("Printing testing lists...");
    //         } else {
    //             println!("Not printing testing lists...");
    //         }
    //     }
    //     None => {}
    // }

    let level = match &cli.loglevel {
            CliLevelFilter::Off => log::LevelFilter::Off,
            CliLevelFilter::Error => log::LevelFilter::Error,
            CliLevelFilter::Warn => log::LevelFilter::Warn,
            CliLevelFilter::Info => log::LevelFilter::Info,
            CliLevelFilter::Debug => log::LevelFilter::Debug,
            CliLevelFilter::Trace => log::LevelFilter::Trace,
    };

    let level_patton = HashMap::from([
        (log::LevelFilter::Debug, "[{d(%H:%M:%S)}][{l}] - {m}{n}"),
        (log::LevelFilter::Error, "[{d}]{l} - {m}{n}"),
        (log::LevelFilter::Info, "{m}{n}"),
        (log::LevelFilter::Trace, "[{d}]{l} - {m}{n}"),
        (log::LevelFilter::Warn, "[{d}]{l} - {m}{n}"),
        (log::LevelFilter::Off, ""),
    ]);

    let stdout: ConsoleAppender = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            level_patton.get(&level).unwrap(),
        )))
        .build();
    let log_config = log4rs::config::Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(level))
        .unwrap();
    log4rs::init_config(log_config).unwrap();

    let mut ips_vec = vec![];
        if let Some(input_path) = cli.input.as_deref() {
            debug!("Value for config: {}", input_path.display());
            if let Ok(lines) = read_lines(input_path) {
                // 使用迭代器，返回一个（可选）字符串
                for line in lines {
                    if let Ok(ip) = line {
                        ips_vec .append( &mut parse_str_cidr_or_range_to_ip_list(&ip) );
                        // let ip = Ipv4Addr::from_str(&ip)
                        //     .unwrap_or_else(move |e| panic!("convert ip error: {}", e));
                        // ips_vec.push(ip.to_string());
                    }
                }
            }
        }

    let filename = match cli.outputpath {
        Some(path) => {
            let path1 = path.into_os_string().into_string().unwrap();
            Some(check_file(&path1))
        },
        None => None,
    };

    let is_log = if CliLevelFilter::Off == cli.loglevel {false} else {true};


    for cidr in &cli.cidr {
        let ip_string = (cidr).trim();
        ips_vec .append( &mut parse_str_cidr_or_range_to_ip_list(ip_string) );
    }

    if ips_vec.len() == 0 {
        Cli::command().print_help().unwrap();
        // error!("no ip input");
        std::process::exit(1);
    } else if ips_vec.len() > 20 {
        debug!(
            "{:?}",
            [
                &ips_vec[..5],
                &[String::from("......")],
                &ips_vec[ips_vec.len() - 5..]
            ]
            .concat()
        );
        // println!("{:?}", &ips_vec[ips_vec.len()-5..])
    } else {
        trace!("{:?}", ips_vec);
    }
    

// }

// fn main1() {
//     let (ping_times, timeout, interval, filename, ips_vec, is_log) = detect_cli_input();
    // let m = PingRecord {
    //     ipaddress: "192.168.1.1".to_string(),
    //     delay: vec![Delay::Idle, Delay::DelayTime(Duration::from_millis(100)), Delay::DelayTime(Duration::from_millis(200)),]
    // };
    // pretty_env_logger::init();
    debug!("{} {} {} {:?} {:?} {} {}", cli.count, cli.timeout, cli.interval, filename, ips_vec, is_log, level);
    let (pinger, results) = match Pinger::new(Some(cli.timeout), Some(64)) {
        Ok((pinger, results)) => (pinger, results),
        Err(e) => panic!("Error creating pinger: {}", e),
    };

    for x in ips_vec.iter() {
        pinger.add_ipaddr(x);
    }

    // pinger.add_ipaddr("127.0.0.1");
    // pinger.add_ipaddr("7.7.7.7");
    // pinger.add_ipaddr("2001:4860:4860::8888");
    debug!("add ips num {} completed!", ips_vec.len());
    // let ping_times: u32 = 4;
    pinger.run_pinger(cli.count, cli.interval);

    // receive result segment
    let target_ips = pinger.get_target_count() as u64;
    let mut count = target_ips * cli.count as u64;
    debug!("ALL ip ping of times: {}", count);
    let mut epoch_reset_count: u64 = 0;
    let (ctrlc_tx, ctrlc_rx) = channel();
    ctrlc::set_handler(move || ctrlc_tx.send(()).expect("Could not send signal on channel."))
        .expect("Error setting Ctrl-C handler");
    println!("Press Ctrl-C to stop...");
    let mut ping_record_result: Vec<PingResult> = vec![];
    loop {
        match results.recv() {
            Ok(result) => {
                match result {
                    Idle { addr } => {
                        trace!("Timeout Address {}.", addr);
                    }
                    Receive {
                        addr,
                        rtt: _rtt,
                        recv_duration,
                    } => {
                        // let now: DateTime<Utc> = Utc::now();
                        // if is_log {
                            debug!(
                                "Receive from Address {} in {:?}.",
                                // now.format("%H:%M:%S"),
                                addr,
                                recv_duration
                            );
                        // }
                    }
                }
                ping_record_result.push(result);
            }
            Err(_) => panic!("Worker threads disconnected before the solution was found!"),
        }

        epoch_reset_count += 1;
        if epoch_reset_count == target_ips {
            epoch_reset_count = 0;
            match ctrlc_rx.recv_timeout(Duration::from_millis(10)) {
                Ok(_) => {
                    info!("Exit by Ctrl+C");
                    // todo save ping result
                    save::save_result(ping_record_result, filename, is_log, &ips_vec).unwrap();
                    break;
                }
                Err(e) => {
                    // info receive timeout interval log.
                    trace!("recvtimeouterror: {}", e)
                }
            }
        }
        if cli.count == 0 {
            continue; // loop ping no timeout
        } else if count == 1 {
            // 因为是先判断，后-
            debug!("Stopped");
            save::save_result(ping_record_result, filename, is_log, &ips_vec).unwrap();
            break;
        } else {
            count -= 1;
        }
    }
}

fn _detect_cli_input() -> (u32, u32, u64, Option<String>, Vec<String>, bool) {
    use prettytable::{Cell, Row, Table};
    // Add a row per time

    let mut help_table = Table::new();
    help_table.add_row(row![
        " ",
        " ",
        "per ip times",
        "TM(ms)",
        "send i(us)",
        "input.text",
        "x.csv",
        "is log(op)",
        "cidr|range|ip",
        "..."
    ]);
    // A more complicated way to add a row:
    help_table.add_row(Row::new(vec![
        Cell::new("sudo"),
        Cell::new("./pingmu"),
        Cell::new("4"),
        Cell::new("2000"),
        Cell::new("100"),
        Cell::new("input.text"),
        Cell::new("out.csv"),
        Cell::new("nolog"),
        Cell::new("192.168.1.1/30"),
        Cell::new("192.168.2.1-192.168.3.255"),
    ]));

    let args: Vec<String> = std::env::args().collect();
    println!("{:?}", args);
    if args.len() <= 1 || args[1].as_str() == "-h" {
        println!("do not > 4w ips"); //DOTO write to help.
        help_table.printstd();
        println!("example:\n {} \n {}",  "sudo ./pingmu 4 192.168.1.1/24 192.168.1.2/31",
        "sudo ./pingmu 4 2000 100 input.text out.csv nolog 192.168.1.1/30 10.0.0.1-10.0.0.5 127.0.0.1");
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
            Cell::new("...(ms)"),
        ]));
        help_table.add_row(Row::new(vec![
            Cell::new("192.168.10.x"),
            Cell::new("0%"),
            Cell::new("5.93"),
            Cell::new("15.37"),
            Cell::new("20.00"),
            Cell::new("32"),
            Cell::new("10"),
        ]));
        help_table.printstd();
        std::process::exit(1);
    }
    let times = args[1]
        .parse::<u32>()
        .unwrap_or_else(move |e| panic!("{}", e));
    let mut filename: Option<String> = None;
    let mut ips_vec: Vec<String> = vec![];

    let mut sub_v_flag: usize = 2;
    // let mut timeout: u32;
    let timeout = match args[sub_v_flag].parse::<u32>() {
        Ok(a) => {
            sub_v_flag += 1;
            a
        }
        _ => 2000, // default timeout = 2000ms
    };
    let interval = match args[sub_v_flag].parse::<u32>() {
        Ok(a) => {
            sub_v_flag += 1;
            a as u64
        }
        _ => 100, // default interval 100us
    };

    // check text
    if (&args[sub_v_flag]).contains(".text") || (&args[sub_v_flag]).starts_with("input") {
        // println!("detect input file");
        if let Ok(lines) = read_lines(args[sub_v_flag].to_string()) {
            // 使用迭代器，返回一个（可选）字符串
            for line in lines {
                if let Ok(ip) = line {
                    let ip = ip.trim().to_string();
                    let ip = Ipv4Addr::from_str(&ip)
                        .unwrap_or_else(move |e| panic!("convert ip error: {}", e));
                    ips_vec.push(ip.to_string());
                }
            }
        }

        sub_v_flag += 1;
    }
    // check .csv
    if (&args[sub_v_flag]).ends_with(".csv") {
        // check_file(args[sub_v_flag])
        let x = check_file(&args[sub_v_flag].to_string());
        println!("will out to {}", x);
        filename = Some(x);
        sub_v_flag += 1;
    }

    // check is DE log; default log print.
    let mut is_log = true;
    if sub_v_flag < (&args).len() && (&args[sub_v_flag]).contains("nolog") {
        is_log = false;
        sub_v_flag += 1;
    }

    for i in sub_v_flag..args.len() {
        let ip_string = (&args[i]).trim();
        // println!("{}", ip_string);
        if ip_string.contains("-") {
            ips_vec.append(&mut ip_range_to_list(ip_string));
        } else if ip_string.contains("/") {
            let ips =
                ipnetwork::IpNetwork::from_str(ip_string).unwrap_or_else(move |e| panic!("{}", e));
            for x in ips.iter() {
                ips_vec.push(x.to_string());
            }
        } else if ip_string.contains(".") {
            let ip = Ipv4Addr::from_str(ip_string).unwrap_or_else(move |e| panic!("{}", e));
            ips_vec.push(ip.to_string());
        } else {
            panic!("error input.")
        }
    }

    if ips_vec.len() == 0 {
        error!("no ip input");
        std::process::exit(0);
    } else if ips_vec.len() > 20 {
        debug!(
            "{:?}",
            [
                &ips_vec[..5],
                &[String::from("......")],
                &ips_vec[ips_vec.len() - 5..]
            ]
            .concat()
        );
        // println!("{:?}", &ips_vec[ips_vec.len()-5..])
    } else {
        trace!("{:?}", ips_vec);
    }
    return (times, timeout, interval, filename, ips_vec, is_log);
}

/// 解析cidr or ip范围为地址列表，cidr自动跳过网络位和广播位
/// eg： ("192.168.1.0/30") -> ["192.168.1.1", "192.168.1.2"]
/// eg： ("192.168.1.1-2") -> ["192.168.1.1", "192.168.1.2"]
/// eg： ("192.168.1.1-192.168.1.2") -> ["192.168.1.1", "192.168.1.2"]
fn parse_str_cidr_or_range_to_ip_list(ip_input: &str) -> Vec<String> {
    let mut ips_vec = vec![];
    let ip_string = (ip_input).trim();
        // println!("{}", ip_string);
        if ip_string.contains("-") {
            ips_vec.append(&mut ip_range_to_list(ip_string));
        } else if ip_string.contains("/") {
            let ips =
                ipnetwork::IpNetwork::from_str(ip_string).unwrap_or_else(move |e| panic!("{}", e));
            let mut ips_iter = ips.iter();
            ips_iter.next(); // pop network addr. 去除网络地址
            
            debug!("{}--{} {:?} {:?}", ips.prefix(), ips.network(), ips.broadcast(), ips.size());
            for x in ips_iter {
                // if !x.to_string().ends_with(".0") {
                    ips_vec.push(x.to_string());
                // }
            }
            match ips.size() {
                ipnetwork::NetworkSize::V4(num) => 
                    if num > 1 {ips_vec.pop();}, // pop broadcast
                ipnetwork::NetworkSize::V6(_) => {},
            }
        } else if ip_string.contains(".") {
            let ip = Ipv4Addr::from_str(ip_string).unwrap_or_else(move |e| panic!("{}", e));
            ips_vec.push(ip.to_string());
        } else {
            info!("error input, skiped: {}", ip_input);
        }
        ips_vec
}


/// parse ipaddr range to vec<String>
fn ip_range_to_list(ip_range: &str) -> Vec<String> {
    let (ip_from, ip_to) = ip_range.split_once("-").expect("input error");
    let ip_from_arr: [&str; 4] = ip_from.splitn(4, ".").collect::<Vec<&str>>().try_into().expect("input ip error");
    if ip_from_arr.len() != 4 {panic!("error input")}
    let ip_to_vec: Vec<&str> = ip_to.splitn(4, ".").collect();
    let mut ip_to_arr: [&str; 4] = [""; 4];
    let concat_index = ip_from_arr.len() - ip_to_vec.len();
    for x in 0..4 {
        ip_to_arr[x] = if x < concat_index {
            ip_from_arr[x]
        } else {
            ip_to_vec[x-concat_index]
        }
    }
    let ip_from_arr = ip_from_arr.map(|a| a.parse::<u8>().unwrap());
    let ip_to_arr = ip_to_arr.map(|a| a.parse::<u8>().unwrap());

    let mut ip_from_big_int = 0u32;
    let mut ip_to_big_int = 0u32;
    // [192, 168, 2, 2] >>> 3232236034
    for x in 0..4 {
        ip_from_big_int <<= 8;
        ip_from_big_int += ip_from_arr[x] as u32;
        ip_to_big_int <<= 8;
        ip_to_big_int += ip_to_arr[x] as u32;
    }

    trace!("ip-range_to-list: {:?} {:?} {} {} ",ip_from_arr, ip_to_arr, ip_from_big_int, ip_to_big_int);


    let mut ip_vec: Vec<String> = vec![];
    for mut ip_big_int in ip_from_big_int..=ip_to_big_int {
        let mut ip_u8_arr = [0u8; 4];
        for x in 0..4 {
             let ip_shift_int = ip_big_int >> ((3-x)*8); // 3232236034 >> 24 == 192
            //  ip_str[x] = &ip_u8_int.to_string();
            ip_big_int -= ip_shift_int << ((3-x)*8);  // 减去从左侧起第一个8位的数
            ip_u8_arr[x] = ip_shift_int as u8;
        }
        // trace!("ip_u8_arr: {:?}", ip_u8_arr);
        let ip_str = ip_u8_arr.map(|a| a.to_string()).join(".");
        ip_vec.push(ip_str);
    }


    // let x: Vec<&str> = ip_range.split("-").collect();
    // trace!("split: {:?}", x);
    // let ip1 = x[0];
    // let ip2 = x[1];
    // let ip1_string = ip_str_to_hex(ip1);
    // trace!("ip_A to hex: {}", ip1_string);
    // let ip2_string = ip_str_to_hex(ip2);
    // trace!("ip_B to hex: {}", ip2_string);
    // // 转为十进制表示
    // let ip1_int = u64::from_str_radix(ip1_string.as_str(), 16).unwrap();
    // let ip2_int = u64::from_str_radix(ip2_string.as_str(), 16).unwrap();
    // trace!("ip_A_B: {} {}", ip1_int, ip2_int);
    // // let ip2_int = ip_str_to_hex(ip2).parse::<u64>().unwrap();
    // let mut ip_vec: Vec<String> = vec![];
    // for ip in ip1_int..=ip2_int {
    //     // trace!("ip: {}", ip);
    //     // if ip%256 == 0 {
    //     //     continue;
    //     // }
    //     // 再转为16进制 ｜ to hex
    //     let ip_str = format!("{:0>8x}", ip);
    //     trace!(
    //         "to hex format --> {}-{}-{}-{}",
    //         &ip_str[0..2],
    //         &ip_str[2..4],
    //         &ip_str[4..6],
    //         &ip_str[6..8]
    //     );
    //     let ip_str_arr = [&ip_str[0..2], &ip_str[2..4], &ip_str[4..6], &ip_str[6..8]];
    //     let ip_u8_arr = ip_str_arr.map(move |a| {
    //         // 两位 hex 转为 十进制表示
    //         u8::from_str_radix(a, 16).unwrap().to_string()
    //     });
    //     ip_vec.push(ip_u8_arr.join("."));
    // }
    ip_vec
}

/// parse ipaddr to hex:
/// "192.168.1.64"  -> "c0a80140"
fn ip_str_to_hex<'a>(s: &'a str) -> String {
    // let ip1_arr : &[&str] = s.split(".").collect();
    let ip1_vec: Vec<&str> = s.split(".").collect();
    let ip1_str_arr: Vec<String> = ip1_vec.into_iter().map(|a| format!("{:0>2x}", a.parse::<u8>().unwrap() ))
        .collect();
    // let ip1_arr: [&str; 4] = ip1_vec.try_into().unwrap();
    // let ip1_str_arr = ip1_arr.map(move |a| format!("{:0>2x}", a.parse::<u8>().unwrap()));
    ip1_str_arr.join("")
    // ip1_str_arr.join("")
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn check_file(path: &str) -> String {
    // std::fs::metadata(path).is_ok();
    let path = path.trim();
    let out_path;// = String::from(path);
    if Path::new(path).is_file() || path.ends_with(".csv") { // file is existed.
        let now: DateTime<Utc> = Utc::now();
        let time_s = now.format("%Y%m%d_%H%M%S").to_string();
        // let s_vec: Vec<&str> = path.split(".").collect();
        // let mut a: String = (&s_vec[0..(s_vec.len() - 1)]).join("");
        // // let f_now = time::strftime("%Y%m%d_%H%M%S", &now).unwrap();
        // // let b = &s_vec[s_vec.len()-1];
        // a.push_str(&time_s);
        // a.push_str(".csv");
        // return a;
            out_path =  
            [path.rsplit_once(".csv").unwrap_or((path, "")).0,
            &time_s, ".csv"].concat().to_string();
        
    } else {
        out_path = [path, ".csv"].concat();
    }
    return out_path;
    // let file =  std::fs::try_exists(path);
}


#[cfg(test)]
mod tests {

    #[test]
    fn test_a() {
        let a: Vec<&str> = "11.2.1.1".splitn(4, ".").collect();
        let ip_to_arr: [&str; 4] = a.try_into().unwrap();
        println!("{:?}", ip_to_arr);
        let arr: [&str;4] = [""; 4];
        println!("{:?}", arr);

    }
}