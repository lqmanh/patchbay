pub mod singbox;

use crate::config::Mode;
use crate::domains::DomainRule;

#[derive(Debug, Clone, PartialEq)]
pub struct EngineOptions {
    pub socks_port: u16,
    pub mode: Mode,
    pub domains: Vec<DomainRule>,
}
