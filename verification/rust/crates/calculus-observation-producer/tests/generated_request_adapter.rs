use std::process::Command;

use calculus_observation_producer::{describe, generate_request_batch, ADAPTER_REVISION};
use calculus_observation_types::{canonical_value_bytes, object, validate_canonical_value};
use serde_json::Value;

fn empty_request() -> Vec<u8> {
    canonical_value_bytes(&object([
        (
            "adapterRevision",
            Value::String(ADAPTER_REVISION.to_owned()),
        ),
        ("cases", Value::Array(Vec::new())),
        ("domain", Value::String("neco.calculus".to_owned())),
        (
            "generator",
            object([
                ("algorithm", Value::String("splitmix64-v1".to_owned())),
                (
                    "drawScheduleRevision",
                    Value::String("draw-schedule-v1".to_owned()),
                ),
                ("seed", Value::String("0000000000000000".to_owned())),
            ]),
        ),
        (
            "schema",
            Value::String("neco.verification.generated-request-batch.v1".to_owned()),
        ),
    ]))
    .expect("canonical request")
}

fn producer() -> &'static str {
    env!("CARGO_BIN_EXE_calculus-observation-producer")
}

#[test]
fn describe_and_generate_use_self_observed_identity() {
    let description = describe().expect("describe");
    assert_eq!(description["schema"], "neco.verification.producer-description.v1");
    assert!(description["rustcIdentity"]
        .as_str()
        .is_some_and(|text| text.starts_with("rustc ")));
    assert!(description["targetTriple"]
        .as_str()
        .is_some_and(|text| !text.is_empty()));
    assert_eq!(description["resourceClaim"]["maxBatchCases"], 2000);
    assert!(description.get("sourceOid").is_none());
    let batch = generate_request_batch(&empty_request()).expect("generate");
    assert!(batch.cases().is_empty());
}

#[test]
fn describe_emits_canonical_stdout() {
    let output = Command::new(producer())
        .arg("describe")
        .output()
        .expect("run producer");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let description = validate_canonical_value(&output.stdout).expect("canonical describe");
    assert_eq!(description["schema"], "neco.verification.producer-description.v1");
    assert!(description["rustcIdentity"]
        .as_str()
        .is_some_and(|text| text.starts_with("rustc ")));
    assert!(description["targetTriple"]
        .as_str()
        .is_some_and(|text| !text.is_empty()));
    assert_eq!(description["resourceClaim"]["maxBatchCases"], 2000);
    assert!(description.get("sourceOid").is_none());
}
