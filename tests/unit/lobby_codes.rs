use super::*;

#[test]
fn share_codes_match_the_six_character_join_format() {
    for _ in 0..64 {
        let code = generate_game_code();
        assert_eq!(code.len(), 6);
        assert!(!code.contains('-'));
        assert!(valid_game_code(&code));
    }
    assert!(valid_game_code(" 4t2h6k "));
    assert!(valid_game_code("oil234"), "Accept the same readable aliases as SQL");
    assert!(valid_game_code("C385-8694"), "Existing hosted games remain accessible");
    for invalid in ["", "4T2H6", "4T2H6K9", "ABC?EF", "ABCÜEF", "AAAAAAZZ"] {
        assert!(!valid_game_code(invalid), "Unexpectedly accepted {invalid}");
    }
}

#[test]
fn recovery_accepts_compact_codes_and_existing_saves() {
    for code in [
        "4T2H6K9PW3RX",
        " 4t2h 6k9p w3rx ",
        "4T2H-6K9P-W3RX-Y7CZ",
        "8BD3042BCA6A4D91BEE7CF1C4EB1F81B",
    ] {
        assert!(valid_recovery_code(code), "Rejected {code}");
    }
    for invalid in ["4T2H6K9PW3R", "4T2H6K9PW3RXY", "4T2H6K9PW3R?", "Z".repeat(32).as_str()] {
        assert!(!valid_recovery_code(invalid), "Unexpectedly accepted {invalid}");
    }
}
