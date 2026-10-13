use neco_calculus_observer::{
    canonical_batch, canonical_batch_bytes, canonical_json, sha256, verdict_counts,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const GENERATED_REQUEST_SCHEMA: &str = "neco.verification.generated-request-batch.v1";
const PRODUCER_DESCRIPTION_SCHEMA: &str = "neco.verification.producer-description.v1";
const PUBLIC_API_PARITY_SCHEMA: &str = "neco.verification.public-api-parity-result.v1";
const ADAPTER_FAILURE_SCHEMA: &str = "neco.verification.adapter-failure.v1";
const ADAPTER_REVISION: &str = "calculus-adapter-v3";

fn worktree_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("observer manifest is inside verification/observer")
        .to_path_buf()
}

fn cargo_program() -> OsString {
    env::var_os("CARGO")
        .filter(|program| !program.is_empty())
        .unwrap_or_else(|| OsString::from("cargo"))
}

fn remove_empty_cargo_target_dir(command: &mut Command) {
    if env::var_os("CARGO_TARGET_DIR").is_some_and(|directory| directory.is_empty()) {
        command.env_remove("CARGO_TARGET_DIR");
    }
}

fn prepare_child_process(command: &mut Command) {
    remove_empty_cargo_target_dir(command);
}

fn run(command: &mut Command, subject: &str) -> Output {
    prepare_child_process(command);
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("{subject} を起動できません: {error}"));
    assert!(
        output.status.success(),
        "{subject} が {} で終了しました\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn run_with_stdin(command: &mut Command, input: &[u8], subject: &str) -> Output {
    prepare_child_process(command);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("{subject} を起動できません: {error}"));
    child
        .stdin
        .take()
        .expect("stdin pipe を開く")
        .write_all(input)
        .unwrap_or_else(|error| panic!("{subject} stdin を書き込めません: {error}"));
    let output = child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("{subject} の終了を待機できません: {error}"));
    assert!(
        output.status.success(),
        "{subject} が {} で終了しました\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn staged_lean_input(root: &Path) -> PathBuf {
    let executable = root.join("verification/lean/.lake/build/bin").join(format!(
        "neco-calculus-checker{}",
        env::consts::EXE_SUFFIX
    ));
    assert!(
        executable.is_file(),
        "個別検査済み Lean checker executable がありません: {}",
        executable.display()
    );
    let mut tests = Command::new(&executable);
    tests.arg("all");
    run(&mut tests, "Lean checker individual test");
    executable
}

struct DescriptorExpectations {
    semantics_digest: String,
    projection_digest: String,
}

impl DescriptorExpectations {
    fn canonical(root: &Path, projection: &Value) -> Self {
        let semantics_bytes = fs::read(root.join("verification/manifest/calculus-semantics.json"))
            .expect("calculus semantics declaration を読む");
        let families_bytes = canonical_json(&projection["families"])
            .expect("projection families を canonicalize する");
        Self {
            semantics_digest: sha256(&semantics_bytes),
            projection_digest: sha256(&families_bytes),
        }
    }
}

fn build_producer(root: &Path) -> PathBuf {
    let manifest = root.join("verification/rust/Cargo.toml");
    let mut build = Command::new(cargo_program());
    build.current_dir(root);
    build.args([
        "build",
        "--manifest-path",
        manifest.to_str().expect("Rust manifest path is UTF-8"),
        "--locked",
        "-p",
        "calculus-observation-producer",
        "--bin",
        "calculus-observation-producer",
        "--message-format=json-render-diagnostics",
    ]);
    let output = run(&mut build, "Rust producer build");
    let executable = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .find_map(|message| {
            (message["reason"] == "compiler-artifact"
                && message["target"]["name"] == "calculus-observation-producer"
                && message["target"]["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin")))
            .then(|| message["executable"].as_str().map(PathBuf::from))
            .flatten()
        })
        .expect("Rust producer build は executable 成果物を返します");
    assert!(
        executable.is_file(),
        "Rust producer executable がありません: {}",
        executable.display()
    );
    executable
}

fn producer_failure(producer: &Path, arguments: &[&OsStr]) -> Value {
    let mut command = Command::new(producer);
    command.args(arguments);
    prepare_child_process(&mut command);
    let output = command
        .output()
        .expect("producer failure process を起動する");
    assert_eq!(
        output.status.code(),
        Some(2),
        "producer failure は exit 2 を返します\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(output.stdout.is_empty(), "producer failure stdout は空です");
    let failure = canonical_value(&output.stderr, "producer failure stderr");
    assert_eq!(failure["schema"], ADAPTER_FAILURE_SCHEMA);
    failure
}

struct StagedArtifacts {
    checker: PathBuf,
    rust_directory: PathBuf,
}

fn stage_artifacts(root: &Path, lean: &Path) -> StagedArtifacts {
    let artifacts = root.join("verification/artifacts");
    let lean_directory = artifacts.join("lean");
    let rust_directory = artifacts.join("rust");
    fs::create_dir_all(&lean_directory).expect("Lean artifact directory を作成する");
    fs::create_dir_all(&rust_directory).expect("Rust artifact directory を作成する");

    let checker = lean_directory.join(format!(
        "neco-calculus-checker{}",
        env::consts::EXE_SUFFIX
    ));
    fs::copy(lean, &checker).expect("Lean checker executable を stage する");
    assert!(
        checker.is_file(),
        "staged Lean checker executable がありません: {}",
        checker.display()
    );

    StagedArtifacts {
        checker,
        rust_directory,
    }
}

struct ObserverResult {
    digest: String,
    verdict_counts: BTreeMap<String, usize>,
}

struct CheckerResult {
    digest: String,
    verdict_counts: BTreeMap<String, usize>,
    case_ids: Vec<String>,
    value: Value,
}

fn checker_rejects_missing_batch(checker: &Path, directory: &Path) {
    let missing_batch = directory.join("calculus-checker-input-absent.json");
    assert!(
        !missing_batch.exists(),
        "checker process error 用の入力パスは存在しません"
    );
    let mut command = Command::new(checker);
    command.args([OsStr::new("check"), missing_batch.as_os_str()]);
    prepare_child_process(&mut command);
    let output = command
        .output()
        .expect("missing batch の Lean checker process を起動する");
    assert!(
        !output.status.success(),
        "存在しない batch に対する Lean checker は process error を返します"
    );
    assert!(
        output.stdout.is_empty(),
        "checker process error stdout は空です"
    );
}

fn checker_result(checker: &Path, batch: &Path) -> CheckerResult {
    let mut command = Command::new(checker);
    command.args([OsStr::new("check"), batch.as_os_str()]);
    let output = run(&mut command, "staged Lean checker");
    let result = canonical_value(&output.stdout, "Lean checker stdout");
    let object = result.as_object().expect("Lean checker output は object");
    let digest = object["digest"]
        .as_str()
        .expect("Lean checker digest は string")
        .to_owned();
    assert_eq!(
        object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from(["cases", "digest"]),
        "Lean checker case output は cases と digest を持つ"
    );
    let cases = object["cases"]
        .as_array()
        .expect("Lean checker cases は array");
    let verdict_counts =
        verdict_counts(&object["cases"]).expect("Lean checker cases は一意の既定 verdict を持つ");
    let case_ids = cases
        .iter()
        .map(|case| {
            case["caseId"]
                .as_str()
                .expect("Lean checker caseId は string")
                .to_owned()
        })
        .collect();
    CheckerResult {
        digest,
        verdict_counts,
        case_ids,
        value: result,
    }
}

fn stage_checker_result(directory: &Path, result: &CheckerResult) -> PathBuf {
    let path = directory.join("calculus-generated-checker-result.json");
    let bytes = canonical_json(&result.value).expect("Lean checker result を canonicalize する");
    fs::write(&path, bytes).expect("Lean checker result を stage する");
    path
}

fn observer() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_neco-calculus-observer"))
}

fn run_observer(root: &Path, batch: &Path, checker_result: &Path) -> ObserverResult {
    let observer_directory = root.join("verification/observer");
    let mut command = Command::new(observer());
    command.current_dir(observer_directory).args([
        OsStr::new("--batch"),
        batch.as_os_str(),
        OsStr::new("--checker-result"),
        checker_result.as_os_str(),
    ]);
    let output = run(&mut command, "observer stack runner");
    observer_result(&output.stdout)
}

fn observer_rejects_checker_result(root: &Path, batch: &Path, checker_result: &Path) {
    let observer_directory = root.join("verification/observer");
    let mut command = Command::new(observer());
    command.current_dir(observer_directory).args([
        OsStr::new("--batch"),
        batch.as_os_str(),
        OsStr::new("--checker-result"),
        checker_result.as_os_str(),
    ]);
    prepare_child_process(&mut command);
    let output = command
        .output()
        .expect("observer failure process を起動する");
    assert!(
        !output.status.success(),
        "observer は改変 checker result を受理しません\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(output.stdout.is_empty(), "observer failure stdout は空です");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("checker result"),
        "observer failure stderr は checker result を示します"
    );
}

fn observer_result(bytes: &[u8]) -> ObserverResult {
    let output = canonical_value(bytes, "observer stdout");
    let object = output.as_object().expect("observer output は object");
    assert_eq!(
        object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from(["cases", "digest"]),
        "observer output は cases と digest を持つ"
    );
    let digest = object["digest"]
        .as_str()
        .expect("observer digest は string")
        .to_owned();
    let verdict_counts = object["cases"]
        .as_object()
        .expect("observer cases は object")
        .iter()
        .map(|(verdict, count)| {
            (
                verdict.clone(),
                usize::try_from(count.as_u64().expect("verdict count は整数"))
                    .expect("verdict count は usize に収まる"),
            )
        })
        .collect();
    ObserverResult {
        digest,
        verdict_counts,
    }
}

fn canonical_value(bytes: &[u8], subject: &str) -> Value {
    let text =
        std::str::from_utf8(bytes).unwrap_or_else(|error| panic!("{subject} は UTF-8: {error}"));
    let text = text.strip_suffix('\n').unwrap_or(text);
    let value: Value =
        serde_json::from_str(text).unwrap_or_else(|error| panic!("{subject} は JSON: {error}"));
    assert_eq!(
        canonical_json(&value).unwrap_or_else(|error| panic!("{subject} を canonicalize: {error}")),
        text.as_bytes(),
        "{subject} は canonical JSON"
    );
    value
}

fn assert_verified(counts: &BTreeMap<String, usize>, subject: &str) {
    assert!(
        counts.get("verified").is_some_and(|count| *count > 0) && counts.len() == 1,
        "{subject} は verified case だけを返します: {counts:?}"
    );
}

fn projection_declaration(root: &Path) -> Value {
    let bytes = fs::read(root.join("verification/manifest/calculus-projection-declaration.json"))
        .expect("projection declaration を読む");
    canonical_value(&bytes, "projection declaration")
}

struct ResourceDeclaration {
    value: Value,
    digest: String,
}

fn resource_declaration(root: &Path) -> ResourceDeclaration {
    let bytes = fs::read(root.join("verification/manifest/calculus-resource-declaration.json"))
        .expect("resource declaration を読む");
    let value = canonical_value(&bytes, "resource declaration");
    ResourceDeclaration {
        digest: sha256(&bytes),
        value,
    }
}

fn canonical_generated_request(root: &Path) -> (Value, Vec<u8>) {
    let request_path = root.join("verification/vectors/requests/canonical-request-batch.json");
    let bytes = fs::read(&request_path).unwrap_or_else(|error| {
        panic!(
            "canonical generated request を読めません: {}: {error}",
            request_path.display()
        )
    });
    let request = canonical_value(&bytes, "canonical generated request");
    (request, bytes)
}

fn assert_generated_request_coverage(
    request: &Value,
    projection: &Value,
    resource: &ResourceDeclaration,
) -> Vec<String> {
    assert_eq!(request["schema"], GENERATED_REQUEST_SCHEMA);
    assert_eq!(request["domain"], "neco.calculus");
    assert_eq!(request["adapterRevision"], ADAPTER_REVISION);

    let families = projection["families"]
        .as_array()
        .expect("projection declaration families は array");
    assert_eq!(
        families.len(),
        10,
        "projection declaration は十 family を持つ"
    );
    let expected_kinds = families
        .iter()
        .map(|family| {
            let family_name = family["family"]
                .as_str()
                .expect("projection family name は string");
            let requested_kinds = family["requestedObservationKinds"]
                .as_array()
                .expect("requested observation kinds は array");
            assert_eq!(
                requested_kinds.len(),
                2,
                "{family_name} は success と typed failure を持つ"
            );
            assert_eq!(requested_kinds[0], "success");
            assert_eq!(requested_kinds[1], family["typedFailure"]);
            (
                family_name.to_owned(),
                requested_kinds
                    .iter()
                    .map(|kind| {
                        kind.as_str()
                            .expect("requested observation kind は string")
                            .to_owned()
                    })
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let expected_selectors = BTreeSet::from([
        "familyParameter",
        "formulaVariant",
        "intervalWidth",
        "multiplicity",
        "mutation",
        "observationClass",
        "reserved",
        "sign",
        "value",
    ]);
    let cases = request["cases"]
        .as_array()
        .expect("canonical generated request cases は array");
    assert!(
        cases.len() as u64
            <= resource.value["maxBatchCases"]
                .as_u64()
                .expect("resource declaration maxBatchCases は整数"),
        "canonical generated request は producer resource limit を超えません"
    );
    assert!(
        cases.len() >= expected_kinds.len() * 2,
        "canonical generated request は family ごとの success と typed failure を持ちます"
    );

    let mut actual_kinds = BTreeMap::<String, BTreeSet<String>>::new();
    let mut case_ids = BTreeSet::new();
    let ordered_case_ids = cases
        .iter()
        .map(|case| {
            let family = case["family"]
                .as_str()
                .expect("canonical request family は string");
            let requested_kind = case["requestedObservationKind"]
                .as_str()
                .expect("canonical request requestedObservationKind は string");
            assert!(
                expected_kinds.contains_key(family),
                "canonical generated request family は projection declaration にあります"
            );
            let selectors = case["selectors"]
                .as_object()
                .expect("canonical request selectors は object");
            assert_eq!(
                selectors
                    .keys()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>(),
                expected_selectors,
                "canonical generated request は九 selector を持ちます"
            );
            let case_id = case["caseId"]
                .as_str()
                .expect("canonical request caseId は string")
                .to_owned();
            assert!(
                case_ids.insert(case_id.clone()),
                "canonical generated request caseId は一意です"
            );
            actual_kinds
                .entry(family.to_owned())
                .or_default()
                .insert(requested_kind.to_owned());
            case_id
        })
        .collect::<Vec<_>>();
    assert_eq!(actual_kinds, expected_kinds);
    ordered_case_ids
}

fn batch_case_ids(bytes: &[u8]) -> Vec<String> {
    let batch = canonical_value(bytes, "generated batch");
    batch["cases"]
        .as_array()
        .expect("generated batch cases は array")
        .iter()
        .map(|case| {
            case["caseId"]
                .as_str()
                .expect("generated batch caseId は string")
                .to_owned()
        })
        .collect()
}

fn stage_semantic_invalid_batch(root: &Path, directory: &Path) -> PathBuf {
    let source = root.join("verification/vectors/invalid/unknown-operation.json");
    let batch = directory.join("calculus-semantic-invalid-observation-batch.json");
    fs::copy(&source, &batch).expect("semantic-invalid batch を stage する");
    canonical_batch(&batch).expect("semantic-invalid batch は canonical calculus batch");
    batch
}

fn assert_bound_description(
    producer: &Path,
    expectations: &DescriptorExpectations,
    projection: &Value,
    resource: &ResourceDeclaration,
) {
    let mut command = Command::new(producer);
    command.arg("describe");
    let description = canonical_value(
        &run(&mut command, "bound producer describe").stdout,
        "producer describe stdout",
    );
    assert_eq!(description["schema"], PRODUCER_DESCRIPTION_SCHEMA);
    assert_eq!(description["adapterRevision"], ADAPTER_REVISION);
    assert_eq!(description["domain"], "neco.calculus");
    assert!(description["rustcIdentity"]
        .as_str()
        .is_some_and(|text| text.starts_with("rustc ")));
    assert!(description["targetTriple"]
        .as_str()
        .is_some_and(|text| !text.is_empty()));
    assert!(description.get("sourceOid").is_none());
    assert_eq!(
        description["semanticsRef"]["declarationDigest"],
        expectations.semantics_digest
    );
    assert_eq!(
        description["projectionClaim"]["projectionRevision"],
        expectations.projection_digest
    );
    assert_eq!(
        description["projectionClaim"]["projectionDeclarationDigest"],
        expectations.projection_digest
    );
    assert_eq!(
        description["projectionClaim"]["projectionDeclarations"],
        projection["families"]
    );
    assert_eq!(
        description["resourceClaim"]["maxBatchCases"],
        resource.value["maxBatchCases"]
    );
    assert_eq!(description["resourceClaim"]["maxBatchCases"], 2000);
    assert_eq!(
        description["resourceClaim"]["resourceRevision"],
        resource.value["resourceRevision"]
    );
    assert_eq!(
        description["resourceClaim"]["resourceDeclarationDigest"],
        resource.digest
    );

    let declared_families = description["projectionClaim"]["projectionDeclarations"]
        .as_array()
        .expect("projection claim declarations は array");
    assert_eq!(declared_families.len(), 10);
    for family in declared_families {
        assert!(family["family"]
            .as_str()
            .is_some_and(|name| !name.is_empty()));
        assert_eq!(family["requestedObservationKinds"][0], "success");
        assert_eq!(
            family["requestedObservationKinds"][1],
            family["typedFailure"]
        );
        for binding in family["selectorBindings"]
            .as_array()
            .expect("projection selector bindings は array")
        {
            let buckets = binding["buckets"]
                .as_array()
                .expect("projection buckets は array");
            assert!(!buckets.is_empty(), "projection binding は bucket を持つ");
            let rule = &binding["selectorRule"];
            assert_eq!(rule["modulus"].as_u64(), Some(buckets.len() as u64));
            assert_eq!(rule["selector"], binding["selector"]);
            assert_eq!(
                rule["bucketsByRemainder"]
                    .as_object()
                    .expect("selector remainder map")
                    .len(),
                buckets.len()
            );
            for bucket in buckets {
                assert!(bucket["name"].as_str().is_some_and(|name| !name.is_empty()));
                assert!(bucket["projectedCondition"]
                    .as_str()
                    .is_some_and(|condition| !condition.is_empty()));
            }
        }
    }
}

fn assert_batch_resource_limit_rejection(
    producer: &Path,
    resource: &ResourceDeclaration,
    directory: &Path,
) {
    let max_batch_cases = resource.value["maxBatchCases"]
        .as_u64()
        .expect("resource declaration maxBatchCases は整数");
    let received_case_count = max_batch_cases
        .checked_add(1)
        .expect("resource declaration maxBatchCases に一件を加算できる");
    let case_count = usize::try_from(received_case_count)
        .expect("resource declaration maxBatchCases は usize に収まる");
    let request = serde_json::json!({
        "adapterRevision": ADAPTER_REVISION,
        "cases": vec![Value::Null; case_count],
        "domain": "neco.calculus",
        "generator": {
            "algorithm": "splitmix64-v1",
            "drawScheduleRevision": "draw-schedule-v1",
            "seed": "0000000000000000",
        },
        "schema": GENERATED_REQUEST_SCHEMA,
    });
    let request_path = directory.join("calculus-generated-request-over-batch-limit.json");
    fs::write(
        &request_path,
        canonical_json(&request).expect("over-limit generated request を canonicalize する"),
    )
    .expect("over-limit generated request を stage する");
    let failure = producer_failure(
        producer,
        &[
            OsStr::new("generate"),
            OsStr::new("--request"),
            request_path.as_os_str(),
        ],
    );
    assert_eq!(failure["kind"], "batchResourceLimit");
    assert_eq!(failure["failingOperation"], "decodeRequest");
    assert_eq!(
        failure["operandProvenance"]["resourceRevision"],
        resource.value["resourceRevision"]
    );
    assert_eq!(
        failure["witness"]["maxBatchCases"],
        resource.value["maxBatchCases"]
    );
    assert_eq!(
        failure["witness"]["receivedCaseCount"].as_u64(),
        Some(received_case_count)
    );
}

fn generate_batch_from_file(producer: &Path, request: &Path) -> Vec<u8> {
    let mut command = Command::new(producer);
    command.args([
        OsStr::new("generate"),
        OsStr::new("--request"),
        request.as_os_str(),
    ]);
    run(&mut command, "producer generate file").stdout
}

fn generate_batch_from_stdin(producer: &Path, request: &[u8]) -> Vec<u8> {
    let mut command = Command::new(producer);
    command.args(["generate", "--request", "-"]);
    run_with_stdin(&mut command, request, "producer generate stdin").stdout
}

fn self_check(producer: &Path, request: &Path, batch: &Path) -> Value {
    let mut command = Command::new(producer);
    command.args([
        OsStr::new("self-check"),
        OsStr::new("--request"),
        request.as_os_str(),
        OsStr::new("--batch"),
        batch.as_os_str(),
    ]);
    canonical_value(
        &run(&mut command, "producer self-check").stdout,
        "producer self-check stdout",
    )
}

#[test]
fn stack_runner_stages_real_components_and_validates_vector_sets() {
    let root = worktree_root();
    let lean = staged_lean_input(&root);
    let producer = build_producer(&root);
    let command_failure = producer_failure(&producer, &[OsStr::new("unsupported-command")]);
    assert_eq!(command_failure["kind"], "invalidRequest");
    assert_eq!(command_failure["failingOperation"], "command");

    let projection = projection_declaration(&root);
    let expectations = DescriptorExpectations::canonical(&root, &projection);
    let resource = resource_declaration(&root);
    assert_bound_description(&producer, &expectations, &projection, &resource);
    let (request, request_bytes) = canonical_generated_request(&root);
    let expected_case_ids = assert_generated_request_coverage(&request, &projection, &resource);

    let staged = stage_artifacts(&root, &lean);
    assert_batch_resource_limit_rejection(&producer, &resource, &staged.rust_directory);
    checker_rejects_missing_batch(&staged.checker, &staged.rust_directory);
    let request_path = staged
        .rust_directory
        .join("calculus-generated-request.json");
    fs::write(&request_path, &request_bytes).expect("generated request を stage する");
    let generated_from_file = generate_batch_from_file(&producer, &request_path);
    let generated_from_stdin = generate_batch_from_stdin(&producer, &request_bytes);
    assert_eq!(
        generated_from_file, generated_from_stdin,
        "file と stdin の generated batch は一致します"
    );
    canonical_batch_bytes(&request_path, generated_from_file.clone())
        .expect("generated batch は canonical calculus batch");
    assert_eq!(batch_case_ids(&generated_from_file), expected_case_ids);

    let generated_batch = staged
        .rust_directory
        .join("calculus-generated-observation-batch.json");
    fs::write(&generated_batch, &generated_from_file).expect("generated batch を stage する");
    let parity = self_check(&producer, &request_path, &generated_batch);
    assert_eq!(parity["schema"], PUBLIC_API_PARITY_SCHEMA);
    assert_eq!(parity["result"], "matched");
    assert_eq!(
        parity["caseCount"].as_u64(),
        Some(expected_case_ids.len() as u64)
    );
    assert_eq!(parity["requestDigest"], sha256(&request_bytes));
    assert_eq!(parity["observationDigest"], sha256(&generated_from_file));
    assert_eq!(
        parity["projectionRevision"],
        expectations.projection_digest
    );
    assert_eq!(
        parity["semanticsRef"]["declarationDigest"],
        expectations.semantics_digest
    );

    let expected_generated_digest =
        sha256(&canonical_batch(&generated_batch).expect("staged generated batch は canonical"));
    let generated_checker = checker_result(&staged.checker, &generated_batch);
    assert_eq!(generated_checker.digest, expected_generated_digest);
    assert_eq!(generated_checker.case_ids, expected_case_ids);
    assert_eq!(
        generated_checker.verdict_counts,
        BTreeMap::from([("verified".to_owned(), expected_case_ids.len())])
    );
    assert_verified(
        &generated_checker.verdict_counts,
        "generated producer batch",
    );

    let staged_checker_result = stage_checker_result(&staged.rust_directory, &generated_checker);
    let observer = run_observer(&root, &generated_batch, &staged_checker_result);
    assert_eq!(observer.digest, expected_generated_digest);
    assert_eq!(observer.verdict_counts, generated_checker.verdict_counts);
    assert_verified(&observer.verdict_counts, "generated observer result");

    let mut altered_checker_result = generated_checker.value.clone();
    altered_checker_result["digest"] = Value::String("0".repeat(64));
    let altered_checker_result_path = staged
        .rust_directory
        .join("calculus-generated-checker-result-altered-digest.json");
    fs::write(
        &altered_checker_result_path,
        canonical_json(&altered_checker_result)
            .expect("altered checker result を canonicalize する"),
    )
    .expect("altered checker result を stage する");
    observer_rejects_checker_result(&root, &generated_batch, &altered_checker_result_path);

    let batch_invalid_checker_result = serde_json::json!({
        "batchVerdict": "invalid",
        "digest": expected_generated_digest.clone(),
    });
    let batch_invalid_checker_result_path = staged
        .rust_directory
        .join("calculus-generated-checker-result-batch-invalid.json");
    fs::write(
        &batch_invalid_checker_result_path,
        canonical_json(&batch_invalid_checker_result)
            .expect("batch-invalid checker result を canonicalize する"),
    )
    .expect("batch-invalid checker result を stage する");
    observer_rejects_checker_result(&root, &generated_batch, &batch_invalid_checker_result_path);

    let semantic_invalid_batch = stage_semantic_invalid_batch(&root, &staged.rust_directory);
    let expected_semantic_invalid_digest = sha256(
        &canonical_batch(&semantic_invalid_batch)
            .expect("staged semantic-invalid batch は canonical"),
    );
    let semantic_invalid_checker = checker_result(&staged.checker, &semantic_invalid_batch);
    assert_eq!(
        semantic_invalid_checker.digest,
        expected_semantic_invalid_digest
    );
    assert_eq!(
        semantic_invalid_checker.case_ids,
        vec!["polynomial-001".to_owned()]
    );
    assert_eq!(
        semantic_invalid_checker.verdict_counts,
        BTreeMap::from([("invalid".to_owned(), 1)])
    );
    let semantic_invalid_checker_result =
        stage_checker_result(&staged.rust_directory, &semantic_invalid_checker);
    let semantic_invalid_observer = run_observer(
        &root,
        &semantic_invalid_batch,
        &semantic_invalid_checker_result,
    );
    assert_eq!(
        semantic_invalid_observer.digest,
        expected_semantic_invalid_digest
    );
    assert_eq!(
        semantic_invalid_observer.verdict_counts,
        semantic_invalid_checker.verdict_counts
    );
}

#[test]
fn manifests_separate_inputs_outputs_and_component_responsibilities() {
    let root = worktree_root();
    let manifest_directory = root.join("verification/manifest");
    let expected_keys = BTreeSet::from([
        "repositoryInputs",
        "toolchainInputs",
        "runtimeInputs",
        "repositoryOutputs",
        "forbidden",
    ]);
    let manifests = [
        ("lean-inputs.json", "Lean checker"),
        ("rust-inputs.json", "Rust producer"),
        ("observer-inputs.json", "observer"),
    ];
    for (file_name, component) in manifests {
        let manifest_path = manifest_directory.join(file_name);
        let manifest_bytes = fs::read(&manifest_path).expect("input manifest を読む");
        let manifest: Value =
            serde_json::from_slice(&manifest_bytes).expect("input manifest は JSON");
        assert_eq!(
            canonical_json(&manifest).expect("input manifest を canonicalize する"),
            manifest_bytes,
            "{component} manifest は canonical JSON"
        );
        let object = manifest.as_object().expect("manifest は object");
        assert_eq!(
            object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            expected_keys
        );
        for key in &expected_keys {
            assert!(
                manifest[*key].is_array(),
                "{component} の {key} は array です"
            );
            assert!(
                !manifest[*key]
                    .as_array()
                    .expect("manifest category は array")
                    .is_empty(),
                "{component} の {key} は空ではありません"
            );
        }
        let inputs = manifest["repositoryInputs"]
            .as_array()
            .expect("repositoryInputs は array");
        let outputs = manifest["repositoryOutputs"]
            .as_array()
            .expect("repositoryOutputs は array");
        let input_text = serde_json::to_string(inputs).expect("repositoryInputs を読む");
        let output_text = serde_json::to_string(outputs).expect("repositoryOutputs を読む");
        assert!(
            !input_text.contains("digest"),
            "digest は論理入力として列挙しません"
        );
        assert!(
            output_text.contains("digest"),
            "digest は repositoryOutputs に列挙します"
        );
    }

    let lean = fs::read_to_string(manifest_directory.join("lean-inputs.json"))
        .expect("Lean manifest を読む");
    assert!(lean.contains("NecoAlgebraVerify/Canonical.lean"));
    assert!(lean.contains("NecoAlgebraVerify/Semantics.lean"));
    assert!(lean.contains("calculus-side duplicate canonical decoder"));

    let rust = fs::read_to_string(manifest_directory.join("rust-inputs.json"))
        .expect("Rust manifest を読む");
    assert!(rust.contains("public items exported by the neco-calculus crate root"));
    assert!(rust.contains("dependency types appearing in public neco-calculus signatures"));
    assert!(rust.contains("calculus product private modules"));

    let observer = fs::read_to_string(manifest_directory.join("observer-inputs.json"))
        .expect("observer manifest を読む");
    assert!(observer.contains("canonical observation batch supplied with --batch <file>"));
    assert!(observer.contains("product API crates as direct dependencies"));
    assert!(observer.contains("producer observation generation duplicated in the observer"));

    let observer_manifest = root.join("verification/observer/Cargo.toml");
    let mut metadata = Command::new(cargo_program());
    metadata.current_dir(&root).args([
        "metadata",
        "--manifest-path",
        observer_manifest
            .to_str()
            .expect("observer manifest path is UTF-8"),
        "--locked",
        "--no-deps",
        "--format-version",
        "1",
    ]);
    let output = run(&mut metadata, "observer Cargo metadata");
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("Cargo metadata は JSON");
    let package = metadata["packages"]
        .as_array()
        .expect("Cargo metadata packages は array")
        .iter()
        .find(|package| package["name"] == "neco-calculus-observer")
        .expect("observer package が metadata にあります");
    let direct_dependencies = package["dependencies"]
        .as_array()
        .expect("Cargo metadata dependencies は array")
        .iter()
        .filter_map(|dependency| dependency["name"].as_str())
        .collect::<BTreeSet<_>>();
    let product_dependencies = BTreeSet::from([
        "neco-algnum",
        "neco-bigint",
        "neco-calculus",
        "neco-expr",
        "neco-jet",
        "neco-polynomial",
        "neco-rational",
        "neco-root",
    ]);
    assert!(
        direct_dependencies.is_disjoint(&product_dependencies),
        "observer は製品 API crate に直接依存しません: {direct_dependencies:?}"
    );
}
