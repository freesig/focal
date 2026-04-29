#[test]
fn public_core_code_avoids_panic_style_recoverable_paths() {
    let source = include_str!("../src/lib.rs");

    for forbidden in [
        "unwrap(",
        "expect(",
        "panic!",
        "todo!",
        "unimplemented!",
        "get_unchecked",
    ] {
        assert!(
            !source.contains(forbidden),
            "public core source contains forbidden recoverable-state pattern: {forbidden}"
        );
    }
}
