use calculus_observation_types::{validate_canonical, validate_canonical_value};

fn assert_one_case(bytes: &[u8], case_id: &str, operation: &str) {
    let batch = validate_canonical(bytes).expect("canonical vector");
    assert_eq!(batch.cases().len(), 1);
    assert_eq!(batch.cases()[0]["caseId"], case_id);
    assert_eq!(batch.cases()[0]["operation"], operation);
}

fn decoded_hex(text: &str) -> Vec<u8> {
    let text = text.trim();
    assert!(!text.is_empty());
    assert_eq!(text.len() % 2, 0);
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).expect("lowercase hex"))
        .collect()
}

#[test]
fn known_vector_is_canonical() {
    let known = include_bytes!("../../../../vectors/known/families.json");

    assert_eq!(validate_canonical(known).unwrap().cases().len(), 20);
}

#[test]
fn every_family_has_a_canonical_tampered_vector() {
    for (case_id, operation, bytes) in [
        (
            "polynomial-001",
            "polynomial-exact",
            include_bytes!("../../../../vectors/tampered/polynomial-exact.json").as_slice(),
        ),
        (
            "polynomial-enclosure-001",
            "polynomial-enclosure",
            include_bytes!("../../../../vectors/tampered/polynomial-enclosure.json").as_slice(),
        ),
        (
            "rational-001",
            "rational-exact",
            include_bytes!("../../../../vectors/tampered/rational-exact.json").as_slice(),
        ),
        (
            "rational-interval-001",
            "rational-interval",
            include_bytes!("../../../../vectors/tampered/rational-interval.json").as_slice(),
        ),
        (
            "root-bracket-001",
            "root-bracket",
            include_bytes!("../../../../vectors/tampered/root-bracket.json").as_slice(),
        ),
        (
            "bisection-001",
            "bisection",
            include_bytes!("../../../../vectors/tampered/bisection.json").as_slice(),
        ),
        (
            "even-touch-001",
            "even-touch",
            include_bytes!("../../../../vectors/tampered/even-touch.json").as_slice(),
        ),
        (
            "local-region-transition-001",
            "local-region-transition",
            include_bytes!("../../../../vectors/tampered/local-region-transition.json").as_slice(),
        ),
        (
            "surface-exact-001",
            "surface-exact",
            include_bytes!("../../../../vectors/tampered/surface-exact.json").as_slice(),
        ),
        (
            "surface-checked-001",
            "surface-checked",
            include_bytes!("../../../../vectors/tampered/surface-checked.json").as_slice(),
        ),
        (
            "rational-interval-001",
            "rational-interval",
            include_bytes!("../../../../vectors/tampered/rational-interval-reversed.json")
                .as_slice(),
        ),
        (
            "surface-checked-001",
            "surface-checked",
            include_bytes!("../../../../vectors/tampered/surface-checked-zero-precision.json")
                .as_slice(),
        ),
    ] {
        assert_one_case(bytes, case_id, operation);
    }
}

#[test]
fn domain_zero_success_claim_is_a_canonical_tampered_vector() {
    assert_one_case(
        include_bytes!("../../../../vectors/tampered/rational-interval-domain-violation.json"),
        "rational-interval-001",
        "rational-interval",
    );
}

#[test]
fn canonical_encoding_violations_are_rejected() {
    for bytes in [
        include_bytes!("../../../../vectors/invalid/canonical-bytes.json").as_slice(),
        include_bytes!("../../../../vectors/invalid/duplicate-key.json").as_slice(),
        include_bytes!("../../../../vectors/invalid/key-order.json").as_slice(),
        include_bytes!("../../../../vectors/invalid/reencode-mismatch.json").as_slice(),
        include_bytes!("../../../../vectors/invalid/whitespace.json").as_slice(),
    ] {
        assert!(validate_canonical(bytes).is_err());
    }

    let bytes = decoded_hex(include_str!("../../../../vectors/invalid/non-utf8.hex"));
    assert!(std::str::from_utf8(&bytes).is_err());
    assert!(validate_canonical(&bytes).is_err());
}

#[test]
fn semantic_invalid_vectors_are_canonical_protocol_batches() {
    for (case_id, operation, bytes) in [
        (
            "polynomial-001",
            "polynomial-exact",
            include_bytes!("../../../../vectors/invalid/nondecimal.json").as_slice(),
        ),
        (
            "polynomial-001",
            "polynomial-exact",
            include_bytes!("../../../../vectors/invalid/non-normalized-polynomial.json").as_slice(),
        ),
        (
            "polynomial-001",
            "unknown",
            include_bytes!("../../../../vectors/invalid/unknown-operation.json").as_slice(),
        ),
        (
            "surface-checked-001",
            "surface-checked",
            include_bytes!("../../../../vectors/invalid/exponent-range.json").as_slice(),
        ),
        (
            "polynomial-enclosure-001",
            "polynomial-enclosure",
            include_bytes!("../../../../vectors/invalid/dyadic-exponent-range.json").as_slice(),
        ),
        (
            "rational-interval-001",
            "rational-interval",
            include_bytes!("../../../../vectors/invalid/rational-interval-precision-range.json")
                .as_slice(),
        ),
        (
            "surface-exact-001",
            "surface-exact",
            include_bytes!("../../../../vectors/invalid/unreduced-surface-coefficient.json")
                .as_slice(),
        ),
    ] {
        assert_one_case(bytes, case_id, operation);
    }
}

#[test]
fn duplicate_case_id_is_canonical_bytes_with_an_invalid_case_identity() {
    let bytes = include_bytes!("../../../../vectors/invalid/duplicate-case-id.json");

    validate_canonical_value(bytes).expect("canonical bytes");
    assert_eq!(
        validate_canonical(bytes).unwrap_err(),
        "case identity is invalid"
    );
}
