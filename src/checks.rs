use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use eframe::egui;

use crate::dns::{self, Resolver, SERVICES};
use crate::sysdns;

pub const MONITOR_INTERVAL: Duration = Duration::from_secs(12);

pub enum Event {
    Latency(usize, Option<u64>),
    Service(usize, usize, bool),
    Applied(Option<usize>, bool, String),
    Health {
        dns_ok: bool,
        online: bool,
    },
    Config {
        alias: Option<String>,
        servers: Vec<String>,
    },
}

pub struct Engine {
    tx: Sender<Event>,
    ctx: egui::Context,
    monitored: Arc<Mutex<Option<Resolver>>>,
}

impl Engine {
    pub fn new(ctx: egui::Context) -> (Self, Receiver<Event>) {
        let (tx, rx) = mpsc::channel();
        let monitored = Arc::new(Mutex::new(None));

        let engine = Self {
            tx: tx.clone(),
            ctx: ctx.clone(),
            monitored: monitored.clone(),
        };

        let health_tx = tx;
        thread::spawn(move || loop {
            thread::sleep(MONITOR_INTERVAL);
            let current = monitored.lock().ok().and_then(|slot| slot.clone());
            let Some(resolver) = current else {
                continue;
            };

            let dns_ok = dns::query_a(resolver.primary, "example.com", Duration::from_millis(1500))
                .is_some();
            let online = tcp_probe(
                &[Ipv4Addr::new(1, 1, 1, 1)],
                443,
                Duration::from_millis(1500),
            );

            if health_tx.send(Event::Health { dns_ok, online }).is_err() {
                return;
            }
            ctx.request_repaint();
        });

        (engine, rx)
    }

    pub fn measure(&self, resolver: Resolver) {
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        thread::spawn(move || {
            let value = dns::latency(resolver.primary, 4).map(|d| d.as_millis() as u64);
            if tx.send(Event::Latency(resolver.id, value)).is_ok() {
                ctx.request_repaint();
            }
        });
    }

    pub fn check_services(&self, resolver: Resolver) {
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        thread::spawn(move || {
            for (index, service) in SERVICES.iter().enumerate() {
                let ok = service_available(resolver.primary, service.host);
                if tx.send(Event::Service(resolver.id, index, ok)).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        });
    }

    pub fn apply(&self, resolver: Resolver, alias: Option<String>) {
        if let Ok(mut slot) = self.monitored.lock() {
            *slot = Some(resolver.clone());
        }

        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        thread::spawn(move || {
            let result = match alias {
                Some(alias) => sysdns::apply_and_verify(
                    &alias,
                    &resolver.servers(),
                    &resolver.primary.to_string(),
                ),
                None => Err("Не найдено активное подключение".to_owned()),
            };

            let event = match result {
                Ok(()) => Event::Applied(
                    Some(resolver.id),
                    true,
                    format!("Применён DNS {}", resolver.name),
                ),
                Err(message) => Event::Applied(None, false, message),
            };
            if tx.send(event).is_ok() {
                ctx.request_repaint();
            }
        });
    }

    pub fn reset(&self, alias: Option<String>) {
        if let Ok(mut slot) = self.monitored.lock() {
            *slot = None;
        }

        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        thread::spawn(move || {
            let result = match alias {
                Some(alias) => sysdns::reset_and_verify(&alias),
                None => Err("Не найдено активное подключение".to_owned()),
            };

            let event = match result {
                Ok(()) => Event::Applied(None, true, "DNS возвращён к автоматическому".to_owned()),
                Err(message) => Event::Applied(None, false, message),
            };
            if tx.send(event).is_ok() {
                ctx.request_repaint();
            }
        });
    }

    pub fn refresh_config(&self) {
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        thread::spawn(move || {
            let alias = sysdns::active_alias();
            let servers = alias
                .as_deref()
                .map(sysdns::current_servers)
                .unwrap_or_default();
            if tx.send(Event::Config { alias, servers }).is_ok() {
                ctx.request_repaint();
            }
        });
    }

    pub fn probe_health(&self) {
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        let resolver = self.monitored.lock().ok().and_then(|slot| slot.clone());
        thread::spawn(move || {
            let (dns_ok, online) = match resolver {
                Some(resolver) => (
                    dns::query_a(resolver.primary, "example.com", Duration::from_millis(1500))
                        .is_some(),
                    tcp_probe(
                        &[Ipv4Addr::new(1, 1, 1, 1)],
                        443,
                        Duration::from_millis(1500),
                    ),
                ),
                None => (true, true),
            };
            if tx.send(Event::Health { dns_ok, online }).is_ok() {
                ctx.request_repaint();
            }
        });
    }
}

fn service_available(server: Ipv4Addr, host: &str) -> bool {
    let Some((_, addresses)) = dns::query_a(server, host, Duration::from_millis(1800)) else {
        return false;
    };
    tcp_probe(&addresses, 443, Duration::from_millis(1500))
}

fn tcp_probe(addresses: &[Ipv4Addr], port: u16, timeout: Duration) -> bool {
    addresses
        .iter()
        .take(3)
        .any(|ip| TcpStream::connect_timeout(&SocketAddr::from((*ip, port)), timeout).is_ok())
}
