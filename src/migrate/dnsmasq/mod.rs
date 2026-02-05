use super::options::DnsmasqOptionSpec;

pub(crate) use convert::convert_dnsmasq;
pub(crate) use scan::scan_dnsmasq;

mod convert;
mod scan;

fn range_key(iface: &str, start: &str, end: &str, prefix_len: &str, mask: &str) -> String {
    format!("{}|{}|{}|{}|{}", iface, start, end, prefix_len, mask)
}

fn option_key_for_spec(spec: &DnsmasqOptionSpec) -> String {
    crate::extract_dnsmasq::dnsmasq_option_key(
        "set",
        &spec.option,
        &spec.option6,
        &spec.iface,
        "",
        "",
    )
}
