#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use sha2::{Digest, Sha256};

pub const BATCH_SCHEMA: &str = "neco.calculus.observation-batch.v1";

#[derive(Clone, Debug, PartialEq)]
pub struct ObservationBatch {
    cases: Vec<Value>,
}

impl ObservationBatch {
    pub fn new(cases: Vec<Value>) -> Self {
        Self { cases }
    }

    pub fn cases(&self) -> &[Value] {
        &self.cases
    }

    pub fn into_cases(self) -> Vec<Value> {
        self.cases
    }
}

pub fn object(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .collect(),
    )
}

pub fn decimal(value: impl ToString) -> Value {
    Value::String(value.to_string())
}

pub fn observation(
    case_id: String,
    operation: &'static str,
    input: Value,
    observation: Value,
) -> Value {
    object([
        ("caseId", Value::String(case_id)),
        ("input", input),
        ("observation", observation),
        ("operation", Value::String(operation.to_owned())),
    ])
}

pub fn success(value: Value) -> Value {
    object([
        ("kind", Value::String("success".to_owned())),
        ("value", value),
    ])
}

pub fn failure(kind: &'static str) -> Value {
    object([
        ("failure", Value::String(kind.to_owned())),
        ("kind", Value::String("failure".to_owned())),
    ])
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObservationOutcome {
    Success(Value),
    Failure(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalObservation {
    case_id: String,
    operation: &'static str,
    input: Value,
    outcome: ObservationOutcome,
}

impl CanonicalObservation {
    pub fn new(
        case_id: String,
        operation: &'static str,
        input: Value,
        outcome: ObservationOutcome,
    ) -> Self {
        Self {
            case_id,
            operation,
            input,
            outcome,
        }
    }

    pub fn into_value(self) -> Value {
        let outcome = match self.outcome {
            ObservationOutcome::Success(value) => success(value),
            ObservationOutcome::Failure(kind) => object([
                ("failure", Value::String(kind)),
                ("kind", Value::String("failure".to_owned())),
            ]),
        };
        observation(self.case_id, self.operation, self.input, outcome)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalEncodingFailure {
    violated_condition: String,
    witness: String,
}

impl CanonicalEncodingFailure {
    fn from_serde(error: serde_json::Error) -> Self {
        Self {
            violated_condition: "value must be representable as canonical JSON".to_owned(),
            witness: error.to_string(),
        }
    }

    pub fn violated_condition(&self) -> &str {
        &self.violated_condition
    }

    pub fn witness(&self) -> &str {
        &self.witness
    }
}

impl core::fmt::Display for CanonicalEncodingFailure {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{}: {}", self.violated_condition, self.witness)
    }
}

impl std::error::Error for CanonicalEncodingFailure {}

pub fn canonical_bytes(batch: &ObservationBatch) -> Result<Vec<u8>, CanonicalEncodingFailure> {
    canonical_value_bytes(&object([
        ("cases", Value::Array(batch.cases.clone())),
        ("schema", Value::String(BATCH_SCHEMA.to_owned())),
    ]))
}

pub fn canonical_value_bytes(value: &Value) -> Result<Vec<u8>, CanonicalEncodingFailure> {
    serde_json::to_vec(value).map_err(CanonicalEncodingFailure::from_serde)
}

pub fn digest_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn validate_canonical_value(bytes: &[u8]) -> Result<Value, String> {
    reject_duplicate_keys(bytes)?;
    let value: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let encoded = canonical_value_bytes(&value).map_err(|failure| failure.to_string())?;
    if encoded != bytes {
        return Err("JSON is not canonical".to_owned());
    }
    Ok(value)
}

pub fn validate_canonical(bytes: &[u8]) -> Result<ObservationBatch, String> {
    let value = validate_canonical_value(bytes)?;
    let object = value
        .as_object()
        .ok_or_else(|| "batch must be an object".to_owned())?;
    if object.len() != 2 || object.get("schema") != Some(&Value::String(BATCH_SCHEMA.to_owned())) {
        return Err("unexpected schema".to_owned());
    }
    let cases = object
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "cases must be an array".to_owned())?;
    let mut case_ids = BTreeSet::new();
    for case in cases {
        validate_case(case)?;
        let case_id = case["caseId"]
            .as_str()
            .ok_or_else(|| "case identity is invalid".to_owned())?;
        if case_id.is_empty() || !case_ids.insert(case_id) {
            return Err("case identity is invalid".to_owned());
        }
    }
    let encoded = canonical_bytes(&ObservationBatch::new(cases.to_vec()))
        .map_err(|failure| failure.to_string())?;
    if encoded != bytes {
        return Err("batch is not canonical".to_owned());
    }
    Ok(ObservationBatch::new(cases.to_vec()))
}

fn validate_case(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "case must be an object".to_owned())?;
    if object.len() != 4
        || !["caseId", "operation", "input", "observation"]
            .iter()
            .all(|key| object.contains_key(*key))
    {
        return Err("case fields are invalid".to_owned());
    }
    if object.get("caseId").and_then(Value::as_str).is_none()
        || object.get("operation").and_then(Value::as_str).is_none()
    {
        return Err("case identity is invalid".to_owned());
    }
    Ok(())
}

fn reject_duplicate_keys(bytes: &[u8]) -> Result<(), String> {
    let mut parser = DuplicateKeyParser { bytes, offset: 0 };
    parser.value()?;
    parser.space();
    if parser.offset == bytes.len() {
        Ok(())
    } else {
        Err("trailing JSON data".to_owned())
    }
}

struct DuplicateKeyParser<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl DuplicateKeyParser<'_> {
    fn space(&mut self) {
        while self
            .bytes
            .get(self.offset)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.offset += 1;
        }
    }
    fn take(&mut self, byte: u8) -> Result<(), String> {
        self.space();
        if self.bytes.get(self.offset) == Some(&byte) {
            self.offset += 1;
            Ok(())
        } else {
            Err("invalid JSON".to_owned())
        }
    }
    fn value(&mut self) -> Result<(), String> {
        self.space();
        match self.bytes.get(self.offset) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'\"') => {
                self.string()?;
                Ok(())
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b't') => self.word(b"true"),
            Some(b'f') => self.word(b"false"),
            Some(b'n') => self.word(b"null"),
            _ => Err("invalid JSON".to_owned()),
        }
    }
    fn object(&mut self) -> Result<(), String> {
        self.take(b'{')?;
        self.space();
        let mut keys = BTreeSet::new();
        if self.bytes.get(self.offset) == Some(&b'}') {
            self.offset += 1;
            return Ok(());
        }
        loop {
            let key = self.string()?;
            if !keys.insert(key) {
                return Err("duplicate object key".to_owned());
            }
            self.take(b':')?;
            self.value()?;
            self.space();
            if self.bytes.get(self.offset) == Some(&b'}') {
                self.offset += 1;
                return Ok(());
            }
            self.take(b',')?;
        }
    }
    fn array(&mut self) -> Result<(), String> {
        self.take(b'[')?;
        self.space();
        if self.bytes.get(self.offset) == Some(&b']') {
            self.offset += 1;
            return Ok(());
        }
        loop {
            self.value()?;
            self.space();
            if self.bytes.get(self.offset) == Some(&b']') {
                self.offset += 1;
                return Ok(());
            }
            self.take(b',')?;
        }
    }
    fn string(&mut self) -> Result<String, String> {
        let start = self.offset;
        self.take(b'\"')?;
        let mut escaped = false;
        while let Some(byte) = self.bytes.get(self.offset) {
            self.offset += 1;
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'\"' {
                return serde_json::from_slice(&self.bytes[start..self.offset])
                    .map_err(|error| error.to_string());
            } else if *byte < 0x20 {
                return Err("invalid JSON string".to_owned());
            }
        }
        Err("unterminated JSON string".to_owned())
    }
    fn number(&mut self) -> Result<(), String> {
        let start = self.offset;
        while self.bytes.get(self.offset).is_some_and(|byte| {
            byte.is_ascii_digit() || matches!(*byte, b'-' | b'+' | b'.' | b'e' | b'E')
        }) {
            self.offset += 1;
        }
        serde_json::from_slice::<Value>(&self.bytes[start..self.offset])
            .map(|_| ())
            .map_err(|_| "invalid JSON number".to_owned())
    }
    fn word(&mut self, word: &[u8]) -> Result<(), String> {
        if self.bytes.get(self.offset..self.offset + word.len()) == Some(word) {
            self.offset += word.len();
            Ok(())
        } else {
            Err("invalid JSON token".to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_json_and_digest() {
        let cases = vec![observation(
            "case-001".to_owned(),
            "polynomial-exact",
            object([]),
            success(object([])),
        )];
        let bytes = canonical_bytes(&ObservationBatch::new(cases)).expect("canonical test batch");
        assert!(validate_canonical(&bytes).is_ok());
        assert_eq!(digest_hex(&bytes).len(), 64);
        assert!(validate_canonical(br#"{"cases":[{"caseId":"x","caseId":"y"}],"schema":"neco.calculus.observation-batch.v1"}"#).is_err());
    }
}
