extern crate pnet;
extern crate pnet_macros_support;
#[macro_use]
extern crate log;
extern crate rand;

mod ping;

use ping::{send_pings, Ping, ReceivedPing};
use pnet::packet::icmp::echo_reply::EchoReplyPacket;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::Packet;
use pnet::packet::{icmp, icmpv6};
use pnet::transport::transport_channel;
use pnet::transport::TransportChannelType::Layer4;
use pnet::transport::TransportProtocol::{Ipv4, Ipv6};
use pnet::transport::{icmp_packet_iter, icmpv6_packet_iter};
use pnet::transport::{TransportReceiver, TransportSender};
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use std::option::Option::Some;

// result type returned by fastping_rs::Pinger::new()
pub type NewPingerResult = Result<(Pinger, Receiver<PingResult>), String>;

// ping result type.  Idle represents pings that have not received a repsonse within the max_rtt.
// Receive represents pings which have received a repsonse
pub enum PingResult {
    Idle { addr: IpAddr },
    Receive { addr: IpAddr, rtt: Duration, recv_duration: Duration },
}

pub struct Pinger {
    // Number of milliseconds of an idle timeout. Once it passed,
    // the library calls an idle callback function.  Default is 2000
    max_rtt: Arc<Duration>,

    // map of addresses to ping on each run
    targets: Arc<Mutex<BTreeMap<IpAddr, Ping>>>,

    // Size in bytes of the payload to send.  Default is 16 bytes
    size: usize,

    // sender end of the channel for piping results to client
    results_sender: Sender<PingResult>,

    // sender end of libpnet icmp v4 transport channel
    tx: Arc<Mutex<TransportSender>>,

    // receiver end of libpnet icmp v4 transport channel
    rx: Arc<Mutex<TransportReceiver>>,

    // sender end of libpnet icmp v6 transport channel
    txv6: Arc<Mutex<TransportSender>>,

    // receiver end of libpnet icmp v6 transport channel
    rxv6: Arc<Mutex<TransportReceiver>>,

    // sender for internal result passing beween threads
    thread_tx: Sender<ReceivedPing>,

    // receiver for internal result passing beween threads
    thread_rx: Arc<Mutex<Receiver<ReceivedPing>>>,

    // timer for tracking round trip times
    timer: Arc<RwLock<Instant>>,

    // flag to stop pinging
    stop: Arc<Mutex<bool>>,
}

impl Pinger {
    // initialize the pinger and start the icmp and icmpv6 listeners
    pub fn new(_max_rtt: Option<u64>, _size: Option<usize>) -> NewPingerResult {
        let targets = BTreeMap::new();
        let (sender, receiver) = channel();

        let protocol = Layer4(Ipv4(IpNextHeaderProtocols::Icmp));
        let (tx, rx) = match transport_channel(4096, protocol) {
            Ok((tx, rx)) => (tx, rx),
            Err(e) => return Err(e.to_string()),
        };

        let protocolv6 = Layer4(Ipv6(IpNextHeaderProtocols::Icmpv6));
        let (txv6, rxv6) = match transport_channel(4096, protocolv6) {
            Ok((txv6, rxv6)) => (txv6, rxv6),
            Err(e) => return Err(e.to_string()),
        };

        let (thread_tx, thread_rx) = channel();

        let mut pinger = Pinger {
            max_rtt: Arc::new(Duration::from_millis(2000)),
            targets: Arc::new(Mutex::new(targets)),
            size: _size.unwrap_or(16),
            results_sender: sender,
            tx: Arc::new(Mutex::new(tx)),
            rx: Arc::new(Mutex::new(rx)),
            txv6: Arc::new(Mutex::new(txv6)),
            rxv6: Arc::new(Mutex::new(rxv6)),
            thread_rx: Arc::new(Mutex::new(thread_rx)),
            thread_tx,
            timer: Arc::new(RwLock::new(Instant::now())),
            stop: Arc::new(Mutex::new(false)),
        };
        if let Some(rtt_value) = _max_rtt {
            pinger.max_rtt = Arc::new(Duration::from_millis(rtt_value));
        }
        if let Some(size_value) = _size {
            pinger.size = size_value;
        }

        pinger.start_listener();
        Ok((pinger, receiver))
    }

    // add either an ipv4 or ipv6 target address for pinging
    pub fn add_ipaddr(&self, ipaddr: &str) {
        let addr = ipaddr.parse::<IpAddr>();
        match addr {
            Ok(valid_addr) => {
                debug!("Address added {}", valid_addr);
                let new_ping = Ping::new(valid_addr);
                self.targets.lock().unwrap().insert(valid_addr, new_ping);
            }
            Err(e) => {
                error!("Error adding ip address {}. Error: {}", ipaddr, e);
            }
        };
    }

    pub fn get_target_count(&self) -> u32 {
        self.targets.lock().unwrap().len() as u32
    }

    // remove a previously added ipv4 or ipv6 target address
    pub fn remove_ipaddr(&self, ipaddr: &str) {
        let addr = ipaddr.parse::<IpAddr>();
        match addr {
            Ok(valid_addr) => {
                debug!("Address removed {}", valid_addr);
                self.targets.lock().unwrap().remove(&valid_addr);
            }
            Err(e) => {
                error!("Error removing ip address {}. Error: {}", ipaddr, e);
            }
        };
    }

    // stop running the continous pinger
    pub fn stop_pinger(&self) {
        let mut stop = self.stop.lock().unwrap();
        *stop = true;
    }

    // run one round of pinging and stop
    pub fn ping_once(&self) {
        self.run_pings(Some(0))
    }

    // run the continuous pinger
    pub fn run_pinger(&self, n: u32) {
        self.run_pings(Some(n))
    }

    // run pinger either once or continuously
    fn run_pings(&self, run_n_of_times: Option<u32>) {
        let thread_rx = self.thread_rx.clone();
        let tx = self.tx.clone();
        let txv6 = self.txv6.clone();
        let results_sender = self.results_sender.clone();
        let stop = self.stop.clone();
        let targets = self.targets.clone();
        let timer = self.timer.clone();
        let max_rtt = self.max_rtt.clone();
        let size = self.size;

        {
            let mut stop_mut = self.stop.lock().unwrap();
            if let Some(n) = run_n_of_times {
                if n == 1 {
                    *stop_mut = true;
                    debug!("Running pinger for one round");
                } else {
                    *stop_mut = false;
                }
            } else {
                panic!("number of times error!")
            }
        }
        match run_n_of_times.unwrap() {
            0 => {
                thread::spawn(move ||{
                    loop {
                        send_pings(
                            size,
                            &timer,
                            &stop,
                            &results_sender,
                            &thread_rx,
                            &tx,
                            &txv6,
                            &targets,
                            &max_rtt,
                        );
                    }
                });
            },
            ofn => {
                thread::spawn( move || {
                    for _ in 0..ofn {
                        send_pings(
                            size,
                            &timer,
                            &stop,
                            &results_sender,
                            &thread_rx,
                            &tx,
                            &txv6,
                            &targets,
                            &max_rtt,
                        );
                    }
                });

            }
        }
    }

    fn start_listener(&self) {
        // start icmp listeners in the background and use internal channels for results

        // setup ipv4 listener
        let thread_tx = self.thread_tx.clone();
        let rx = self.rx.clone();
        let timer = self.timer.clone();
        let stop = self.stop.clone();

        thread::spawn(move || {
            let mut receiver = rx.lock().unwrap();
            let mut iter = icmp_packet_iter(&mut receiver);
            loop {
                match iter.next() {
                    Ok((packet, addr)) => match EchoReplyPacket::new(packet.packet()) {
                        Some(echo_reply) => {
                            if packet.get_icmp_type() == icmp::IcmpType::new(0) {
                                let start_time = timer.read().unwrap();
                                match thread_tx.send(ReceivedPing {
                                    addr,
                                    identifier: echo_reply.get_identifier(),
                                    sequence_number: echo_reply.get_sequence_number(),
                                    rtt: Instant::now().duration_since(*start_time),
                                    recv_time: Instant::now(),
                                }) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        if !*stop.lock().unwrap() {
                                            error!("Error sending ping result on channel: {}", e)
                                        } else {
                                            return;
                                        }
                                    }
                                }
                            } else {
                                debug!(
                                    "ICMP type other than reply (0) received from {:?}: {:?}",
                                    addr,
                                    packet.get_icmp_type()
                                );
                            }
                        }
                        None => {}
                    },
                    Err(e) => {
                        error!("An error occurred while reading: {}", e);
                    }
                }
            }
        });

        // setup ipv6 listener
        let thread_txv6 = self.thread_tx.clone();
        let rxv6 = self.rxv6.clone();
        let timerv6 = self.timer.clone();
        let stopv6 = self.stop.clone();

        thread::spawn(move || {
            let mut receiver = rxv6.lock().unwrap();
            let mut iter = icmpv6_packet_iter(&mut receiver);
            loop {
                match iter.next() {
                    Ok((packet, addr)) => {
                        if packet.get_icmpv6_type() == icmpv6::Icmpv6Type::new(129) {
                            let start_time = timerv6.read().unwrap();
                            match thread_txv6.send(ReceivedPing {
                                addr,
                                identifier: 0,
                                sequence_number: 0,
                                rtt: Instant::now().duration_since(*start_time),
                                recv_time: Instant::now(),
                            }) {
                                Ok(_) => {}
                                Err(e) => {
                                    if !*stopv6.lock().unwrap() {
                                        error!("Error sending ping result on channel: {}", e)
                                    } else {
                                        return;
                                    }
                                }
                            }
                        } else {
                            debug!(
                                "ICMP type other than reply (129) received from {:?}: {:?}",
                                addr,
                                packet.get_icmpv6_type()
                            );
                        }
                    }
                    Err(e) => {
                        error!("An error occurred while reading: {}", e);
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newpinger() {
        // test we can create a new pinger with optional arguments,
        // test it returns the new pinger and a client channel
        // test we can use the client channel
        match Pinger::new(Some(3000 as u64), Some(24)) {
            Ok((test_pinger, test_channel)) => {
                assert_eq!(test_pinger.max_rtt, Arc::new(Duration::new(3, 0)));
                assert_eq!(test_pinger.size, 24);

                match test_pinger.results_sender.send(PingResult::Idle {
                    addr: "127.0.0.1".parse::<IpAddr>().unwrap(),
                }) {
                    Ok(_) => match test_channel.recv() {
                        Ok(result) => match result {
                            PingResult::Idle { addr } => {
                                assert_eq!(addr, "127.0.0.1".parse::<IpAddr>().unwrap());
                            }
                            _ => {}
                        },
                        Err(_) => assert!(false),
                    },
                    Err(_) => assert!(false),
                }
            }
            Err(e) => {
                println!("Test failed: {}", e);
                assert!(false)
            }
        };
    }

    #[test]
    fn test_add_remove_addrs() {
        match Pinger::new(None, None) {
            Ok((test_pinger, _)) => {
                test_pinger.add_ipaddr("127.0.0.1");
                assert_eq!(test_pinger.targets.lock().unwrap().len(), 1);
                assert!(test_pinger
                    .targets
                    .lock()
                    .unwrap()
                    .contains_key(&"127.0.0.1".parse::<IpAddr>().unwrap()));

                test_pinger.remove_ipaddr("127.0.0.1");
                assert_eq!(test_pinger.targets.lock().unwrap().len(), 0);
                assert_eq!(
                    test_pinger
                        .targets
                        .lock()
                        .unwrap()
                        .contains_key(&"127.0.0.1".parse::<IpAddr>().unwrap()),
                    false
                );
            }
            Err(e) => {
                println!("Test failed: {}", e);
                assert!(false)
            }
        }
    }

    #[test]
    fn test_stop() {
        match Pinger::new(None, None) {
            Ok((test_pinger, _)) => {
                assert_eq!(*test_pinger.stop.lock().unwrap(), false);
                test_pinger.stop_pinger();
                assert_eq!(*test_pinger.stop.lock().unwrap(), true);
            }
            Err(e) => {
                println!("Test failed: {}", e);
                assert!(false)
            }
        }
    }

    #[test]
    fn test_integration() {
        // more comprehensive integration test
        match Pinger::new(None, None) {
            Ok((test_pinger, test_channel)) => {
                let test_addrs = vec!["127.0.0.1", "7.7.7.7", "::1"];
                for target in test_addrs.iter() {
                    test_pinger.add_ipaddr(target);
                }
                test_pinger.ping_once();
                for _ in test_addrs.iter() {
                    match test_channel.recv() {
                        Ok(result) => match result {
                            PingResult::Idle { addr } => {
                                assert_eq!("7.7.7.7".parse::<IpAddr>().unwrap(), addr);
                            }
                            PingResult::Receive { addr, rtt: _, .. } => {
                                if addr == "::1".parse::<IpAddr>().unwrap()
                                    || addr == "127.0.0.1".parse::<IpAddr>().unwrap()
                                {
                                    assert!(true)
                                } else {
                                    assert!(false)
                                }
                            }
                            _ => {
                                assert!(false)
                            }
                        },
                        Err(_) => assert!(false),
                    }
                }
            }
            Err(e) => {
                println!("Test failed: {}", e);
                assert!(false)
            }
        }
    }
}
