// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/tests/config.rs.

use slot_cheevos::config::remember_token;
use slot_cheevos::{Credentials, Secret, CHEEVOS_FILE};

fn root_with(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join(CHEEVOS_FILE), text).unwrap();
    dir
}

#[test]
fn reads_a_username_and_password() {
    let c = Credentials::parse("username = Player\npassword = hunter2\n").unwrap();
    assert_eq!(c.username, "Player");
    assert_eq!(c.secret, Secret::Password("hunter2".into()));
    assert_eq!(c.host, None);
}

#[test]
fn reads_a_remembered_token() {
    let c = Credentials::parse("username=Player\ntoken=abc\n").unwrap();
    assert_eq!(c.secret, Secret::Token("abc".into()));
}

#[test]
fn a_freshly_typed_password_beats_an_old_token() {
    let c = Credentials::parse("username=Player\ntoken=stale\npassword=new\n").unwrap();
    assert_eq!(c.secret, Secret::Password("new".into()));
}

#[test]
fn needs_both_a_username_and_a_secret() {
    assert_eq!(Credentials::parse("password=hunter2\n"), None);
    assert_eq!(Credentials::parse("username=Player\n"), None);
    assert_eq!(Credentials::parse("username=\npassword=x\n"), None);
    assert_eq!(Credentials::parse("username=Player\npassword=\n"), None);
}

#[test]
fn an_optional_host_points_at_another_server() {
    let c = Credentials::parse("username=P\ntoken=t\nhost=http://localhost:8080\n").unwrap();
    assert_eq!(c.host.as_deref(), Some("http://localhost:8080"));
}

#[test]
fn a_missing_file_means_no_credentials() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(Credentials::read(dir.path()), None);
}

#[test]
fn remembering_the_token_drops_the_password_and_keeps_everything_else() {
    let dir = root_with("# my account\nusername = Player\npassword = hunter2\nhost = x\n");
    remember_token(dir.path(), "abc").unwrap();
    let text = std::fs::read_to_string(dir.path().join(CHEEVOS_FILE)).unwrap();
    assert_eq!(
        text,
        "# my account\nusername = Player\nhost = x\ntoken = abc\n"
    );
    assert_eq!(
        Credentials::read(dir.path()).unwrap().secret,
        Secret::Token("abc".into())
    );
    remember_token(dir.path(), "abc").unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join(CHEEVOS_FILE)).unwrap(),
        text
    );
    let leftovers: Vec<_> = std::fs::read_dir(dir.path().join("Config"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(leftovers, ["retroachievements.txt"]);
}
