use std::fmt;

#[derive(Debug, Clone, Default, clap::ValueEnum)]
pub enum Backend {
    /// Kea DHCP (default)
    #[default]
    Kea,
    /// dnsmasq DHCP
    Dnsmasq,
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Backend::Kea => write!(f, "Kea"),
            Backend::Dnsmasq => write!(f, "dnsmasq"),
        }
    }
}
