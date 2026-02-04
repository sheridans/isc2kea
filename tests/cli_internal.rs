use isc2kea::cli::run_with_args;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!(
        "isc2kea_cli_{label}_{}_{}",
        std::process::id(),
        nanos
    ));
    path
}

fn write_temp_file(label: &str, contents: &str) -> PathBuf {
    let path = temp_path(label);
    fs::write(&path, contents).expect("write temp file");
    path
}

#[test]
fn run_with_args_rejects_same_input_output() {
    let input = write_temp_file(
        "same_io",
        r#"<?xml version="1.0"?>
<opnsense>
  <dhcpd>
    <lan></lan>
  </dhcpd>
</opnsense>
"#,
    );

    let result = run_with_args([
        "isc2kea",
        "convert",
        "--in",
        input.to_str().unwrap(),
        "--out",
        input.to_str().unwrap(),
    ]);

    let err = result.expect_err("should fail on same input/output");
    assert!(err
        .to_string()
        .contains("Output path must be different from input path"));
}

#[test]
fn run_with_args_requires_force_for_existing_output() {
    let input = write_temp_file(
        "existing_out_in",
        r#"<?xml version="1.0"?>
<opnsense>
  <dhcpd>
    <lan></lan>
  </dhcpd>
  <Kea>
    <dhcp4>
      <subnets>
        <subnet4 uuid="test-subnet">
          <subnet>192.168.1.0/24</subnet>
        </subnet4>
      </subnets>
      <reservations></reservations>
    </dhcp4>
  </Kea>
</opnsense>
"#,
    );
    let output_path = write_temp_file("existing_out_out", "<opnsense></opnsense>");

    let result = run_with_args([
        "isc2kea",
        "convert",
        "--in",
        input.to_str().unwrap(),
        "--out",
        output_path.to_str().unwrap(),
    ]);

    let err = result.expect_err("should fail when output exists");
    assert!(err.to_string().contains("Output file already exists"));
}

#[test]
fn run_with_args_scan_missing_input() {
    let input = temp_path("missing_input");

    let result = run_with_args(["isc2kea", "scan", "--in", input.to_str().unwrap()]);
    let err = result.expect_err("should fail for missing input");
    assert!(err.to_string().contains("Failed to open input file"));
}

#[test]
fn run_with_args_convert_writes_output() {
    let input = write_temp_file(
        "convert_ok_in",
        r#"<?xml version="1.0"?>
<opnsense>
  <dhcpd>
    <lan>
      <staticmap>
        <mac>00:11:22:33:44:55</mac>
        <ipaddr>192.168.1.10</ipaddr>
        <hostname>testhost</hostname>
      </staticmap>
    </lan>
  </dhcpd>
  <Kea>
    <dhcp4>
      <subnets>
        <subnet4 uuid="test-subnet">
          <subnet>192.168.1.0/24</subnet>
        </subnet4>
      </subnets>
      <reservations></reservations>
    </dhcp4>
  </Kea>
</opnsense>
"#,
    );
    let output_path = temp_path("convert_ok_out");

    let result = run_with_args([
        "isc2kea",
        "convert",
        "--in",
        input.to_str().unwrap(),
        "--out",
        output_path.to_str().unwrap(),
    ]);

    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn run_with_args_convert_force_overwrites_output() {
    let input = write_temp_file(
        "convert_force_in",
        r#"<?xml version="1.0"?>
<opnsense>
  <dhcpd>
    <lan>
      <staticmap>
        <mac>00:11:22:33:44:55</mac>
        <ipaddr>192.168.1.10</ipaddr>
        <hostname>testhost</hostname>
      </staticmap>
    </lan>
  </dhcpd>
  <Kea>
    <dhcp4>
      <subnets>
        <subnet4 uuid="test-subnet">
          <subnet>192.168.1.0/24</subnet>
        </subnet4>
      </subnets>
      <reservations></reservations>
    </dhcp4>
  </Kea>
</opnsense>
"#,
    );
    let output_path = write_temp_file("convert_force_out", "<opnsense></opnsense>");

    let result = run_with_args([
        "isc2kea",
        "convert",
        "--in",
        input.to_str().unwrap(),
        "--out",
        output_path.to_str().unwrap(),
        "--force",
    ]);

    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn run_with_args_scan_backend_not_configured() {
    let input = write_temp_file(
        "scan_backend_missing",
        r#"<?xml version="1.0"?>
<opnsense>
  <dhcpd>
    <lan>
      <staticmap>
        <mac>00:11:22:33:44:55</mac>
        <ipaddr>192.168.1.10</ipaddr>
        <hostname>testhost</hostname>
      </staticmap>
    </lan>
  </dhcpd>
</opnsense>
"#,
    );

    let result = run_with_args(["isc2kea", "scan", "--in", input.to_str().unwrap()]);
    let err = result.expect_err("should fail when Kea is not configured");
    assert!(err.to_string().contains("Kea"));
}

#[test]
fn run_with_args_scan_backend_not_configured_with_options() {
    let input = write_temp_file(
        "scan_backend_missing_opts",
        r#"<?xml version="1.0"?>
<opnsense>
  <interfaces>
    <lan>
      <ipaddr>192.168.1.1</ipaddr>
      <subnet>24</subnet>
    </lan>
  </interfaces>
  <dhcpd>
    <lan>
      <range>
        <from>192.168.1.100</from>
        <to>192.168.1.200</to>
      </range>
      <staticmap>
        <mac>00:11:22:33:44:55</mac>
        <ipaddr>192.168.1.10</ipaddr>
        <hostname>testhost</hostname>
      </staticmap>
    </lan>
  </dhcpd>
</opnsense>
"#,
    );

    let result = run_with_args([
        "isc2kea",
        "scan",
        "--in",
        input.to_str().unwrap(),
        "--create-subnets",
    ]);
    let err = result.expect_err("should fail when Kea is not configured");
    assert!(err.to_string().contains("Kea"));
}

#[test]
fn run_with_args_convert_dnsmasq_writes_output() {
    let input = write_temp_file(
        "convert_dnsmasq_in",
        r#"<?xml version="1.0"?>
<opnsense>
  <dhcpd>
    <lan>
      <staticmap>
        <mac>00:11:22:33:44:55</mac>
        <ipaddr>192.168.1.10</ipaddr>
        <hostname>testhost</hostname>
      </staticmap>
    </lan>
  </dhcpd>
  <dnsmasq></dnsmasq>
</opnsense>
"#,
    );
    let output_path = temp_path("convert_dnsmasq_out");

    let result = run_with_args([
        "isc2kea",
        "convert",
        "--in",
        input.to_str().unwrap(),
        "--out",
        output_path.to_str().unwrap(),
        "--backend",
        "dnsmasq",
    ]);

    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn run_with_args_scan_dnsmasq_fail_if_existing() {
    let input = write_temp_file(
        "scan_dnsmasq_existing",
        r#"<?xml version="1.0"?>
<opnsense>
  <dhcpd>
    <lan>
      <staticmap>
        <mac>00:11:22:33:44:55</mac>
        <ipaddr>192.168.1.10</ipaddr>
        <hostname>testhost</hostname>
      </staticmap>
    </lan>
  </dhcpd>
  <dnsmasq>
    <hosts uuid="existing-host">
      <hwaddr>99:99:99:99:99:99</hwaddr>
      <ip>192.168.1.10</ip>
      <host>existing</host>
    </hosts>
  </dnsmasq>
</opnsense>
"#,
    );

    let result = run_with_args([
        "isc2kea",
        "scan",
        "--in",
        input.to_str().unwrap(),
        "--backend",
        "dnsmasq",
        "--fail-if-existing",
    ]);

    let err = result.expect_err("should fail when dnsmasq has existing hosts");
    assert!(err.to_string().contains("Existing dnsmasq hosts found"));
}

#[test]
fn run_with_args_convert_with_create_flags() {
    let input = write_temp_file(
        "convert_flags_in",
        r#"<?xml version="1.0"?>
<opnsense>
  <interfaces>
    <lan>
      <ipaddr>192.168.1.1</ipaddr>
      <subnet>24</subnet>
    </lan>
  </interfaces>
  <dhcpd>
    <lan>
      <range>
        <from>192.168.1.100</from>
        <to>192.168.1.200</to>
      </range>
      <dnsserver>8.8.8.8</dnsserver>
      <dnsserver>1.1.1.1</dnsserver>
      <gateway>192.168.1.1</gateway>
      <domain>example.com</domain>
      <domainsearchlist>example2.com example3.com</domainsearchlist>
      <ntpserver>192.168.1.2</ntpserver>
      <staticmap>
        <mac>00:11:22:33:44:55</mac>
        <ipaddr>192.168.1.10</ipaddr>
        <hostname>testhost</hostname>
      </staticmap>
    </lan>
  </dhcpd>
  <Kea>
    <dhcp4>
      <subnets></subnets>
      <reservations></reservations>
    </dhcp4>
  </Kea>
</opnsense>
"#,
    );
    let output_path = temp_path("convert_flags_out");

    let result = run_with_args([
        "isc2kea",
        "convert",
        "--in",
        input.to_str().unwrap(),
        "--out",
        output_path.to_str().unwrap(),
        "--create-subnets",
        "--force-subnets",
        "--create-options",
        "--force-options",
    ]);

    assert!(result.is_ok());
    assert!(output_path.exists());
}
