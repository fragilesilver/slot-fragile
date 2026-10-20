use slot_store::{read_extras, write_extras, ExtrasState};

#[test]
fn a_missing_file_reads_as_all_defaults() {
    let d = tempfile::tempdir().unwrap();
    let s = read_extras(d.path());
    assert_eq!(s, ExtrasState::default());
    assert!(s.flag("show_favorites", true));
}

#[test]
fn round_trips_and_keeps_keys_it_does_not_know() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("Config")).unwrap();
    std::fs::write(
        d.path().join("Config/extras.state"),
        "from_a_newer_build=7\nclock_24h=0\n",
    )
    .unwrap();
    let mut s = read_extras(d.path());
    assert!(!s.flag("clock_24h", true));
    s.set_flag("show_favorites", false);
    write_extras(d.path(), &s).unwrap();
    let back = read_extras(d.path());
    assert_eq!(back.get("from_a_newer_build"), Some("7"));
    assert!(!back.flag("show_favorites", true));
    assert_eq!(back, s);
}

#[test]
fn garbage_falls_back_to_the_default() {
    let s = ExtrasState::parse("clock_24h=maybe\nbadline\n=nokey\nff=abc\n");
    assert!(s.flag("clock_24h", true));
    assert_eq!(s.number::<u8>("ff"), None);
}

#[test]
fn values_that_would_break_the_file_are_refused() {
    let mut s = ExtrasState::default();
    s.set("ssid", Some("two\nlines"));
    s.set("a=b", Some("x"));
    assert_eq!(s.get("ssid"), None);
    assert_eq!(s.get("a=b"), None);
    s.set("name", Some("has = sign"));
    assert_eq!(ExtrasState::parse(&s.text()).get("name"), Some("has = sign"));
}
