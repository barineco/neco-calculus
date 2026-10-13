use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub fn canonical_json(value: &serde_json::Value) -> Result<Vec<u8>, String> {
    let mut output = String::new();
    write_value(value, &mut output)?;
    Ok(output.into_bytes())
}

fn write_value(value: &serde_json::Value, output: &mut String) -> Result<(), String> {
    match value {
        serde_json::Value::Null => output.push_str("null"),
        serde_json::Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        serde_json::Value::Number(value) => output.push_str(&value.to_string()),
        serde_json::Value::String(value) => {
            output.push_str(&serde_json::to_string(value).map_err(|error| error.to_string())?)
        }
        serde_json::Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_value(value, output)?;
            }
            output.push(']');
        }
        serde_json::Value::Object(values) => {
            output.push('{');
            for (index, (key, value)) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&serde_json::to_string(key).map_err(|error| error.to_string())?);
                output.push(':');
                write_value(value, output)?;
            }
            output.push('}');
        }
    }
    Ok(())
}

pub fn canonical_batch(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    canonical_batch_bytes(path, bytes)
}

pub fn canonical_batch_bytes(path: &Path, bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| format!("{}: UTF-8 ではありません: {error}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| format!("{}: JSON として復号できません: {error}", path.display()))?;
    validate_batch(&value, path)?;
    if canonical_json(&value)? != bytes {
        return Err(format!(
            "{}: canonical JSON bytes と一致しません",
            path.display()
        ));
    }
    Ok(bytes)
}

fn validate_batch(value: &serde_json::Value, path: &Path) -> Result<(), String> {
    let batch = value
        .as_object()
        .ok_or_else(|| format!("{}: batch は object でなければなりません", path.display()))?;
    if batch.len() != 2
        || batch.get("schema")
            != Some(&serde_json::Value::String(
                "neco.calculus.observation-batch.v1".to_owned(),
            ))
        || !batch.contains_key("cases")
    {
        return Err(format!(
            "{}: batch は schema と cases を持つ calculus batch でなければなりません",
            path.display()
        ));
    }
    let cases = batch["cases"]
        .as_array()
        .ok_or_else(|| format!("{}: cases は array でなければなりません", path.display()))?;
    let mut identifiers = BTreeSet::new();
    for case in cases {
        let case = case
            .as_object()
            .ok_or_else(|| format!("{}: case は object でなければなりません", path.display()))?;
        if case.len() != 4
            || !case.contains_key("caseId")
            || !case.contains_key("operation")
            || !case.contains_key("input")
            || !case.contains_key("observation")
        {
            return Err(format!(
                "{}: case は caseId, operation, input, observation だけを持ちます",
                path.display()
            ));
        }
        let case_id = case["caseId"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("{}: caseId は空でない string です", path.display()))?;
        if !identifiers.insert(case_id) {
            return Err(format!("{}: caseId が重複しています", path.display()));
        }
        if case["operation"]
            .as_str()
            .filter(|value| !value.is_empty())
            .is_none()
            || !case["input"].is_object()
            || !case["observation"].is_object()
        {
            return Err(format!("{}: case の型が不正です", path.display()));
        }
    }
    Ok(())
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn verdict_counts(cases: &serde_json::Value) -> Result<BTreeMap<String, usize>, String> {
    let cases = cases
        .as_array()
        .ok_or_else(|| "checker cases は array です".to_owned())?;
    let mut identifiers = BTreeSet::new();
    let mut counts = BTreeMap::new();
    for case in cases {
        let case = case
            .as_object()
            .ok_or_else(|| "checker case は object です".to_owned())?;
        if case.len() != 2 || !case.contains_key("caseId") || !case.contains_key("verdict") {
            return Err("checker case は caseId と verdict だけを持ちます".to_owned());
        }
        let verdict = case["verdict"]
            .as_str()
            .ok_or_else(|| "checker verdict は string です".to_owned())?;
        if !matches!(verdict, "verified" | "refuted" | "undetermined" | "invalid") {
            return Err("checker verdict が未定義です".to_owned());
        }
        let case_id = case["caseId"]
            .as_str()
            .ok_or_else(|| "checker caseId は string です".to_owned())?;
        if !identifiers.insert(case_id) {
            return Err("checker caseId が重複しています".to_owned());
        }
        *counts.entry(verdict.to_owned()).or_default() += 1;
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_batch_accepts_the_calculus_shape() {
        let bytes = br#"{"cases":[{"caseId":"p-001","input":{},"observation":{},"operation":"polynomial-exact"}],"schema":"neco.calculus.observation-batch.v1"}"#;
        assert!(canonical_batch_bytes(Path::new("batch.json"), bytes.to_vec()).is_ok());
    }
    #[test]
    fn verdict_counts_rejects_duplicate_case_ids() {
        let cases: serde_json::Value = serde_json::from_str(
            r#"[{"caseId":"a","verdict":"verified"},{"caseId":"a","verdict":"invalid"}]"#,
        )
        .unwrap();
        assert!(verdict_counts(&cases).is_err());
    }

    #[test]
    fn verdict_counts_rejects_duplicate_empty_case_ids() {
        let cases = serde_json::json!([
            {"caseId": "", "verdict": "invalid"},
            {"caseId": "", "verdict": "verified"}
        ]);
        assert!(verdict_counts(&cases).is_err());
    }
}
