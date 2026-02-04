use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!("isc2kea_{label}_{}_{}", std::process::id(), nanos));
    path
}

fn write_temp_file(label: &str, contents: &str) -> PathBuf {
    let path = temp_path(label);
    fs::write(&path, contents).expect("write temp file");
    path
}

#[test]
fn test_cli_convert_rejects_same_input_output() {
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

    let exe = env!("CARGO_BIN_EXE_isc2kea");
    let output = Command::new(exe)
        .args(["convert", "--in"])
        .arg(&input)
        .args(["--out"])
        .arg(&input)
        .output()
        .expect("run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Output path must be different from input path"));
}

#[test]
fn test_cli_convert_requires_force_for_existing_output() {
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

    let exe = env!("CARGO_BIN_EXE_isc2kea");
    let output = Command::new(exe)
        .args(["convert", "--in"])
        .arg(&input)
        .args(["--out"])
        .arg(&output_path)
        .output()
        .expect("run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Output file already exists"));
}

#[test]
fn test_cli_scan_missing_input() {
    let input = temp_path("missing_input");

    let exe = env!("CARGO_BIN_EXE_isc2kea");
    let output = Command::new(exe)
        .args(["scan", "--in"])
        .arg(&input)
        .output()
        .expect("run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to open input file"));
}

#[test]
fn test_cli_scan_success() {
    let input = write_temp_file(
        "scan_ok",
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
    </dhcp4>
  </Kea>
</opnsense>
"#,
    );

    let exe = env!("CARGO_BIN_EXE_isc2kea");
    let output = Command::new(exe)
        .args(["scan", "--in"])
        .arg(&input)
        .output()
        .expect("run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ISC DHCP static mappings found"));
    assert!(stdout.contains("Kea subnet4 entries found"));
}
