use crate::PingResult;
use chrono::{DateTime, Utc};
use log::trace;
#[allow(unused_imports)]
use log::{debug, error, info, log, warn};
use pnet::packet::icmp::echo_request;
use pnet::packet::icmp::IcmpTypes;
use pnet::packet::icmpv6::{Icmpv6Types, MutableIcmpv6Packet};
use pnet::packet::Packet;
use pnet::transport::TransportSender;
use pnet::util;
use pnet_macros_support::types::*;
use rand::random;
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

pub struct Ping {
    addr: IpAddr,
    identifier: u16,
    sequence_number: u16,
    pub seen: bool,
    pub send_time: Instant,
}

pub struct ReceivedPing {
    pub addr: IpAddr,
    pub identifier: u16,
    pub sequence_number: u16,
    pub rtt: Duration,
    pub recv_time: Instant,
}

impl Ping {
    pub fn new(addr: IpAddr) -> Ping {
        let mut identifier = 0;
        if addr.is_ipv4() {
            identifier = random::<u16>();
        }
        let send_time = Instant::now();
        Ping {
            addr,
            identifier,
            sequence_number: 0,
            seen: false,
            send_time,
        }
    }

    pub fn get_identifier(&self) -> u16 {
         self.identifier
    }

    pub fn get_sequence_number(&self) -> u16 {
         self.sequence_number
    }

    pub fn increment_sequence_number(&mut self) -> u16 {
        self.sequence_number += 1;
        self.sequence_number
    }
}

fn send_echo(
    tx: &mut TransportSender,
    ping: &mut Ping,
    size: usize,
) -> Result<usize, std::io::Error> {
    // Allocate enough space for a new packet
    let mut vec: Vec<u8> = vec![0; size];

    // Use echo_request so we can set the identifier and sequence number
    let mut echo_packet = echo_request::MutableEchoRequestPacket::new(&mut vec[..]).unwrap();
    echo_packet.set_sequence_number(ping.increment_sequence_number());
    echo_packet.set_identifier(ping.get_identifier());
    echo_packet.set_icmp_type(IcmpTypes::EchoRequest);

    let csum = icmp_checksum(&echo_packet);
    echo_packet.set_checksum(csum);

    tx.send_to(echo_packet, ping.addr)
}

fn send_echov6(
    tx: &mut TransportSender,
    addr: IpAddr,
    size: usize,
) -> Result<usize, std::io::Error> {
    // Allocate enough space for a new packet
    let mut vec: Vec<u8> = vec![0; size];

    let mut echo_packet = MutableIcmpv6Packet::new(&mut vec[..]).unwrap();
    echo_packet.set_icmpv6_type(Icmpv6Types::EchoRequest);

    let csum = icmpv6_checksum(&echo_packet);
    echo_packet.set_checksum(csum);

    tx.send_to(echo_packet, addr)
}

#[allow(clippy::too_many_arguments)]
pub fn send_pings(
    size: usize,
    timer: &Arc<RwLock<Instant>>,
    stop: &Arc<Mutex<bool>>,
    results_sender: &Sender<PingResult>,
    thread_rx: &Arc<Mutex<Receiver<ReceivedPing>>>,
    tx: &Arc<Mutex<TransportSender>>,
    txv6: &Arc<Mutex<TransportSender>>,
    targets: &Arc<Mutex<BTreeMap<IpAddr, Ping>>>,
    max_rtt: &Arc<Duration>,
    interval: u64,
    // is_log: bool,
) {
    // loop {
    let start_time_0 = Instant::now();
    for (addr, ping) in targets.lock().unwrap().iter_mut() {
        ping.send_time = Instant::now();
        if let Err(e) = if addr.is_ipv4() {
            send_echo(&mut tx.lock().unwrap(), ping, size)
        } else if addr.is_ipv6() {
            send_echov6(&mut txv6.lock().unwrap(), *addr, size)
        } else {
            Ok(0)
        } {
            error!("Failed to send ping to {:?}: {}", *addr, e);
        }
        // {
        //     Err(e) => error!("Failed to send ping to {:?}: {}", *addr, e),
        //     _ => {}
        // }
        ping.seen = false;

        thread::sleep(Duration::from_micros(interval));
    }
    debug!("send elapsed: {:?}", start_time_0.elapsed());
    // if is_log {  println!("send elapsed: {:?}", start_time_0.elapsed()); }
    // thread::sleep(Duration::from_millis(2));
    {
        // start the timer
        let mut timer = timer.write().unwrap();
        *timer = Instant::now();
    }
    loop {
        // use recv_timeout so we don't cause a CPU to needlessly spin
        match thread_rx
            .lock()
            .unwrap()
            // .recv()
            .recv_timeout(Duration::from_millis(10)) //default
            // .recv_timeout(*max_rtt)
        {
            Ok(ping_result) => {
                // let ReceivedPing {
                //         addr,
                //         identifier,
                //         sequence_number,
                //         rtt: _,
                //         recv_time,
                //     } = ping_result  
                    {
                        // Update the address to the ping response being received
                        if let Some(ping) = targets.lock().unwrap().get_mut(&ping_result.addr) {
                            if ping.get_identifier() == ping_result.identifier
                                && ping.get_sequence_number() == ping_result.sequence_number
                            {
                                ping.seen = true;
                                // Send the ping result over the client channel
                                match results_sender.send(PingResult::Receive {
                                    addr: ping_result.addr,
                                    rtt: ping_result.rtt,
                                    recv_duration: ping_result.recv_time.duration_since(ping.send_time),
                                    
                                }) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        if !*stop.lock().unwrap() {
                                            error!(
                                                "Error sending ping result on channel: {}",
                                                e
                                            )
                                        }
                                    }
                                }
                            } else {
                                debug!("Received echo reply from target {}, but sequence_number (expected {} but got {}) and identifier (expected {} but got {}) don't match", 
                                ping_result.addr, 
                                ping.get_sequence_number(),
                                ping_result.sequence_number, 
                                 ping.get_identifier(), 
                                 ping_result.identifier);
                            }
                        }
                    }
                
            }
            Err(_) => {
                // Check we haven't exceeded the max rtt
                let start_time = timer.read().unwrap();
                if Instant::now().duration_since(*start_time) > **max_rtt {
                    trace!("exceeded the max rtt...");
                    break;
                }
            }
        }
    }

    let now: DateTime<Utc> = Utc::now();
    trace!("[{}]received completed", now.format("%H:%M:%S"));
    // if is_log {     }
    // check for addresses which haven't replied
    for (addr, ping) in targets.lock().unwrap().iter() {
        if !ping.seen {
            // Send the ping Idle over the client channel
            match results_sender.send(PingResult::Idle { addr: *addr }) {
                Ok(_) => {}
                Err(e) => {
                    if !*stop.lock().unwrap() {
                        error!("Error sending ping Idle result on channel: {}", e)
                    }
                }
            }
        }
    }
    let now: DateTime<Utc> = Utc::now();
    trace!("[{}]check loss completed", now.format("%H:%M:%S"));
    // if is_log {    }
    // check if we've received the stop signal
    if *stop.lock().unwrap() {
        // return;
    }
    // loop end }
}

fn icmp_checksum(packet: &echo_request::MutableEchoRequestPacket) -> u16be {
    util::checksum(packet.packet(), 1)
}

fn icmpv6_checksum(packet: &MutableIcmpv6Packet) -> u16be {
    util::checksum(packet.packet(), 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping() {
        let mut p = Ping::new("127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(p.get_sequence_number(), 0);
        assert!(p.get_identifier() > 0);

        p.increment_sequence_number();
        assert_eq!(p.get_sequence_number(), 1);
    }
}
