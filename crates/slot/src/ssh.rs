//! SSH and SFTP: dropbear server control and password management.
//!
//! On the device everything goes through `card/System/slotssh.sh`, which manages BaseOS's
//! dropbear daemon and root password. Dropbear delegates SFTP directly to BaseOS's
//! `/usr/libexec/sftp-server`. Off the device a pretend server stands in.

use std::path::{Path, PathBuf};

/// The file on the card BaseOS reads its settings from, `ssh_password` among them.
pub const BASEOS_CONF: &str = "baseos.conf";

/// root's default password when baseos.conf sets none.
pub const DEFAULT_PASSWORD: &str = "root";

pub const SSH_PORT: u16 = 22;

pub fn connection_string(ip: &str) -> String {
    format!("ssh root@{ip} (port {SSH_PORT}, SFTP enabled)")
}

pub trait SshServer: Send {
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self);
    fn is_running(&mut self) -> bool;
    fn set_password(&mut self, password: &str) -> Result<(), String>;
}

/// Reads the password from the text of baseos.conf: the value of the last
/// `ssh_password=` line not commented out. None when absent or empty.
pub fn conf_password(conf: &str) -> Option<String> {
    let mut password = None;
    for line in conf.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "ssh_password" {
            let val = value.trim();
            if !val.is_empty() {
                password = Some(val.to_string());
            }
        }
    }
    password
}

pub fn ssh_password_from_conf(conf: &str) -> String {
    conf_password(conf).unwrap_or_else(|| DEFAULT_PASSWORD.to_string())
}

/// Reads the password from `root/baseos.conf`, or "root" if absent.
pub fn read_ssh_password(root: Option<&Path>) -> String {
    root.and_then(|r| std::fs::read_to_string(r.join(BASEOS_CONF)).ok())
        .map(|c| ssh_password_from_conf(&c))
        .unwrap_or_else(|| DEFAULT_PASSWORD.to_string())
}

/// Updates or appends `ssh_password=<password>` in `root/baseos.conf`.
pub fn write_baseos_conf_password(root: &Path, password: &str) -> std::io::Result<()> {
    let path = root.join(BASEOS_CONF);
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut replaced = false;
    let mut lines: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') && trimmed.starts_with("ssh_password") {
            if let Some((key, _)) = trimmed.split_once('=') {
                if key.trim() == "ssh_password" {
                    lines.push(format!("ssh_password={password}"));
                    replaced = true;
                    continue;
                }
            }
        }
        lines.push(line.to_string());
    }

    if !replaced {
        if !lines.is_empty() && !lines.last().is_some_and(|l| l.is_empty()) {
            lines.push(String::new());
        }
        lines.push(format!("ssh_password={password}"));
    }

    let mut output = lines.join("\n");
    output.push('\n');
    std::fs::write(path, output)
}

/// Whether the system has SSH capabilities.
pub fn present() -> bool {
    #[cfg(all(feature = "device", unix))]
    {
        Path::new("/usr/sbin/dropbear").exists() || Path::new("/etc/init.d/dev").exists()
    }
    #[cfg(not(all(feature = "device", unix)))]
    {
        true
    }
}

pub fn for_card(root: &Path) -> Box<dyn SshServer> {
    #[cfg(feature = "device")]
    {
        Box::new(DeviceSsh {
            root: root.to_path_buf(),
        })
    }
    #[cfg(not(feature = "device"))]
    {
        let _ = root;
        Box::new(SimSsh::default())
    }
}

pub struct DeviceSsh {
    root: PathBuf,
}

impl DeviceSsh {
    fn script(&self, args: &[&str], input: Option<&str>) -> std::process::Output {
        let mut cmd = std::process::Command::new("/bin/sh");
        cmd.arg(self.root.join("System/slotssh.sh")).args(args);
        if input.is_some() {
            cmd.stdin(std::process::Stdio::piped());
        }
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                eprintln!("slot: ssh: spawn error: {e}");
                return std::process::Output {
                    status: std::process::ExitStatus::default(),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                };
            }
        };
        if let Some(text) = input {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
        }
        child.wait_with_output().unwrap_or_else(|e| {
            eprintln!("slot: ssh: {e}");
            std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            }
        })
    }

    fn ran(&self, args: &[&str], input: Option<&str>) -> bool {
        self.script(args, input).status.success()
    }
}

impl SshServer for DeviceSsh {
    fn start(&mut self) -> Result<(), String> {
        match self.ran(&["on"], None) {
            true => Ok(()),
            false => Err("SSH server would not start".into()),
        }
    }

    fn stop(&mut self) {
        self.ran(&["off"], None);
    }

    fn is_running(&mut self) -> bool {
        self.ran(&["status"], None)
    }

    fn set_password(&mut self, password: &str) -> Result<(), String> {
        match self.ran(&["password", password], None) {
            true => Ok(()),
            false => Err("Could not set password".into()),
        }
    }
}

#[derive(Default)]
pub struct SimSsh {
    pub running: bool,
    pub password: Option<String>,
}

impl SshServer for SimSsh {
    fn start(&mut self) -> Result<(), String> {
        self.running = true;
        Ok(())
    }

    fn stop(&mut self) {
        self.running = false;
    }

    fn is_running(&mut self) -> bool {
        self.running
    }

    fn set_password(&mut self, password: &str) -> Result<(), String> {
        self.password = Some(password.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conf_password_reads_last_uncommented_password() {
        let conf = "# baseos config\n\
                    # ssh_password=commented\n\
                    ssh_password=first\n\
                    something_else=1\n\
                    ssh_password=final_secret\n";
        assert_eq!(conf_password(conf), Some("final_secret".to_string()));
    }

    #[test]
    fn conf_password_ignores_empty_and_defaults_to_root() {
        let conf = "# baseos config\nssh_password=\n";
        assert_eq!(conf_password(conf), None);
        assert_eq!(ssh_password_from_conf(conf), "root");
    }

    #[test]
    fn writing_baseos_conf_replaces_existing_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let conf_path = tmp.path().join(BASEOS_CONF);
        std::fs::write(&conf_path, "foo=bar\nssh_password=old\nbaz=1\n").unwrap();

        write_baseos_conf_password(tmp.path(), "new_secret").unwrap();

        let read_back = std::fs::read_to_string(&conf_path).unwrap();
        assert!(read_back.contains("ssh_password=new_secret"));
        assert!(!read_back.contains("ssh_password=old"));
        assert!(read_back.contains("foo=bar"));
    }

    #[test]
    fn writing_baseos_conf_appends_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        write_baseos_conf_password(tmp.path(), "hunter2").unwrap();

        let read_back = std::fs::read_to_string(tmp.path().join(BASEOS_CONF)).unwrap();
        assert_eq!(read_back.trim(), "ssh_password=hunter2");
    }

    #[test]
    fn connection_string_formats_correctly() {
        assert_eq!(
            connection_string("192.168.1.50"),
            "ssh root@192.168.1.50 (port 22, SFTP enabled)"
        );
    }

    #[test]
    fn sim_ssh_starts_stops_and_updates_password() {
        let mut sim = SimSsh::default();
        assert!(!sim.is_running());
        sim.start().unwrap();
        assert!(sim.is_running());
        sim.set_password("mypass").unwrap();
        assert_eq!(sim.password.as_deref(), Some("mypass"));
        sim.stop();
        assert!(!sim.is_running());
    }
}
