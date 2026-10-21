// Ported from nickclyde/slot-plus@02c5890, crates/slot-store/tests/find.rs.

use slot_store::find::{fold, matches, rank};

fn rank_of(title: &str, query: &str) -> Option<u8> {
    rank(&fold(title), &fold(query))
}

#[test]
fn a_title_that_starts_with_the_query_ranks_above_one_with_a_word_that_does() {
    assert_eq!(
        rank_of("Pokemon - Emerald Version (USA, Europe)", "pok"),
        Some(0)
    );
    assert_eq!(
        rank_of("Pokemon - Emerald Version (USA, Europe)", "pokemon em"),
        Some(0)
    );
    assert_eq!(
        rank_of("Pokemon - Emerald Version (USA, Europe)", "emerald"),
        Some(1)
    );
}

#[test]
fn a_word_start_ranks_above_a_match_inside_a_word() {
    assert_eq!(rank_of("Golden Sun", "sun"), Some(1));
    assert_eq!(rank_of("Golden Sun", "old"), Some(2));
    let titles = [fold("Arsunal"), fold("Golden Sun"), fold("Sunset Riders")];
    assert_eq!(matches(&titles, &fold("sun")), vec![2, 1, 0]);
}

#[test]
fn region_tags_and_punctuation_neither_match_nor_get_in_the_way() {
    assert_eq!(
        rank_of("Pokemon - Emerald Version (USA, Europe)", "usa"),
        None
    );
    assert_eq!(rank_of("Kirby's Dream Land [!]", "kirbys"), Some(0));
    assert_eq!(rank_of("Kirby's Dream Land", "kirby dream"), Some(1));
    assert_eq!(rank_of("Mr. Driller", "mr driller"), Some(0));
}

#[test]
fn accented_letters_match_their_plain_ones() {
    assert_eq!(rank_of("Pokémon Pinball", "pokemon"), Some(0));
    assert_eq!(rank_of("Pokemon Pinball", "POKÉ"), Some(0));
}

#[test]
fn a_letter_or_two_only_matches_the_start_of_a_word() {
    assert_eq!(rank_of("Pokemon - Emerald Version", "pokemon r"), None);
    assert_eq!(rank_of("Pokemon - Ruby Version", "pokemon r"), Some(0));
    assert_eq!(rank_of("Golden Sun", "ol"), None);
    assert_eq!(rank_of("Golden Sun", "old"), Some(2));
}

#[test]
fn every_word_of_the_query_has_to_match() {
    assert_eq!(rank_of("Golden Sun", "golden moon"), None);
    assert_eq!(rank_of("Golden Sun", ""), None);
    assert_eq!(rank_of("Golden Sun", "   "), None);
}
