//! Find the addresses a phone on the same Wi-Fi can use to reach this computer.

use std::net::{IpAddr, UdpSocket};

/// LAN IPv4 addresses of this machine, best guess first.
///
/// Uses the "connect a UDP socket and read its local address" trick: connecting a UDP
/// socket sends no packets, it only asks the OS which interface would route there.
pub fn lan_addresses() -> Vec<IpAddr> {
    let probes = ["8.8.8.8:80", "192.168.1.1:9", "192.168.0.1:9", "10.0.0.1:9", "172.16.0.1:9"];
    let mut out: Vec<IpAddr> = Vec::new();
    for probe in probes {
        let Ok(sock) = UdpSocket::bind("0.0.0.0:0") else {
            continue;
        };
        if sock.connect(probe).is_err() {
            continue;
        }
        if let Ok(addr) = sock.local_addr() {
            let ip = addr.ip();
            if !ip.is_loopback() && !ip.is_unspecified() && !out.contains(&ip) {
                out.push(ip);
            }
        }
    }
    out
}

/// This computer's network name (for `name.local` Bonjour/mDNS URLs).
pub fn hostname() -> Option<String> {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .map(|h| h.trim().to_lowercase())
        .filter(|h| !h.is_empty())
}
