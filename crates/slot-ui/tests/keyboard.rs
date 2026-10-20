use slot_ui::{all_keys, Key, Keyboard, Layer};

fn type_word(k: &mut Keyboard, word: &str) {
    for c in word.chars() {
        k.type_char(c);
    }
}

#[test]
fn starts_on_q_and_moves_around_the_grid() {
    let mut k = Keyboard::default();
    assert_eq!(k.key(), Key::Char('Q'));
    k.up();
    assert_eq!(k.key(), Key::Char('1'));
    k.up();
    assert_eq!(k.key(), Key::Shift);
    k.left();
    assert_eq!(k.key(), Key::Done);
    k.right();
    assert_eq!(k.key(), Key::Shift);
    k.right();
    assert_eq!(k.key(), Key::Space);
}

#[test]
fn shift_cycles_upper_lower_symbols() {
    let mut k = Keyboard::default();
    assert_eq!(k.layer(), Layer::Upper);
    k.shift();
    assert_eq!(k.key(), Key::Char('q'));
    k.shift();
    assert_eq!(k.layer(), Layer::Symbols);
    k.shift();
    assert_eq!(k.layer(), Layer::Upper);
}

#[test]
fn press_types_deletes_and_finishes() {
    let mut k = Keyboard::default();
    assert!(!k.press());
    assert_eq!(k.text(), "Q");
    for _ in 0..2 {
        k.up();
    }
    k.right();
    k.right();
    assert_eq!(k.key(), Key::Delete);
    k.press();
    assert_eq!(k.text(), "");
    k.right();
    assert!(k.press());
}

#[test]
fn passwords_are_dotted_but_the_last_letter_reads() {
    let mut k = Keyboard::password(63);
    assert_eq!(k.key(), Key::Char('q'));
    type_word(&mut k, "hunter2");
    assert_eq!(k.text(), "hunter2");
    assert_eq!(k.shown(), "••••••2");
}

#[test]
fn entry_stops_at_its_cap() {
    let mut k = Keyboard::password(8);
    type_word(&mut k, "0123456789");
    assert_eq!(k.text(), "01234567");
}

#[test]
fn every_key_has_one_face_slot_and_wpa_symbols_are_reachable() {
    let keys = all_keys();
    for (i, a) in keys.iter().enumerate() {
        assert!(!keys[i + 1..].contains(a), "{a:?} twice");
    }
    for c in r#"!@#$%&*-_.=+?/\|'";:,~^`()[]{}<>"#.chars() {
        assert!(keys.contains(&Key::Char(c)), "{c} cannot be typed");
    }
}
