//! Test cases for bct bracer handling of unmatched brackets.
//!
//! These inputs previously triggered bug!() panics at:
//! - bracer.rs:147 when next_token_index > next_removed_close.0
//! - bracer.rs:203 when there's a branch but no token
//!
//! The problem occurred when unmatched closing brackets appeared in
//! conjunction with other bracket structures. The fix skips removed_closes
//! that fall behind the iterator position after exiting a branch.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// Returns Ok(token_count) on success, Err(panic_message) on panic.
fn dbglex(s: &str) -> Result<usize, String> {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let db = bct::Database::default();
        let source = bct::input::Source::new(&db, s.to_string());
        let chunk = bct::source_map::basic_source_map(&db, source);
        let chunk_lex = bct::lexer::lex_chunk(&db, chunk);
        let bracer = bct::bracer::bracer(&db, chunk_lex);
        // Consume the iterator to trigger any panics.
        let tokens: Vec<_> = bracer.iter(&db).collect();
        tokens.len()
    }));

    result.map_err(|e| {
        if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        }
    })
}

#[test]
fn test_seed_8_delete_opening() {
    let source = r#": @!@@u64] / @[: @u64 / @19, : @u64 / @19]"#;
    assert!(dbglex(source).is_ok(), "Unexpected panic for seed 8 DeleteOpeningBracket");
}

#[test]
fn test_seed_35_delete_opening() {
    let source = r#": !seti32> / set {: i32 / 127, : i32 / 63}"#;
    assert!(dbglex(source).is_ok(), "Unexpected panic for seed 35 DeleteOpeningBracket");
}

#[test]
fn test_seed_37_delete_opening() {
    let source = r#": ##i16) / #(: #i16 / #35)"#;
    assert!(dbglex(source).is_ok(), "Unexpected panic for seed 37 DeleteOpeningBracket");
}

#[test]
fn test_seed_42_delete_opening() {
    let source = r#": #map#i64, #f32> / #map {: #i64 / #0 = : #f32 / #209.6, : #i64 / #59 = : #f32 / #12.8, : #i64 / #108 = : #f32 / #118.2}"#;
    assert!(dbglex(source).is_ok(), "Unexpected panic for seed 42 DeleteOpeningBracket");
}

#[test]
fn test_seed_57_delete_opening() {
    let source = r#": #map#int, #bool> / #map {: #int / #119 = : #bool / #false, : #int / #20 = : #bool / #false}"#;
    assert!(dbglex(source).is_ok(), "Unexpected panic for seed 57 DeleteOpeningBracket");
}

#[test]
fn test_seed_60_delete_opening() {
    let source = r#": #enum Gen44(#i16), GenType17} / #enum Gen44(: #i16 / #63)"#;
    assert!(dbglex(source).is_ok(), "Unexpected panic for seed 60 DeleteOpeningBracket");
}

#[test]
fn test_seed_63_delete_opening() {
    let source = r#": y1: string} / {y1 = : string / "value"}"#;
    assert!(dbglex(source).is_ok(), "Unexpected panic for seed 63 DeleteOpeningBracket");
}

#[test]
fn test_seed_77_delete_opening() {
    let source = r#": #data / #data : ##i64, #u16) / #(: #i64 / #99, : #u16 / #61)"#;
    assert!(dbglex(source).is_ok(), "Unexpected panic for seed 77 DeleteOpeningBracket");
}

#[test]
fn test_seed_8_extra_closing() {
    let source = r#": @!@[@u64] /] @[: @u64 / @19, : @u64 / @19]"#;
    assert!(dbglex(source).is_ok(), "Unexpected panic for seed 8 ExtraClosingBracket");
}

#[test]
fn test_seed_9_extra_closing() {
    let source = r#": @map<@bool, @!@u64> /} @map {: @bool / @false = : @!@u64 / @254}"#;
    assert!(dbglex(source).is_ok(), "Unexpected panic for seed 9 ExtraClosingBracket");
}

#[test]
fn test_seed_58_extra_closing() {
    let source = r#": #data / #data : #(#u)64, #bool) / #(: #u64 / #56, : #bool / #false)"#;
    assert!(dbglex(source).is_ok(), "Unexpected panic for seed 58 ExtraClosingBracket");
}

#[test]
fn test_seed_75_extra_closing() {
    let source = r#": [u8] /] [: u8 / 180, : u8 / 202]"#;
    assert!(dbglex(source).is_ok(), "Unexpected panic for seed 75 ExtraClosingBracket");
}
