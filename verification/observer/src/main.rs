use neco_calculus_observer::{canonical_batch, canonical_json, sha256, verdict_counts};
use serde_json::json;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

fn canonical_value(path: &Path) -> Result<serde_json::Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| format!("{}: UTF-8 ではありません: {error}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| format!("{}: JSON として復号できません: {error}", path.display()))?;
    if canonical_json(&value)? != text.as_bytes() {
        return Err(format!(
            "{}: canonical JSON bytes と一致しません",
            path.display()
        ));
    }
    Ok(value)
}

fn batch_case_ids(batch: &serde_json::Value) -> Result<Vec<String>, String> {
    batch["cases"]
        .as_array()
        .ok_or_else(|| "batch cases は array です".to_owned())?
        .iter()
        .map(|case| {
            case["caseId"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "batch caseId は string です".to_owned())
        })
        .collect()
}

enum CheckerResult<'a> {
    BatchInvalid,
    Cases(&'a serde_json::Value),
}

fn checker_result<'a>(
    result: &'a serde_json::Value,
    digest: &str,
) -> Result<CheckerResult<'a>, String> {
    let object = result
        .as_object()
        .ok_or_else(|| "checker result は object です".to_owned())?;
    if object.contains_key("batchVerdict") {
        if object.len() != 2
            || object.get("batchVerdict") != Some(&json!("invalid"))
            || object.get("digest") != Some(&json!(digest))
        {
            return Err("checker result は batchVerdict と digest を持ちます".to_owned());
        }
        return Ok(CheckerResult::BatchInvalid);
    }
    if object.contains_key("cases") {
        if object.len() != 2 || object.get("digest") != Some(&json!(digest)) {
            return Err("checker result は cases と digest を持ちます".to_owned());
        }
        return Ok(CheckerResult::Cases(&result["cases"]));
    }
    Err("checker result は batchVerdict または cases と digest を持ちます".to_owned())
}

fn verify(batch_path: &Path, checker_result_path: &Path) -> Result<String, String> {
    let batch_bytes = canonical_batch(batch_path)?;
    let batch: serde_json::Value = serde_json::from_slice(&batch_bytes)
        .map_err(|error| format!("{}: {error}", batch_path.display()))?;
    let batch_digest = sha256(&batch_bytes);
    let result = canonical_value(checker_result_path)?;
    match checker_result(&result, &batch_digest)? {
        CheckerResult::BatchInvalid => Err(
            "checker result の batchVerdict は observer が batch の case を列挙できた結果と矛盾します"
                .to_owned(),
        ),
        CheckerResult::Cases(cases) => {
            let counts = verdict_counts(cases)?;
            let checker_case_ids = cases
                .as_array()
                .ok_or_else(|| "checker cases は array です".to_owned())?
                .iter()
                .map(|case| {
                    case["caseId"]
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| "checker caseId は string です".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?;
            if checker_case_ids != batch_case_ids(&batch)? {
                return Err("checker caseId は batch の同数同順の caseId と一致します".to_owned());
            }
            String::from_utf8(
                canonical_json(&json!({"cases":counts,"digest":batch_digest}))
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())
        }
    }
}

fn run() -> Result<String, String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let [batch_flag, batch, result_flag, result] = arguments.as_slice() else {
        return Err("usage: --batch <canonical-observation-batch> --checker-result <canonical-checker-result>".to_owned());
    };
    if batch_flag != "--batch" || result_flag != "--checker-result" {
        return Err("usage: --batch <canonical-observation-batch> --checker-result <canonical-checker-result>".to_owned());
    }
    verify(Path::new(batch), Path::new(result))
}

fn main() -> ExitCode {
    match run() {
        Ok(output) => {
            let mut stdout = std::io::stdout().lock();
            if stdout.write_all(output.as_bytes()).is_ok() && stdout.write_all(b"\n").is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            let mut stderr = std::io::stderr().lock();
            let _ = stderr.write_all(format!("neco-calculus-observer: {error}\n").as_bytes());
            ExitCode::from(1)
        }
    }
}
