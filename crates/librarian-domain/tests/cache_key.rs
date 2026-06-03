use librarian_domain::cache_key::derive;
use librarian_domain::{ConfigHash, SourceHash, StageVersion};

fn fixture() -> (SourceHash, &'static str, StageVersion, ConfigHash) {
    (
        SourceHash("aaaa".into()),
        "extractor-pdf",
        StageVersion("1.0.0".into()),
        ConfigHash("cfg".into()),
    )
}

#[test]
fn deterministic() {
    let (s, n, v, c) = fixture();
    assert_eq!(derive(&s, n, &v, &c), derive(&s, n, &v, &c));
}

#[test]
fn distinguishes_source_hash() {
    let (_, n, v, c) = fixture();
    assert_ne!(
        derive(&SourceHash("a".into()), n, &v, &c),
        derive(&SourceHash("b".into()), n, &v, &c),
    );
}

#[test]
fn distinguishes_stage_name() {
    let (s, _, v, c) = fixture();
    assert_ne!(derive(&s, "x", &v, &c), derive(&s, "y", &v, &c));
}

#[test]
fn distinguishes_stage_version() {
    let (s, n, _, c) = fixture();
    assert_ne!(
        derive(&s, n, &StageVersion("1".into()), &c),
        derive(&s, n, &StageVersion("2".into()), &c),
    );
}

#[test]
fn distinguishes_config_hash() {
    let (s, n, v, _) = fixture();
    assert_ne!(
        derive(&s, n, &v, &ConfigHash("p".into())),
        derive(&s, n, &v, &ConfigHash("q".into())),
    );
}

#[test]
fn separator_safe_against_concat_collision() {
    // ("ab", "c") vs ("a", "bc") would collide under naive concatenation.
    let v = StageVersion("v".into());
    let c = ConfigHash("c".into());
    let k1 = derive(&SourceHash("ab".into()), "c", &v, &c);
    let k2 = derive(&SourceHash("a".into()), "bc", &v, &c);
    assert_ne!(k1, k2);
}

#[test]
fn output_is_64_char_lowercase_hex() {
    let (s, n, v, c) = fixture();
    let key = derive(&s, n, &v, &c).0;
    assert_eq!(key.len(), 64);
    assert!(key
        .chars()
        .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()));
}
