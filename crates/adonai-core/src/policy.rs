use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointExposure {
    LoopbackOnly,
    LanExplicit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindAddress {
    pub host: IpAddr,
    pub port: u16,
}

impl BindAddress {
    pub fn loopback(port: u16) -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port,
        }
    }

    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }

    pub fn exposure(&self) -> EndpointExposure {
        if self.host.is_loopback() {
            EndpointExposure::LoopbackOnly
        } else {
            EndpointExposure::LanExplicit
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointPolicy {
    pub bind: BindAddress,
    pub exposure: EndpointExposure,
}

impl EndpointPolicy {
    pub fn loopback(port: u16) -> Self {
        let bind = BindAddress::loopback(port);

        Self {
            exposure: bind.exposure(),
            bind,
        }
    }

    pub fn from_bind(bind: BindAddress) -> Self {
        Self {
            exposure: bind.exposure(),
            bind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_policy_is_default_local_only() {
        let policy = EndpointPolicy::loopback(49231);

        assert_eq!(policy.exposure, EndpointExposure::LoopbackOnly);
        assert_eq!(policy.bind.socket_addr().to_string(), "127.0.0.1:49231");
    }

    #[test]
    fn non_loopback_bind_requires_explicit_lan_exposure() {
        let bind = BindAddress {
            host: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: 49231,
        };

        let policy = EndpointPolicy::from_bind(bind);

        assert_eq!(policy.exposure, EndpointExposure::LanExplicit);
    }
}
