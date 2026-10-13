use std::collections::{BTreeMap, BTreeSet};

use calculus_observation_types::{
    decimal, digest_hex, object, CanonicalEncodingFailure, CanonicalObservation, ObservationBatch,
    ObservationOutcome,
};
use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational, ReducedRational, Sign};
use neco_calculus::{
    evaluate_interval, evaluate_polynomial, evaluate_rational, evaluate_surface,
    evaluate_surface_checked, expose_membership_transitions, expose_region_sign,
    refine_root_bisection, CalculusFailure, ConvergenceStatus, Enclosure, EvaluationFailure,
    ExactExpressionRequirement,
    MembershipJudgmentRequest, MembershipLocusKind, NecoFailure, NecoImplementationSource,
    OwnerId,
    Polynomial, PolynomialFailure, PolynomialResourceFailure, RationalFailure, RationalFunction,
    RationalSurfacePolynomials, RegionSign, RootBracket, RootEvidence, RootFailure,
    RootMultiplicity, RootRefinementMethod, SurfaceEvaluationInput, SurfaceEvaluationInput2,
    SurfaceEvaluationProducer, SurfacePolynomial, SurfaceTerm, MAX_PRECISION_BITS,
};
use neco_expr::{
    AbsoluteBits, ConsumerId, ExactDecisionRequest, ExprId,
};
use serde_json::Value;

pub const GENERATED_REQUEST_SCHEMA: &str = "neco.verification.generated-request-batch.v1";
pub const PRODUCER_DESCRIPTION_SCHEMA: &str = "neco.verification.producer-description.v1";
pub const PUBLIC_API_PARITY_SCHEMA: &str = "neco.verification.public-api-parity-result.v1";
pub const ADAPTER_REVISION: &str = "calculus-adapter-v3";
pub const GENERATED_REQUEST_DOMAIN: &str = "neco.calculus";

const RUSTC_IDENTITY: &str = include_str!(concat!(env!("OUT_DIR"), "/rustc-identity.txt"));
const TARGET_TRIPLE: &str = include_str!(concat!(env!("OUT_DIR"), "/target-triple.txt"));
const SEMANTICS_DECLARATION_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/calculus-semantics.json"));

#[derive(Clone, Debug, PartialEq)]
pub struct AdapterFailure(Box<AdapterFailureRecord>);

#[derive(Clone, Debug, PartialEq)]
struct AdapterFailureRecord {
    kind: String,
    failing_operation: String,
    operand_provenance: Value,
    violated_condition: String,
    witness: Option<Value>,
    contributing_failures: Vec<Value>,
}

impl AdapterFailure {
    pub fn command_failure(
        failing_operation: impl Into<String>,
        violated_condition: impl Into<String>,
    ) -> Self {
        Self::new(
            "invalidRequest",
            failing_operation,
            object([]),
            violated_condition,
        )
    }

    pub fn canonical_serialization(
        failing_operation: impl Into<String>,
        failure: CanonicalEncodingFailure,
    ) -> Self {
        canonical_serialization_failure(&failing_operation.into(), object([]), failure)
    }

    pub fn process_io_failure(
        failing_operation: impl Into<String>,
        location: impl Into<String>,
        failure: std::io::Error,
    ) -> Self {
        Self::new(
            "processIoFailure",
            failing_operation,
            object([("location", Value::String(location.into()))]),
            "I/O operation failed",
        )
        .with_witness(object([
            ("errorKind", Value::String(format!("{:?}", failure.kind()))),
            ("message", Value::String(failure.to_string())),
        ]))
    }

    fn new(
        kind: impl Into<String>,
        failing_operation: impl Into<String>,
        operand_provenance: Value,
        violated_condition: impl Into<String>,
    ) -> Self {
        Self(Box::new(AdapterFailureRecord {
            kind: kind.into(),
            failing_operation: failing_operation.into(),
            operand_provenance,
            violated_condition: violated_condition.into(),
            witness: None,
            contributing_failures: Vec::new(),
        }))
    }

    fn with_witness(mut self, witness: Value) -> Self {
        self.0.witness = Some(witness);
        self
    }

    fn with_contributing_failure(mut self, failure: &ProductFailure) -> Self {
        self.0.contributing_failures.push(failure.canonical_value());
        self
    }

    fn with_canonical_encoding_failure(mut self, failure: &CanonicalEncodingFailure) -> Self {
        self.0.contributing_failures.push(object([
            (
                "schema",
                Value::String("neco.verification.canonical-encoding-failure.v1".to_owned()),
            ),
            (
                "violatedCondition",
                Value::String(failure.violated_condition().to_owned()),
            ),
            ("witness", Value::String(failure.witness().to_owned())),
        ]));
        self
    }

    pub fn canonical_value(&self) -> Value {
        object([
            (
                "contributingFailures",
                Value::Array(self.0.contributing_failures.clone()),
            ),
            (
                "failingOperation",
                Value::String(self.0.failing_operation.clone()),
            ),
            ("kind", Value::String(self.0.kind.clone())),
            ("operandProvenance", self.0.operand_provenance.clone()),
            (
                "schema",
                Value::String("neco.verification.adapter-failure.v1".to_owned()),
            ),
            (
                "violatedCondition",
                Value::String(self.0.violated_condition.clone()),
            ),
            (
                "witness",
                match &self.0.witness {
                    Some(witness) => witness.clone(),
                    None => Value::Null,
                },
            ),
        ])
    }
}

impl core::fmt::Display for AdapterFailure {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.0.kind)?;
        formatter.write_str(": ")?;
        formatter.write_str(&self.0.violated_condition)
    }
}

impl std::error::Error for AdapterFailure {}

fn canonical_serialization_failure(
    operation: &str,
    provenance: Value,
    failure: CanonicalEncodingFailure,
) -> AdapterFailure {
    AdapterFailure::new(
        "canonicalSerialization",
        operation,
        provenance,
        failure.violated_condition().to_owned(),
    )
    .with_canonical_encoding_failure(&failure)
    .with_witness(Value::String(failure.witness().to_owned()))
}

const PROJECTION_DECLARATION_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/calculus-projection-declaration.json"));

const RESOURCE_DECLARATION_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/calculus-resource-declaration.json"));

#[derive(Clone, Debug)]
struct ProjectionBucket {
    name: String,
    projected_condition: String,
}

#[derive(Clone, Debug)]
struct SelectorRule {
    selector: String,
    modulus: u64,
    buckets_by_remainder: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
struct SelectorBinding {
    axis: String,
    selector: String,
    buckets: Vec<ProjectionBucket>,
    selector_rule: SelectorRule,
}

#[derive(Clone, Debug)]
struct ProjectionFamilyDeclaration {
    family: String,
    requested_observation_kinds: Vec<String>,
    typed_failure: String,
    bindings: Vec<SelectorBinding>,
}

#[derive(Clone, Debug)]
struct ProjectionDeclarationCatalog {
    projection_api_revision: String,
    families: Vec<ProjectionFamilyDeclaration>,
    value: Value,
    projection_digest: String,
}

type ProjectFunction = fn(
    &GeneratedRequestCase,
    &ProjectionFamilyDeclaration,
    &AdapterContext,
) -> Result<ProductInvocation, AdapterFailure>;

#[derive(Clone, Copy)]
struct ProjectionImplementation {
    family: &'static str,
    project: ProjectFunction,
}

const PROJECTION_IMPLEMENTATIONS: [ProjectionImplementation; 10] = [
    ProjectionImplementation {
        family: "polynomial-exact",
        project: project_polynomial_exact,
    },
    ProjectionImplementation {
        family: "polynomial-enclosure",
        project: project_polynomial_enclosure,
    },
    ProjectionImplementation {
        family: "rational-exact",
        project: project_rational_exact,
    },
    ProjectionImplementation {
        family: "rational-interval",
        project: project_rational_interval,
    },
    ProjectionImplementation {
        family: "root-bracket",
        project: project_root_bracket,
    },
    ProjectionImplementation {
        family: "bisection",
        project: project_bisection,
    },
    ProjectionImplementation {
        family: "even-touch",
        project: project_even_touch,
    },
    ProjectionImplementation {
        family: "local-region-transition",
        project: project_local_region_transition,
    },
    ProjectionImplementation {
        family: "surface-exact",
        project: project_surface_exact,
    },
    ProjectionImplementation {
        family: "surface-checked",
        project: project_surface_checked,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct Selectors {
    value: u64,
    family_parameter: u64,
    observation_class: u64,
    sign: u64,
    multiplicity: u64,
    interval_width: u64,
    formula_variant: u64,
    mutation: u64,
    reserved: u64,
}

impl Selectors {
    fn value_for(&self, name: &str) -> Option<u64> {
        match name {
            "value" => Some(self.value),
            "familyParameter" => Some(self.family_parameter),
            "observationClass" => Some(self.observation_class),
            "sign" => Some(self.sign),
            "multiplicity" => Some(self.multiplicity),
            "intervalWidth" => Some(self.interval_width),
            "formulaVariant" => Some(self.formula_variant),
            "mutation" => Some(self.mutation),
            "reserved" => Some(self.reserved),
            _ => None,
        }
    }

    fn canonical_value(&self) -> Value {
        object([
            ("familyParameter", hex_selector(self.family_parameter)),
            ("formulaVariant", hex_selector(self.formula_variant)),
            ("intervalWidth", hex_selector(self.interval_width)),
            ("multiplicity", hex_selector(self.multiplicity)),
            ("mutation", hex_selector(self.mutation)),
            ("observationClass", hex_selector(self.observation_class)),
            ("reserved", hex_selector(self.reserved)),
            ("sign", hex_selector(self.sign)),
            ("value", hex_selector(self.value)),
        ])
    }
}

#[derive(Clone, Debug)]
struct GeneratedRequestCase {
    case_id: String,
    family: String,
    requested_observation_kind: String,
    selectors: Selectors,
}

#[derive(Clone)]
struct ResourceDeclaration {
    max_batch_cases: usize,
    revision: String,
    declaration_digest: String,
}

#[derive(Clone)]
struct BoundBuildIdentity {
    rustc_identity: String,
    target_triple: String,
    semantics_digest: String,
    semantics_revision: String,
    projection_digest: String,
}

struct AdapterContext {
    identity: BoundBuildIdentity,
    projection: ProjectionDeclarationCatalog,
    resource: ResourceDeclaration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DyadicArgument {
    integer: i64,
    exponent: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PolynomialExactArguments {
    coefficients: Vec<i64>,
    point: i64,
    formula_condition: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PolynomialEnclosureArguments {
    coefficients: Vec<i64>,
    lower: i64,
    upper: i64,
    precision_bits: u32,
    formula_condition: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RationalExactArguments {
    numerator: Vec<i64>,
    denominator: Vec<i64>,
    point: i64,
    formula_condition: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RationalIntervalArguments {
    numerator: Vec<i64>,
    denominator: Vec<i64>,
    lower: i64,
    upper: i64,
    precision_bits: u32,
    formula_condition: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RootBracketArguments {
    coefficients: Vec<i64>,
    lower: i64,
    upper: i64,
    sign_condition: String,
    formula_condition: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BisectionArguments {
    coefficients: Vec<i64>,
    lower: i64,
    upper: i64,
    requested_iterations: usize,
    iteration_condition: String,
    sign_condition: String,
    formula_condition: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EvenTouchArguments {
    coefficients: Vec<i64>,
    lower: DyadicArgument,
    upper: DyadicArgument,
    candidate: DyadicArgument,
    max_order: usize,
    declared_multiplicity: usize,
    formula_condition: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocalRegionArguments {
    coefficients: Vec<i64>,
    lower: i64,
    upper: i64,
    candidate: i64,
    max_order: usize,
    precision_bits: u32,
    declared_multiplicity: usize,
    formula_condition: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurfaceTermArgument {
    coefficient: i64,
    exponent_u: u32,
    exponent_v: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurfaceArguments {
    x: Vec<SurfaceTermArgument>,
    y: Vec<SurfaceTermArgument>,
    z: Vec<SurfaceTermArgument>,
    denominator: Vec<SurfaceTermArgument>,
    formula_condition: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SurfaceExactArguments {
    surface: SurfaceArguments,
    u: i64,
    v: i64,
}

#[derive(Debug, Eq, PartialEq)]
struct SurfaceCheckedArguments {
    surface: SurfaceArguments,
    lower_u: i64,
    upper_u: i64,
    lower_v: i64,
    upper_v: i64,
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
}

#[derive(Debug, Eq, PartialEq)]
enum ProductArguments {
    PolynomialExact(PolynomialExactArguments),
    PolynomialEnclosure(PolynomialEnclosureArguments),
    RationalExact(RationalExactArguments),
    RationalInterval(RationalIntervalArguments),
    RootBracket(RootBracketArguments),
    Bisection(BisectionArguments),
    EvenTouch(EvenTouchArguments),
    LocalRegionTransition(LocalRegionArguments),
    SurfaceExact(SurfaceExactArguments),
    SurfaceChecked(SurfaceCheckedArguments),
}

impl ProductArguments {
    fn kind(&self) -> &'static str {
        match self {
            Self::PolynomialExact(_) => "polynomial-exact",
            Self::PolynomialEnclosure(_) => "polynomial-enclosure",
            Self::RationalExact(_) => "rational-exact",
            Self::RationalInterval(_) => "rational-interval",
            Self::RootBracket(_) => "root-bracket",
            Self::Bisection(_) => "bisection",
            Self::EvenTouch(_) => "even-touch",
            Self::LocalRegionTransition(_) => "local-region-transition",
            Self::SurfaceExact(_) => "surface-exact",
            Self::SurfaceChecked(_) => "surface-checked",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ProductInvocation {
    arguments: ProductArguments,
}

impl ProductInvocation {
    fn input_value(&self) -> Value {
        match &self.arguments {
            ProductArguments::PolynomialExact(arguments) => object([
                ("coefficients", integer_array(&arguments.coefficients)),
                ("point", decimal(arguments.point)),
            ]),
            ProductArguments::PolynomialEnclosure(arguments) => object([
                ("coefficients", integer_array(&arguments.coefficients)),
                ("lower", decimal(arguments.lower)),
                (
                    "precisionBits",
                    Value::Number(arguments.precision_bits.into()),
                ),
                ("upper", decimal(arguments.upper)),
            ]),
            ProductArguments::RationalExact(arguments) => object([
                ("denominator", integer_array(&arguments.denominator)),
                ("numerator", integer_array(&arguments.numerator)),
                ("point", decimal(arguments.point)),
            ]),
            ProductArguments::RationalInterval(arguments) => object([
                ("denominator", integer_array(&arguments.denominator)),
                ("lower", decimal(arguments.lower)),
                ("numerator", integer_array(&arguments.numerator)),
                (
                    "precisionBits",
                    Value::Number(arguments.precision_bits.into()),
                ),
                ("upper", decimal(arguments.upper)),
            ]),
            ProductArguments::RootBracket(arguments) => object([
                ("coefficients", integer_array(&arguments.coefficients)),
                ("lower", decimal(arguments.lower)),
                ("upper", decimal(arguments.upper)),
            ]),
            ProductArguments::Bisection(arguments) => object([
                ("coefficients", integer_array(&arguments.coefficients)),
                (
                    "iterations",
                    Value::Number(arguments.requested_iterations.into()),
                ),
                ("lower", decimal(arguments.lower)),
                ("upper", decimal(arguments.upper)),
            ]),
            ProductArguments::EvenTouch(arguments) => object([
                ("candidate", dyadic_decimal(&arguments.candidate)),
                ("coefficients", integer_array(&arguments.coefficients)),
                ("maxOrder", Value::Number(arguments.max_order.into())),
            ]),
            ProductArguments::LocalRegionTransition(arguments) => object([
                ("candidate", decimal(arguments.candidate)),
                ("coefficients", integer_array(&arguments.coefficients)),
                ("lower", decimal(arguments.lower)),
                ("maxOrder", Value::Number(arguments.max_order.into())),
                (
                    "precisionBits",
                    Value::Number(arguments.precision_bits.into()),
                ),
                ("upper", decimal(arguments.upper)),
            ]),
            ProductArguments::SurfaceExact(arguments) => surface_input_value(
                &arguments.surface,
                decimal(arguments.u),
                decimal(arguments.v),
                None,
            ),
            ProductArguments::SurfaceChecked(arguments) => surface_input_value(
                &arguments.surface,
                enclosure_argument_value(arguments.lower_u, arguments.upper_u),
                enclosure_argument_value(arguments.lower_v, arguments.upper_v),
                Some((&arguments.owner, &arguments.requirement, &arguments.source)),
            ),
        }
    }

    fn projected_conditions(&self) -> BTreeMap<&'static str, String> {
        let mut conditions = BTreeMap::new();
        match &self.arguments {
            ProductArguments::PolynomialExact(arguments) => {
                conditions.insert("value", format!("value={}", arguments.point));
                conditions.insert("formulaVariant", arguments.formula_condition.clone());
            }
            ProductArguments::PolynomialEnclosure(arguments) => {
                conditions.insert("value", format!("value={}", arguments.lower));
                conditions.insert(
                    "intervalWidth",
                    format!("upper-lower={}", arguments.upper - arguments.lower),
                );
                let selected_precision = if arguments.precision_bits > MAX_PRECISION_BITS {
                    arguments.precision_bits - MAX_PRECISION_BITS
                } else {
                    arguments.precision_bits
                };
                conditions.insert(
                    "familyParameter",
                    format!(
                        "successPrecisionBits={selected_precision},failurePrecisionBits={}",
                        MAX_PRECISION_BITS + selected_precision
                    ),
                );
                conditions.insert("formulaVariant", arguments.formula_condition.clone());
            }
            ProductArguments::RationalExact(arguments) => {
                conditions.insert("value", format!("value={}", arguments.point));
                conditions.insert("formulaVariant", arguments.formula_condition.clone());
            }
            ProductArguments::RationalInterval(arguments) => {
                conditions.insert("value", format!("value={}", arguments.lower));
                conditions.insert(
                    "intervalWidth",
                    format!("upper-lower={}", arguments.upper - arguments.lower),
                );
                conditions.insert(
                    "familyParameter",
                    format!("precisionBits={}", arguments.precision_bits),
                );
                conditions.insert("formulaVariant", arguments.formula_condition.clone());
            }
            ProductArguments::RootBracket(arguments) => {
                conditions.insert("sign", arguments.sign_condition.clone());
                conditions.insert(
                    "intervalWidth",
                    format!("upper-lower={}", arguments.upper - arguments.lower),
                );
                conditions.insert("formulaVariant", arguments.formula_condition.clone());
            }
            ProductArguments::Bisection(arguments) => {
                conditions.insert("familyParameter", arguments.iteration_condition.clone());
                conditions.insert("sign", arguments.sign_condition.clone());
                conditions.insert(
                    "intervalWidth",
                    format!("upper-lower={}", arguments.upper - arguments.lower),
                );
                conditions.insert("formulaVariant", arguments.formula_condition.clone());
            }
            ProductArguments::EvenTouch(arguments) => {
                conditions.insert(
                    "multiplicity",
                    format!(
                        "successRootMultiplicity={},failureRootMultiplicity={}",
                        arguments.declared_multiplicity,
                        arguments.declared_multiplicity + 1
                    ),
                );
                conditions.insert(
                    "intervalWidth",
                    format!(
                        "upper-lower={}",
                        dyadic_argument_width(arguments.lower, arguments.upper)
                    ),
                );
                conditions.insert("formulaVariant", arguments.formula_condition.clone());
            }
            ProductArguments::LocalRegionTransition(arguments) => {
                conditions.insert(
                    "multiplicity",
                    format!("rootMultiplicity={}", arguments.declared_multiplicity),
                );
                conditions.insert(
                    "intervalWidth",
                    format!("upper-lower={}", arguments.upper - arguments.lower),
                );
                conditions.insert(
                    "familyParameter",
                    format!("precisionBits={}", arguments.precision_bits),
                );
                conditions.insert("formulaVariant", arguments.formula_condition.clone());
            }
            ProductArguments::SurfaceExact(arguments) => {
                conditions.insert("value", format!("value={}", arguments.u));
                conditions.insert("familyParameter", format!("value={}", arguments.v));
                conditions.insert(
                    "formulaVariant",
                    arguments.surface.formula_condition.to_owned(),
                );
            }
            ProductArguments::SurfaceChecked(arguments) => {
                conditions.insert("value", format!("value={}", arguments.lower_u));
                conditions.insert("familyParameter", format!("value={}", arguments.lower_v));
                conditions.insert(
                    "intervalWidth",
                    format!("upper-lower={}", arguments.upper_u - arguments.lower_u),
                );
                conditions.insert(
                    "formulaVariant",
                    arguments.surface.formula_condition.to_owned(),
                );
            }
        }
        conditions
    }
}

#[derive(Debug)]
enum ProductError {
    Polynomial(PolynomialFailure),
    Rational(RationalFailure),
    Root(RootFailure),
    Calculus(CalculusFailure),
    Expression(NecoFailure),
}

#[derive(Clone, Copy, Debug)]
enum ProductFailureStage {
    Construction,
    Execution,
}

#[derive(Debug)]
struct ProductFailure {
    stage: ProductFailureStage,
    error: ProductError,
}

impl ProductFailure {
    fn construction(error: ProductError) -> Self {
        Self {
            stage: ProductFailureStage::Construction,
            error,
        }
    }

    fn execution(error: ProductError) -> Self {
        Self {
            stage: ProductFailureStage::Execution,
            error,
        }
    }

    fn canonical_value(&self) -> Value {
        let stage = match self.stage {
            ProductFailureStage::Construction => "construction",
            ProductFailureStage::Execution => "execution",
        };
        let (source, debug) = match &self.error {
            ProductError::Polynomial(error) => {
                ("neco_calculus::PolynomialFailure", format!("{error:?}"))
            }
            ProductError::Rational(error) => {
                ("neco_calculus::RationalFailure", format!("{error:?}"))
            }
            ProductError::Root(error) => ("neco_calculus::RootFailure", format!("{error:?}")),
            ProductError::Calculus(error) => {
                ("neco_calculus::CalculusFailure", format!("{error:?}"))
            }
            ProductError::Expression(error) => ("neco_expr::NecoFailure", format!("{error:?}")),
        };
        object([
            ("debug", Value::String(debug)),
            (
                "schema",
                Value::String("neco.verification.product-failure.v1".to_owned()),
            ),
            ("source", Value::String(source.to_owned())),
            ("stage", Value::String(stage.to_owned())),
        ])
    }
}

#[derive(Debug)]
struct BisectionStepOutcome {
    midpoint: Dyadic,
    midpoint_sign: Sign,
    selection: &'static str,
    updated_enclosure: Enclosure,
}

#[derive(Debug)]
struct BisectionProductOutcome {
    aggregate: neco_calculus::RootIsolation,
    trace: Vec<BisectionStepOutcome>,
}

#[derive(Debug)]
struct LocalRegionProductOutcome {
    sign: neco_calculus::RegionSignJudgment,
    transitions: neco_calculus::MembershipTransitionReport,
}

#[derive(Debug)]
enum ProductSuccess {
    PolynomialExact(neco_calculus::PolynomialEvaluation),
    PolynomialEnclosure(neco_calculus::IntervalEvaluation),
    RationalExact(neco_calculus::RationalEvaluation),
    RationalInterval(neco_calculus::IntervalEvaluation),
    RootBracket(RootBracket),
    Bisection(BisectionProductOutcome),
    EvenTouch(RootBracket),
    LocalRegionTransition(LocalRegionProductOutcome),
    SurfaceExact(SurfaceEvaluationProducer),
    SurfaceChecked(SurfaceEvaluationProducer),
}

impl ProductSuccess {
    fn kind(&self) -> &'static str {
        match self {
            Self::PolynomialExact(_) => "polynomial-exact-success",
            Self::PolynomialEnclosure(_) => "polynomial-enclosure-success",
            Self::RationalExact(_) => "rational-exact-success",
            Self::RationalInterval(_) => "rational-interval-success",
            Self::RootBracket(_) => "root-bracket-success",
            Self::Bisection(_) => "bisection-success",
            Self::EvenTouch(_) => "even-touch-success",
            Self::LocalRegionTransition(_) => "local-region-transition-success",
            Self::SurfaceExact(_) => "surface-exact-success",
            Self::SurfaceChecked(_) => "surface-checked-success",
        }
    }
}

#[derive(Debug)]
enum ProductOutcome {
    Success(Box<ProductSuccess>),
    Failure(ProductFailure),
}

impl ProductOutcome {
    fn success(value: ProductSuccess) -> Self {
        Self::Success(Box::new(value))
    }
}

fn resource_declaration() -> Result<ResourceDeclaration, AdapterFailure> {
    let value = calculus_observation_types::validate_canonical_value(RESOURCE_DECLARATION_BYTES)
        .map_err(|_| {
            AdapterFailure::new(
                "unboundBuildIdentity",
                "decodeResourceDeclaration",
                object([]),
                "resource declaration must be canonical JSON",
            )
        })?;
    decode_resource_declaration(&value)
}

fn decode_resource_declaration(value: &Value) -> Result<ResourceDeclaration, AdapterFailure> {
    let invalid = |condition: &'static str| {
        AdapterFailure::new(
            "unboundBuildIdentity",
            "validateResourceDeclaration",
            object([]),
            condition,
        )
    };
    let declaration = value
        .as_object()
        .ok_or_else(|| invalid("resource declaration must be an object"))?;
    if declaration.len() != 5
        || ![
            "domain",
            "maxBatchCases",
            "resourceLimits",
            "resourceRevision",
            "schema",
        ]
        .iter()
        .all(|field| declaration.contains_key(*field))
    {
        return Err(invalid(
            "resource declaration fields do not match canonical schema",
        ));
    }
    if declaration.get("domain").and_then(Value::as_str) != Some(GENERATED_REQUEST_DOMAIN)
        || declaration.get("schema").and_then(Value::as_str)
            != Some("neco.verification.resource-declaration.v1")
    {
        return Err(invalid("resource declaration identity is incomplete"));
    }
    let limits = declaration
        .get("resourceLimits")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("resource limits must be an object"))?;
    if limits.len() != 4
        || limits.get("cpuTimeSeconds").and_then(Value::as_u64) != Some(60)
        || limits.get("memoryBytes").and_then(Value::as_u64) != Some(8589934592)
        || limits.get("stdoutPayloadBytes").and_then(Value::as_u64) != Some(67108864)
        || limits.get("timeoutSeconds").and_then(Value::as_u64) != Some(120)
    {
        return Err(invalid("resource limits do not match canonical schema"));
    }
    let revision = declaration
        .get("resourceRevision")
        .and_then(Value::as_str)
        .filter(|revision| !revision.is_empty())
        .ok_or_else(|| invalid("resource revision must be a non-empty string"))?;
    let max_batch_cases = declaration
        .get("maxBatchCases")
        .and_then(Value::as_u64)
        .filter(|maximum| *maximum > 0)
        .and_then(|maximum| usize::try_from(maximum).ok())
        .ok_or_else(|| invalid("maximum batch cases must be a positive integer"))?;
    Ok(ResourceDeclaration {
        max_batch_cases,
        revision: revision.to_owned(),
        declaration_digest: digest_hex(RESOURCE_DECLARATION_BYTES),
    })
}

fn projection_declaration_catalog() -> Result<ProjectionDeclarationCatalog, AdapterFailure> {
    let value = calculus_observation_types::validate_canonical_value(PROJECTION_DECLARATION_BYTES)
        .map_err(|_| {
            AdapterFailure::new(
                "unsupportedProjection",
                "decodeProjectionDeclaration",
                object([]),
                "projection declaration must be canonical JSON",
            )
        })?;
    validate_projection_metadata(&value)?;
    let declaration = value.as_object().ok_or_else(|| {
        AdapterFailure::new(
            "unsupportedProjection",
            "decodeProjectionDeclaration",
            object([]),
            "projection declaration must be an object",
        )
    })?;
    let families = declaration
        .get("families")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AdapterFailure::new(
                "unsupportedProjection",
                "decodeProjectionDeclaration",
                object([]),
                "projection declaration families must be an array",
            )
        })?
        .iter()
        .map(decode_projection_family)
        .collect::<Result<Vec<_>, _>>()?;
    let encoded = calculus_observation_types::canonical_value_bytes(&value["families"]).map_err(
        |failure| {
            AdapterFailure::new(
                "unsupportedProjection",
                "decodeProjectionDeclaration",
                object([]),
                failure.violated_condition().to_owned(),
            )
        },
    )?;
    Ok(ProjectionDeclarationCatalog {
        projection_api_revision: metadata_string(declaration, "projectionApiRevision")?,
        families,
        value,
        projection_digest: digest_hex(&encoded),
    })
}

fn decode_projection_family(value: &Value) -> Result<ProjectionFamilyDeclaration, AdapterFailure> {
    let family = value.as_object().ok_or_else(|| {
        AdapterFailure::new(
            "unsupportedProjection",
            "decodeProjectionDeclaration",
            object([]),
            "projection family must be an object",
        )
    })?;
    let requested_observation_kinds = family
        .get("requestedObservationKinds")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AdapterFailure::new(
                "unsupportedProjection",
                "decodeProjectionDeclaration",
                object([]),
                "requested observation kinds must be an array",
            )
        })?
        .iter()
        .map(|value| {
            value.as_str().map(ToOwned::to_owned).ok_or_else(|| {
                AdapterFailure::new(
                    "unsupportedProjection",
                    "decodeProjectionDeclaration",
                    object([]),
                    "requested observation kind must be a string",
                )
            })
        })
        .collect::<Result<Vec<_>, AdapterFailure>>()?;
    let bindings = family
        .get("selectorBindings")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AdapterFailure::new(
                "unsupportedProjection",
                "decodeProjectionDeclaration",
                object([]),
                "projection family bindings must be an array",
            )
        })?
        .iter()
        .map(decode_selector_binding)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ProjectionFamilyDeclaration {
        family: metadata_string(family, "family")?,
        requested_observation_kinds,
        typed_failure: metadata_string(family, "typedFailure")?,
        bindings,
    })
}

fn decode_selector_binding(value: &Value) -> Result<SelectorBinding, AdapterFailure> {
    let binding = value.as_object().ok_or_else(|| {
        AdapterFailure::new(
            "unsupportedProjection",
            "decodeProjectionDeclaration",
            object([]),
            "selector binding must be an object",
        )
    })?;
    let buckets = binding
        .get("buckets")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AdapterFailure::new(
                "unsupportedProjection",
                "decodeProjectionDeclaration",
                object([]),
                "selector binding buckets must be an array",
            )
        })?
        .iter()
        .map(|value| {
            let bucket = value.as_object().ok_or_else(|| {
                AdapterFailure::new(
                    "unsupportedProjection",
                    "decodeProjectionDeclaration",
                    object([]),
                    "selector bucket must be an object",
                )
            })?;
            Ok(ProjectionBucket {
                name: metadata_string(bucket, "name")?,
                projected_condition: metadata_string(bucket, "projectedCondition")?,
            })
        })
        .collect::<Result<Vec<_>, AdapterFailure>>()?;
    let selector = metadata_string(binding, "selector")?;
    let selector_rule = decode_selector_rule(binding, &selector, &buckets)?;
    Ok(SelectorBinding {
        axis: metadata_string(binding, "axis")?,
        selector,
        buckets,
        selector_rule,
    })
}

fn decode_selector_rule(
    binding: &serde_json::Map<String, Value>,
    selector: &str,
    buckets: &[ProjectionBucket],
) -> Result<SelectorRule, AdapterFailure> {
    let invalid = || {
        AdapterFailure::new(
            "unsupportedProjection",
            "decodeProjectionDeclaration",
            object([]),
            "selector rule metadata is incomplete",
        )
    };
    let rule = binding
        .get("selectorRule")
        .and_then(Value::as_object)
        .ok_or_else(invalid)?;
    if rule.len() != 3
        || !["bucketsByRemainder", "modulus", "selector"]
            .iter()
            .all(|field| rule.contains_key(*field))
        || rule.get("selector").and_then(Value::as_str) != Some(selector)
    {
        return Err(invalid());
    }
    let modulus = rule
        .get("modulus")
        .and_then(Value::as_u64)
        .filter(|modulus| usize::try_from(*modulus).ok() == Some(buckets.len()))
        .ok_or_else(invalid)?;
    let remainders = rule
        .get("bucketsByRemainder")
        .and_then(Value::as_object)
        .ok_or_else(invalid)?;
    if remainders.len() != buckets.len() {
        return Err(invalid());
    }
    let mut buckets_by_remainder = BTreeMap::new();
    for (index, bucket) in buckets.iter().enumerate() {
        let key = index.to_string();
        if remainders.get(&key).and_then(Value::as_str) != Some(bucket.name.as_str()) {
            return Err(invalid());
        }
        buckets_by_remainder.insert(key, bucket.name.clone());
    }
    Ok(SelectorRule {
        selector: selector.to_owned(),
        modulus,
        buckets_by_remainder,
    })
}

fn metadata_string(
    object_value: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, AdapterFailure> {
    object_value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            AdapterFailure::new(
                "unsupportedProjection",
                "decodeProjectionDeclaration",
                object([]),
                "projection declaration string field is absent",
            )
        })
}

fn validate_projection_metadata(value: &Value) -> Result<(), AdapterFailure> {
    let invalid = |condition: &'static str| {
        AdapterFailure::new(
            "unsupportedProjection",
            "validateProjectionDeclaration",
            object([]),
            condition,
        )
    };
    let declaration = value
        .as_object()
        .ok_or_else(|| invalid("projection declaration must be an object"))?;
    if declaration.len() != 4
        || !["domain", "families", "projectionApiRevision", "schema"]
            .iter()
            .all(|field| declaration.contains_key(*field))
    {
        return Err(invalid(
            "projection declaration fields do not match canonical schema",
        ));
    }
    if declaration.get("domain").and_then(Value::as_str) != Some(GENERATED_REQUEST_DOMAIN)
        || declaration.get("schema").and_then(Value::as_str)
            != Some("neco.calculus.generated-projection-declaration.v1")
        || declaration
            .get("projectionApiRevision")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
    {
        return Err(invalid("projection declaration identity is incomplete"));
    }
    let families = declaration
        .get("families")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("families must be an array"))?;
    if families.len() != PROJECTION_IMPLEMENTATIONS.len() {
        return Err(invalid("family catalog is incomplete"));
    }
    let mut family_names = BTreeSet::new();
    let mut checker_predicates = BTreeSet::new();
    for family in families {
        let family = family
            .as_object()
            .ok_or_else(|| invalid("family declaration must be an object"))?;
        if family.len() != 6
            || ![
                "applicableCoverageAxes",
                "checkerPredicate",
                "family",
                "requestedObservationKinds",
                "selectorBindings",
                "typedFailure",
            ]
            .iter()
            .all(|field| family.contains_key(*field))
        {
            return Err(invalid("family declaration metadata is incomplete"));
        }
        let family_name = family
            .get("family")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| invalid("family name is absent"))?;
        if !family_names.insert(family_name)
            || !PROJECTION_IMPLEMENTATIONS
                .iter()
                .any(|implementation| implementation.family == family_name)
        {
            return Err(invalid("family name is duplicated or unsupported"));
        }
        let typed_failure = family
            .get("typedFailure")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| invalid("typed failure is absent"))?;
        let requested_kinds = family
            .get("requestedObservationKinds")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("requested observation kinds must be an array"))?;
        if requested_kinds.len() != 2
            || requested_kinds[0] != "success"
            || requested_kinds[1] != typed_failure
            || typed_failure == "success"
        {
            return Err(invalid("requested observation kinds are incomplete"));
        }
        let checker_predicate = family
            .get("checkerPredicate")
            .and_then(Value::as_str)
            .filter(|predicate| !predicate.is_empty())
            .ok_or_else(|| invalid("checker predicate is absent"))?;
        if !checker_predicates.insert(checker_predicate) {
            return Err(invalid("checker predicate is duplicated"));
        }
        let axes = family
            .get("applicableCoverageAxes")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("applicable coverage axes must be an array"))?;
        let bindings = family
            .get("selectorBindings")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("selector bindings must be an array"))?;
        if axes.is_empty() || axes.len() != bindings.len() {
            return Err(invalid("coverage axes and selector bindings differ"));
        }
        let mut seen_axes = BTreeSet::new();
        let mut seen_selectors = BTreeSet::new();
        for (axis, binding) in axes.iter().zip(bindings) {
            let axis = axis
                .as_str()
                .filter(|name| !name.is_empty())
                .ok_or_else(|| invalid("coverage axis is absent"))?;
            let binding = binding
                .as_object()
                .ok_or_else(|| invalid("selector binding must be an object"))?;
            if binding.len() != 4
                || !["axis", "buckets", "selector", "selectorRule"]
                    .iter()
                    .all(|field| binding.contains_key(*field))
                || binding.get("axis").and_then(Value::as_str) != Some(axis)
            {
                return Err(invalid("selector binding metadata is incomplete"));
            }
            let selector = binding
                .get("selector")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| invalid("selector name is absent"))?;
            if !seen_axes.insert(axis) || !seen_selectors.insert(selector) {
                return Err(invalid("axis or selector binding is duplicated"));
            }
            let buckets = binding
                .get("buckets")
                .and_then(Value::as_array)
                .filter(|buckets| !buckets.is_empty())
                .ok_or_else(|| invalid("selector buckets are absent"))?;
            let mut bucket_names = BTreeSet::new();
            for bucket in buckets {
                let bucket = bucket
                    .as_object()
                    .ok_or_else(|| invalid("selector bucket must be an object"))?;
                if bucket.len() != 2
                    || !["name", "projectedCondition"]
                        .iter()
                        .all(|field| bucket.contains_key(*field))
                {
                    return Err(invalid("selector bucket metadata is incomplete"));
                }
                let name = bucket
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|name| !name.is_empty())
                    .ok_or_else(|| invalid("selector bucket name is absent"))?;
                if !bucket_names.insert(name)
                    || bucket
                        .get("projectedCondition")
                        .and_then(Value::as_str)
                        .is_none_or(str::is_empty)
                {
                    return Err(invalid(
                        "selector bucket is duplicated or lacks projectedCondition",
                    ));
                }
            }
            let rule = binding
                .get("selectorRule")
                .and_then(Value::as_object)
                .ok_or_else(|| invalid("selector rule must be an object"))?;
            if rule.len() != 3
                || !["bucketsByRemainder", "modulus", "selector"]
                    .iter()
                    .all(|field| rule.contains_key(*field))
                || rule.get("selector").and_then(Value::as_str) != Some(selector)
            {
                return Err(invalid("selector rule metadata is incomplete"));
            }
            let modulus = rule
                .get("modulus")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid("selector rule modulus is absent"))?;
            if usize::try_from(modulus).ok() != Some(buckets.len()) {
                return Err(invalid("selector rule modulus does not match bucket count"));
            }
            let remainders = rule
                .get("bucketsByRemainder")
                .and_then(Value::as_object)
                .ok_or_else(|| invalid("selector rule remainders must be an object"))?;
            if remainders.len() != buckets.len() {
                return Err(invalid("selector rule remainders are incomplete"));
            }
            for (index, bucket) in buckets.iter().enumerate() {
                let name = bucket
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("selector bucket name is absent"))?;
                if remainders.get(&index.to_string()).and_then(Value::as_str) != Some(name) {
                    return Err(invalid("selector rule remainder map is not total"));
                }
            }
        }
    }
    let implementation_names = PROJECTION_IMPLEMENTATIONS
        .iter()
        .map(|implementation| implementation.family)
        .collect::<BTreeSet<_>>();
    if family_names != implementation_names {
        return Err(invalid(
            "family catalog and projection implementations differ",
        ));
    }
    Ok(())
}

fn bound_build_identity(
    projection_digest: String,
) -> Result<BoundBuildIdentity, AdapterFailure> {
    let rustc_identity = RUSTC_IDENTITY.trim_end().to_owned();
    let target_triple = TARGET_TRIPLE.trim_end().to_owned();
    if !rustc_identity.starts_with("rustc ") || target_triple.is_empty() {
        return Err(AdapterFailure::new(
            "unboundBuildIdentity",
            "bindBuildIdentity",
            object([]),
            "rustc identity and target triple must be bound from the producer build",
        ));
    }
    let semantics =
        calculus_observation_types::validate_canonical_value(SEMANTICS_DECLARATION_BYTES)
            .map_err(|_| {
                AdapterFailure::new(
                    "unboundBuildIdentity",
                    "decodeSemanticsDeclaration",
                    object([]),
                    "semantics declaration must be canonical JSON",
                )
            })?;
    if !semantics_declaration_valid(&semantics) {
        return Err(AdapterFailure::new("unboundBuildIdentity", "decodeSemanticsDeclaration",
            object([]), "semantics declaration fields are invalid"));
    }
    let revision = semantics
        .get("revision")
        .and_then(Value::as_str)
        .filter(|revision| !revision.is_empty())
        .ok_or_else(|| {
            AdapterFailure::new(
                "unboundBuildIdentity",
                "decodeSemanticsDeclaration",
                object([]),
                "semantics revision must be a non-empty string",
            )
        })?;
    Ok(BoundBuildIdentity {
        rustc_identity,
        target_triple,
        semantics_digest: digest_hex(SEMANTICS_DECLARATION_BYTES),
        semantics_revision: revision.to_owned(),
        projection_digest,
    })
}

fn semantics_declaration_valid(value: &Value) -> bool {
    fn fields(value: &Value, names: &[&str]) -> bool {
        value.as_object().is_some_and(|v| v.len() == names.len() && names.iter().all(|n| v.contains_key(*n)))
    }
    fn modules(value: &Value, owner: &str) -> bool {
        value.as_array().is_some_and(|a| !a.is_empty() && a.iter().enumerate().all(|(i,m)| {
            fields(m, &["module", "sourcePath"]) &&
                m["module"].as_str().zip(m["sourcePath"].as_str()).is_some_and(|(name,path)|
                    name.starts_with(owner) &&
                    path.strip_prefix("verification/lean/").and_then(|p| p.strip_suffix(".lean"))
                        .is_some_and(|p| name.chars().map(|c| if c == '.' {'/'} else {c}).eq(p.chars()))) &&
                !a[..i].iter().any(|previous| previous["module"] == m["module"])
        }))
    }
    fields(value, &["schema", "domain", "revision", "calculusCheckerApiRevision", "calculusModuleRevision", "ownedModules", "upstreamSemantics"]) &&
        value["schema"] == "neco.verification.semantics-declaration.v2" && value["domain"] == "neco.calculus" &&
        ["revision", "calculusCheckerApiRevision", "calculusModuleRevision"].iter().all(|f| value[*f].as_str().is_some_and(|s| !s.is_empty())) &&
        modules(&value["ownedModules"], "NecoCalculusVerify.") &&
        value["upstreamSemantics"].as_array().is_some_and(|a| a.len() == 1 && a.iter().all(|r|
            fields(r, &["domain", "revision", "sourcePath"]) && r["domain"] == "neco.algebra" &&
            r["revision"].as_str().is_some_and(|s| !s.is_empty()) &&
            r["sourcePath"] == "verification/manifest/algebra-semantics.json"))
}

fn adapter_context() -> Result<AdapterContext, AdapterFailure> {
    let projection = projection_declaration_catalog()?;
    let identity = bound_build_identity(projection.projection_digest.clone())?;
    let resource = resource_declaration()?;
    Ok(AdapterContext {
        identity,
        projection,
        resource,
    })
}

fn self_observation_value(identity: &BoundBuildIdentity) -> Value {
    object([
        (
            "rustcIdentity",
            Value::String(identity.rustc_identity.clone()),
        ),
        (
            "targetTriple",
            Value::String(identity.target_triple.clone()),
        ),
    ])
}

fn projection_claim_value(declaration: &ProjectionDeclarationCatalog) -> Value {
    object([
        (
            "projectionDeclarationDigest",
            Value::String(declaration.projection_digest.clone()),
        ),
        (
            "projectionDeclarations",
            declaration.value["families"].clone(),
        ),
        (
            "projectionRevision",
            Value::String(declaration.projection_digest.clone()),
        ),
    ])
}

fn resource_claim_value(resource: &ResourceDeclaration) -> Value {
    object([
        (
            "maxBatchCases",
            Value::Number(resource.max_batch_cases.into()),
        ),
        (
            "resourceDeclarationDigest",
            Value::String(resource.declaration_digest.clone()),
        ),
        ("resourceRevision", Value::String(resource.revision.clone())),
    ])
}

pub fn describe() -> Result<Value, AdapterFailure> {
    let context = adapter_context()?;
    Ok(object([
        (
            "adapterRevision",
            Value::String(ADAPTER_REVISION.to_owned()),
        ),
        ("domain", Value::String(GENERATED_REQUEST_DOMAIN.to_owned())),
        (
            "producerApiRevision",
            Value::String(context.projection.projection_api_revision.clone()),
        ),
        (
            "projectionClaim",
            projection_claim_value(&context.projection),
        ),
        (
            "resourceClaim",
            resource_claim_value(&context.resource),
        ),
        (
            "rustcIdentity",
            Value::String(context.identity.rustc_identity.clone()),
        ),
        (
            "schema",
            Value::String(PRODUCER_DESCRIPTION_SCHEMA.to_owned()),
        ),
        (
            "semanticsRef",
            object([
                (
                    "declarationDigest",
                    Value::String(context.identity.semantics_digest.clone()),
                ),
                (
                    "revision",
                    Value::String(context.identity.semantics_revision.clone()),
                ),
            ]),
        ),
        (
            "targetTriple",
            Value::String(context.identity.target_triple.clone()),
        ),
    ]))
}

pub fn generate_request_batch(request_bytes: &[u8]) -> Result<ObservationBatch, AdapterFailure> {
    let context = adapter_context()?;
    let cases = decode_generated_request(request_bytes, &context)?;
    let observations = cases
        .iter()
        .map(|case| project_request_case(case, &context))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ObservationBatch::new(observations))
}

pub fn self_check_request_batch(
    request_bytes: &[u8],
    supplied_batch_bytes: &[u8],
) -> Result<Value, AdapterFailure> {
    let context = adapter_context()?;
    let cases = decode_generated_request(request_bytes, &context)?;
    let generated = ObservationBatch::new(
        cases
            .iter()
            .map(|case| project_request_case(case, &context))
            .collect::<Result<Vec<_>, _>>()?,
    );
    let supplied =
        calculus_observation_types::validate_canonical(supplied_batch_bytes).map_err(|_| {
            AdapterFailure::new(
                "invalidRequest",
                "selfCheck",
                object([]),
                "supplied batch is not canonical calculus observation JSON",
            )
        })?;
    let expected_bytes = calculus_observation_types::canonical_bytes(&generated)
        .map_err(|failure| canonical_serialization_failure("selfCheck", object([]), failure))?;
    let supplied_bytes = calculus_observation_types::canonical_bytes(&supplied)
        .map_err(|failure| canonical_serialization_failure("selfCheck", object([]), failure))?;
    if expected_bytes != supplied_bytes {
        return Err(AdapterFailure::new(
            "unexpectedPublicApiOutcome",
            "selfCheck",
            object([
                (
                    "projectionRevision",
                    Value::String(context.identity.projection_digest.clone()),
                ),
                (
                    "resourceRevision",
                    Value::String(context.resource.revision.clone()),
                ),
            ]),
            "public API projection does not match supplied batch",
        ));
    }
    Ok(object([
        (
            "adapterRevision",
            Value::String(ADAPTER_REVISION.to_owned()),
        ),
        ("caseCount", Value::Number(generated.cases().len().into())),
        (
            "observationDigest",
            Value::String(digest_hex(&expected_bytes)),
        ),
        (
            "projectionRevision",
            Value::String(context.identity.projection_digest),
        ),
        ("requestDigest", Value::String(digest_hex(request_bytes))),
        (
            "resourceRevision",
            Value::String(context.resource.revision.clone()),
        ),
        ("result", Value::String("matched".to_owned())),
        ("schema", Value::String(PUBLIC_API_PARITY_SCHEMA.to_owned())),
        (
            "semanticsRef",
            object([
                (
                    "declarationDigest",
                    Value::String(context.identity.semantics_digest),
                ),
                (
                    "revision",
                    Value::String(context.identity.semantics_revision),
                ),
            ]),
        ),
    ]))
}

fn decode_generated_request(
    request_bytes: &[u8],
    context: &AdapterContext,
) -> Result<Vec<GeneratedRequestCase>, AdapterFailure> {
    let request =
        calculus_observation_types::validate_canonical_value(request_bytes).map_err(|_| {
            AdapterFailure::new(
                "invalidRequest",
                "decodeRequest",
                object([]),
                "request must be canonical JSON",
            )
        })?;
    let request_object = request.as_object().ok_or_else(|| {
        AdapterFailure::new(
            "invalidRequest",
            "decodeRequest",
            object([]),
            "request batch must be an object",
        )
    })?;
    require_exact_fields(
        request_object,
        &["adapterRevision", "cases", "domain", "generator", "schema"],
        "decodeRequest",
        object([]),
    )?;
    require_string(
        request_object,
        "schema",
        GENERATED_REQUEST_SCHEMA,
        "decodeRequest",
        object([]),
    )?;
    require_string(
        request_object,
        "domain",
        GENERATED_REQUEST_DOMAIN,
        "decodeRequest",
        object([]),
    )?;
    require_string(
        request_object,
        "adapterRevision",
        ADAPTER_REVISION,
        "decodeRequest",
        object([]),
    )?;
    validate_generator(request_object.get("generator"), &context.identity)?;
    let request_cases = request_object
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AdapterFailure::new(
                "invalidRequest",
                "decodeRequest",
                object([]),
                "cases must be an array",
            )
        })?;
    if request_cases.len() > context.resource.max_batch_cases {
        return Err(AdapterFailure::new(
            "batchResourceLimit",
            "decodeRequest",
            object([(
                "resourceRevision",
                Value::String(context.resource.revision.clone()),
            )]),
            "case count exceeds producer resource limit",
        )
        .with_witness(object([
            (
                "maxBatchCases",
                Value::Number(context.resource.max_batch_cases.into()),
            ),
            (
                "receivedCaseCount",
                Value::Number(request_cases.len().into()),
            ),
        ])));
    }
    let mut case_ids = BTreeSet::new();
    let mut previous_domain_case_index = None;
    request_cases
        .iter()
        .map(|value| {
            let case = decode_request_case(value, context)?;
            let domain_case_index = parse_hex_u64(
                &case_id_field(value, "domainCaseIndex")?,
                "domainCaseIndex",
                case.provenance(context),
            )?;
            if previous_domain_case_index.is_some_and(|previous| domain_case_index <= previous) {
                return Err(AdapterFailure::new(
                    "invalidRequest",
                    "decodeRequest",
                    case.provenance(context),
                    "domainCaseIndex must be strictly ordered",
                ));
            }
            previous_domain_case_index = Some(domain_case_index);
            if !case_ids.insert(case.case_id.clone()) {
                return Err(AdapterFailure::new(
                    "invalidRequest",
                    "decodeRequest",
                    case.provenance(context),
                    "caseId must be unique",
                ));
            }
            Ok(case)
        })
        .collect()
}

fn validate_generator(
    value: Option<&Value>,
    identity: &BoundBuildIdentity,
) -> Result<(), AdapterFailure> {
    let generator = value.and_then(Value::as_object).ok_or_else(|| {
        AdapterFailure::new(
            "invalidRequest",
            "decodeRequest",
            object([]),
            "generator must be an object",
        )
    })?;
    require_exact_fields(
        generator,
        &["algorithm", "drawScheduleRevision", "seed"],
        "decodeRequest",
        object([]),
    )?;
    require_string(
        generator,
        "algorithm",
        "splitmix64-v1",
        "decodeRequest",
        object([]),
    )?;
    require_string(
        generator,
        "drawScheduleRevision",
        "draw-schedule-v1",
        "decodeRequest",
        object([]),
    )?;
    let seed = generator
        .get("seed")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AdapterFailure::new(
                "invalidRequest",
                "decodeRequest",
                object([]),
                "generator seed must be a string",
            )
        })?;
    parse_hex_u64(
        seed,
        "generator seed",
        self_observation_value(identity),
    )?;
    Ok(())
}

fn decode_request_case(
    value: &Value,
    context: &AdapterContext,
) -> Result<GeneratedRequestCase, AdapterFailure> {
    let case_object = value.as_object().ok_or_else(|| {
        AdapterFailure::new(
            "invalidRequest",
            "decodeRequestCase",
            object([]),
            "case must be an object",
        )
    })?;
    require_exact_fields(
        case_object,
        &[
            "caseId",
            "domainCaseIndex",
            "family",
            "globalCaseIndex",
            "requestedObservationKind",
            "selectors",
        ],
        "decodeRequestCase",
        object([]),
    )?;
    let case_id = case_id_field(value, "caseId")?;
    let global_case_index = case_id_field(value, "globalCaseIndex")?;
    parse_hex_u64(&global_case_index, "globalCaseIndex", object([]))?;
    if case_id != format!("case-{global_case_index}") {
        return Err(AdapterFailure::new(
            "invalidRequest",
            "decodeRequestCase",
            object([]),
            "caseId must correspond to globalCaseIndex",
        ));
    }
    let family = case_id_field(value, "family")?;
    let declaration = declaration_for(&context.projection, &family).ok_or_else(|| {
        AdapterFailure::new(
            "unsupportedProjection",
            "decodeRequestCase",
            object([]),
            "family has no ProjectionFamilyDeclaration",
        )
    })?;
    let requested_observation_kind = case_id_field(value, "requestedObservationKind")?;
    if !declaration
        .requested_observation_kinds
        .iter()
        .any(|kind| kind == &requested_observation_kind)
    {
        return Err(AdapterFailure::new(
            "unsupportedProjection",
            "decodeRequestCase",
            object([]),
            "requestedObservationKind is not declared by family",
        ));
    }
    let selectors = decode_selectors(
        case_object.get("selectors"),
        &case_id,
        &family,
        &requested_observation_kind,
        context,
    )?;
    Ok(GeneratedRequestCase {
        case_id,
        family,
        requested_observation_kind,
        selectors,
    })
}

fn case_id_field(value: &Value, field: &str) -> Result<String, AdapterFailure> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            AdapterFailure::new(
                "invalidRequest",
                "decodeRequestCase",
                object([]),
                "request case field must be a string",
            )
        })
}

fn decode_selectors(
    value: Option<&Value>,
    case_id: &str,
    family: &str,
    requested_observation_kind: &str,
    context: &AdapterContext,
) -> Result<Selectors, AdapterFailure> {
    let selector_object = value.and_then(Value::as_object).ok_or_else(|| {
        AdapterFailure::new(
            "invalidRequest",
            "decodeSelectors",
            object([]),
            "selectors must be an object",
        )
    })?;
    require_exact_fields(
        selector_object,
        &[
            "value",
            "familyParameter",
            "observationClass",
            "sign",
            "multiplicity",
            "intervalWidth",
            "formulaVariant",
            "mutation",
            "reserved",
        ],
        "decodeSelectors",
        object([("caseId", Value::String(case_id.to_owned()))]),
    )?;
    let parse = |name: &str| -> Result<u64, AdapterFailure> {
        let text = selector_object
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AdapterFailure::new(
                    "invalidRequest",
                    "decodeSelectors",
                    object([]),
                    "selector must be a lowercase hexadecimal u64",
                )
            })?;
        parse_hex_u64(
            text,
            name,
            object([
                ("caseId", Value::String(case_id.to_owned())),
                ("family", Value::String(family.to_owned())),
                (
                    "requestedObservationKind",
                    Value::String(requested_observation_kind.to_owned()),
                ),
                (
                    "projectionRevision",
                    Value::String(context.identity.projection_digest.clone()),
                ),
                (
                    "resourceRevision",
                    Value::String(context.resource.revision.clone()),
                ),
                (
                    "rustcIdentity",
                    Value::String(context.identity.rustc_identity.clone()),
                ),
                (
                    "targetTriple",
                    Value::String(context.identity.target_triple.clone()),
                ),
            ]),
        )
    };
    Ok(Selectors {
        value: parse("value")?,
        family_parameter: parse("familyParameter")?,
        observation_class: parse("observationClass")?,
        sign: parse("sign")?,
        multiplicity: parse("multiplicity")?,
        interval_width: parse("intervalWidth")?,
        formula_variant: parse("formulaVariant")?,
        mutation: parse("mutation")?,
        reserved: parse("reserved")?,
    })
}

fn declaration_for<'a>(
    catalog: &'a ProjectionDeclarationCatalog,
    family: &str,
) -> Option<&'a ProjectionFamilyDeclaration> {
    catalog
        .families
        .iter()
        .find(|declaration| declaration.family == family)
}

fn implementation_for(family: &str) -> Option<&'static ProjectionImplementation> {
    PROJECTION_IMPLEMENTATIONS
        .iter()
        .find(|implementation| implementation.family == family)
}

fn selected_projected_condition(
    declaration: &ProjectionFamilyDeclaration,
    selector: &str,
    selector_value: u64,
) -> Result<String, AdapterFailure> {
    declaration
        .bindings
        .iter()
        .find(|binding| {
            binding.selector == selector && binding.selector_rule.selector == selector
        })
        .and_then(|binding| {
            selector_value
                .checked_rem(binding.selector_rule.modulus)
                .and_then(|remainder| {
                    binding
                        .selector_rule
                        .buckets_by_remainder
                        .get(&remainder.to_string())
                })
                .and_then(|name| {
                    binding
                        .buckets
                        .iter()
                        .find(|bucket| bucket.name == *name)
                })
        })
        .map(|bucket| bucket.projected_condition.clone())
        .ok_or_else(|| {
            AdapterFailure::new(
                "unsupportedProjection",
                "projectSelectors",
                object([]),
                "projection declaration has no selected bucket",
            )
        })
}

impl GeneratedRequestCase {
    fn provenance(&self, context: &AdapterContext) -> Value {
        object([
            (
                "rustcIdentity",
                Value::String(context.identity.rustc_identity.clone()),
            ),
            ("caseId", Value::String(self.case_id.clone())),
            ("family", Value::String(self.family.clone())),
            (
                "projectionRevision",
                Value::String(context.identity.projection_digest.clone()),
            ),
            (
                "requestedObservationKind",
                Value::String(self.requested_observation_kind.clone()),
            ),
            (
                "resourceRevision",
                Value::String(context.resource.revision.clone()),
            ),
            ("selectors", self.selectors.canonical_value()),
            (
                "targetTriple",
                Value::String(context.identity.target_triple.clone()),
            ),
        ])
    }

    fn requests_failure(&self, declaration: &ProjectionFamilyDeclaration) -> bool {
        self.requested_observation_kind == declaration.typed_failure
    }
}

fn project_request_case(
    case: &GeneratedRequestCase,
    context: &AdapterContext,
) -> Result<Value, AdapterFailure> {
    let declaration = declaration_for(&context.projection, &case.family).ok_or_else(|| {
        AdapterFailure::new(
            "unsupportedProjection",
            "projectRequestCase",
            case.provenance(context),
            "family has no ProjectionFamilyDeclaration",
        )
    })?;
    let implementation = implementation_for(&declaration.family).ok_or_else(|| {
        AdapterFailure::new(
            "unsupportedProjection",
            "projectRequestCase",
            case.provenance(context),
            "family has no ProjectionImplementation",
        )
    })?;
    let invocation = (implementation.project)(case, declaration, context)?;
    validate_projected_conditions(case, declaration, &invocation, context)?;
    let outcome = invoke_public_api(&invocation, case, context)?;
    observe_product_outcome(
        case,
        implementation,
        declaration,
        &invocation,
        outcome,
        context,
    )
}

fn validate_projected_conditions(
    case: &GeneratedRequestCase,
    declaration: &ProjectionFamilyDeclaration,
    invocation: &ProductInvocation,
    context: &AdapterContext,
) -> Result<(), AdapterFailure> {
    let actual = invocation.projected_conditions();
    if actual.len() != declaration.bindings.len() {
        return Err(AdapterFailure::new(
            "unsupportedProjection",
            "projectSelectors",
            case.provenance(context),
            "projected condition count does not match ProjectionFamilyDeclaration",
        ));
    }
    for binding in &declaration.bindings {
        let selector = case.selectors.value_for(&binding.selector).ok_or_else(|| {
            AdapterFailure::new(
                "unsupportedProjection",
                "projectSelectors",
                case.provenance(context),
                "ProjectionFamilyDeclaration names an unknown selector",
            )
        })?;
        let selected = binding
            .buckets
            .get(bucket(selector, binding.buckets.len()))
            .ok_or_else(|| {
                AdapterFailure::new(
                    "unsupportedProjection",
                    "projectSelectors",
                    case.provenance(context),
                    "selector bucket is absent",
                )
            })?;
        if actual.get(binding.axis.as_str()).map(String::as_str)
            != Some(selected.projected_condition.as_str())
        {
            return Err(AdapterFailure::new(
                "unsupportedProjection",
                "projectSelectors",
                case.provenance(context),
                "ProductInvocation arguments do not satisfy projectedCondition",
            )
            .with_witness(object([
                ("axis", Value::String(binding.axis.clone())),
                ("bucket", Value::String(selected.name.clone())),
                (
                    "projectedCondition",
                    Value::String(selected.projected_condition.clone()),
                ),
            ])));
        }
    }
    Ok(())
}

fn observe_product_outcome(
    case: &GeneratedRequestCase,
    implementation: &ProjectionImplementation,
    declaration: &ProjectionFamilyDeclaration,
    invocation: &ProductInvocation,
    outcome: ProductOutcome,
    context: &AdapterContext,
) -> Result<Value, AdapterFailure> {
    let observation_outcome = if case.requests_failure(declaration) {
        match outcome {
            ProductOutcome::Failure(product_failure)
                if expected_typed_failure(declaration, &product_failure) =>
            {
                ObservationOutcome::Failure(declaration.typed_failure.to_owned())
            }
            ProductOutcome::Failure(product_failure) => {
                return Err(unexpected_product_failure(
                    case,
                    context,
                    product_failure,
                    "public API returned a different typed failure",
                ));
            }
            ProductOutcome::Success(success_value) => {
                return Err(AdapterFailure::new(
                    "unexpectedPublicApiOutcome",
                    "observeProductOutcome",
                    case.provenance(context),
                    "public API succeeded when the declared typed failure was requested",
                )
                .with_witness(object([
                    (
                        "invocationVariant",
                        Value::String(invocation.arguments.kind().to_owned()),
                    ),
                    (
                        "outcomeVariant",
                        Value::String(success_value.kind().to_owned()),
                    ),
                ])));
            }
        }
    } else {
        match outcome {
            ProductOutcome::Success(success_value) => ObservationOutcome::Success(
                encode_product_success(case, invocation, *success_value, context)?,
            ),
            ProductOutcome::Failure(product_failure) => {
                return Err(unexpected_product_failure(
                    case,
                    context,
                    product_failure,
                    "public API returned a failure when success was requested",
                ));
            }
        }
    };
    Ok(CanonicalObservation::new(
        case.case_id.clone(),
        implementation.family,
        invocation.input_value(),
        observation_outcome,
    )
    .into_value())
}

fn unexpected_product_failure(
    case: &GeneratedRequestCase,
    context: &AdapterContext,
    failure: ProductFailure,
    violated_condition: &str,
) -> AdapterFailure {
    let kind = match failure.stage {
        ProductFailureStage::Construction => "publicApiConstructionFailure",
        ProductFailureStage::Execution => "unexpectedPublicApiOutcome",
    };
    AdapterFailure::new(
        kind,
        "invokePublicApi",
        case.provenance(context),
        violated_condition,
    )
    .with_contributing_failure(&failure)
}

fn expected_typed_failure(
    declaration: &ProjectionFamilyDeclaration,
    failure: &ProductFailure,
) -> bool {
    match (declaration.family.as_str(), &failure.error) {
        ("polynomial-exact", ProductError::Polynomial(PolynomialFailure::EmptyCoefficients)) => {
            true
        }
        (
            "polynomial-enclosure",
            ProductError::Calculus(CalculusFailure::Polynomial(PolynomialFailure::Resource(
                PolynomialResourceFailure::PrecisionLimitExceeded { requested, maximum },
            ))),
        ) => *requested > *maximum && *maximum == MAX_PRECISION_BITS,
        (
            "rational-exact",
            ProductError::Calculus(CalculusFailure::Rational(RationalFailure::UndefinedDomain)),
        ) => true,
        (
            "rational-interval",
            ProductError::Calculus(CalculusFailure::Rational(RationalFailure::DomainContainsZero)),
        ) => true,
        ("root-bracket", ProductError::Root(RootFailure::NoSignChange)) => true,
        (
            "bisection",
            ProductError::Calculus(CalculusFailure::Root(RootFailure::ResourceLimit {
                resource: neco_root::RootResource::RefinementIterations,
                requested,
                maximum,
            })),
        ) => *requested > *maximum,
        ("even-touch", ProductError::Root(RootFailure::NoEvenMultiplicityTouch)) => true,
        (
            "local-region-transition",
            ProductError::Calculus(CalculusFailure::Root(RootFailure::PointOutsideEnclosure)),
        ) => true,
        (
            "surface-exact",
            ProductError::Calculus(CalculusFailure::Rational(RationalFailure::UndefinedDomain)),
        ) => true,
        ("surface-checked", ProductError::Calculus(CalculusFailure::Evaluation(failure))) => {
            matches!(**failure, EvaluationFailure::DomainViolation { .. })
        }
        _ => false,
    }
}

fn invoke_public_api(
    invocation: &ProductInvocation,
    case: &GeneratedRequestCase,
    context: &AdapterContext,
) -> Result<ProductOutcome, AdapterFailure> {
    let outcome = match &invocation.arguments {
        ProductArguments::PolynomialExact(arguments) => {
            let polynomial = match construct_polynomial(&arguments.coefficients) {
                Ok(polynomial) => polynomial,
                Err(failure) => return Ok(ProductOutcome::Failure(failure)),
            };
            let point = match construct_algebraic(arguments.point) {
                Ok(point) => point,
                Err(failure) => return Ok(ProductOutcome::Failure(failure)),
            };
            match evaluate_polynomial(&polynomial, &point) {
                Ok(value) => ProductOutcome::success(ProductSuccess::PolynomialExact(value)),
                Err(error) => ProductOutcome::Failure(ProductFailure::execution(
                    ProductError::Calculus(error),
                )),
            }
        }
        ProductArguments::PolynomialEnclosure(arguments) => {
            let polynomial = match construct_polynomial(&arguments.coefficients) {
                Ok(polynomial) => polynomial,
                Err(failure) => return Ok(ProductOutcome::Failure(failure)),
            };
            let enclosure = match construct_integer_enclosure(arguments.lower, arguments.upper) {
                Ok(enclosure) => enclosure,
                Err(failure) => return Ok(ProductOutcome::Failure(failure)),
            };
            match evaluate_interval(&polynomial, &enclosure, arguments.precision_bits) {
                Ok(value) => ProductOutcome::success(ProductSuccess::PolynomialEnclosure(value)),
                Err(error) => ProductOutcome::Failure(ProductFailure::execution(
                    ProductError::Calculus(error),
                )),
            }
        }
        ProductArguments::RationalExact(arguments) => {
            let function =
                match construct_rational_function(&arguments.numerator, &arguments.denominator) {
                    Ok(function) => function,
                    Err(failure) => return Ok(ProductOutcome::Failure(failure)),
                };
            let point = match construct_algebraic(arguments.point) {
                Ok(point) => point,
                Err(failure) => return Ok(ProductOutcome::Failure(failure)),
            };
            match evaluate_rational(&function, &point) {
                Ok(value) => ProductOutcome::success(ProductSuccess::RationalExact(value)),
                Err(error) => ProductOutcome::Failure(ProductFailure::execution(
                    ProductError::Calculus(error),
                )),
            }
        }
        ProductArguments::RationalInterval(arguments) => {
            let function =
                match construct_rational_function(&arguments.numerator, &arguments.denominator) {
                    Ok(function) => function,
                    Err(failure) => return Ok(ProductOutcome::Failure(failure)),
                };
            let enclosure = match construct_integer_enclosure(arguments.lower, arguments.upper) {
                Ok(enclosure) => enclosure,
                Err(failure) => return Ok(ProductOutcome::Failure(failure)),
            };
            match evaluate_interval(&function, &enclosure, arguments.precision_bits) {
                Ok(value) => ProductOutcome::success(ProductSuccess::RationalInterval(value)),
                Err(error) => ProductOutcome::Failure(ProductFailure::execution(
                    ProductError::Calculus(error),
                )),
            }
        }
        ProductArguments::RootBracket(arguments) => {
            match construct_root_bracket(&arguments.coefficients, arguments.lower, arguments.upper)
            {
                Ok(bracket) => ProductOutcome::success(ProductSuccess::RootBracket(bracket)),
                Err(failure) => ProductOutcome::Failure(failure),
            }
        }
        ProductArguments::Bisection(arguments) => {
            match invoke_bisection(arguments, case, context)? {
                Ok(value) => ProductOutcome::success(ProductSuccess::Bisection(value)),
                Err(failure) => ProductOutcome::Failure(failure),
            }
        }
        ProductArguments::EvenTouch(arguments) => {
            let polynomial = match construct_polynomial(&arguments.coefficients) {
                Ok(polynomial) => polynomial,
                Err(failure) => return Ok(ProductOutcome::Failure(failure)),
            };
            let enclosure = match construct_dyadic_enclosure(arguments.lower, arguments.upper) {
                Ok(enclosure) => enclosure,
                Err(failure) => return Ok(ProductOutcome::Failure(failure)),
            };
            let candidate = match construct_dyadic(arguments.candidate) {
                Ok(candidate) => candidate,
                Err(failure) => return Ok(ProductOutcome::Failure(failure)),
            };
            match RootBracket::even_multiplicity_touch(
                polynomial,
                enclosure,
                candidate,
                neco_calculus::DerivativeOrder::new(arguments.max_order),
            ) {
                Ok(bracket) => ProductOutcome::success(ProductSuccess::EvenTouch(bracket)),
                Err(error) => {
                    ProductOutcome::Failure(ProductFailure::execution(ProductError::Root(error)))
                }
            }
        }
        ProductArguments::LocalRegionTransition(arguments) => {
            let request = match construct_membership_request(arguments) {
                Ok(request) => request,
                Err(failure) => return Ok(ProductOutcome::Failure(failure)),
            };
            let sign = match expose_region_sign(&request) {
                Ok(sign) => sign,
                Err(error) => {
                    return Ok(ProductOutcome::Failure(ProductFailure::execution(
                        ProductError::Calculus(error),
                    )));
                }
            };
            match expose_membership_transitions(&request) {
                Ok(transitions) => ProductOutcome::success(ProductSuccess::LocalRegionTransition(
                    LocalRegionProductOutcome { sign, transitions },
                )),
                Err(error) => ProductOutcome::Failure(ProductFailure::execution(
                    ProductError::Calculus(error),
                )),
            }
        }
        ProductArguments::SurfaceExact(arguments) => {
            let input = match construct_surface_exact_input(arguments) {
                Ok(input) => input,
                Err(failure) => return Ok(ProductOutcome::Failure(failure)),
            };
            match evaluate_surface(input) {
                Ok(value) => ProductOutcome::success(ProductSuccess::SurfaceExact(value)),
                Err(error) => ProductOutcome::Failure(ProductFailure::execution(
                    ProductError::Calculus(error),
                )),
            }
        }
        ProductArguments::SurfaceChecked(arguments) => {
            let input = match construct_surface_checked_input(arguments) {
                Ok(input) => input,
                Err(failure) => return Ok(ProductOutcome::Failure(failure)),
            };
            match evaluate_surface_checked(input) {
                Ok(value) => ProductOutcome::success(ProductSuccess::SurfaceChecked(value)),
                Err(error) => ProductOutcome::Failure(ProductFailure::execution(
                    ProductError::Calculus(error),
                )),
            }
        }
    };
    Ok(outcome)
}

fn invoke_bisection(
    arguments: &BisectionArguments,
    case: &GeneratedRequestCase,
    context: &AdapterContext,
) -> Result<Result<BisectionProductOutcome, ProductFailure>, AdapterFailure> {
    let aggregate_bracket =
        match construct_root_bracket(&arguments.coefficients, arguments.lower, arguments.upper) {
            Ok(bracket) => bracket,
            Err(failure) => return Ok(Err(failure)),
        };
    let aggregate = match refine_root_bisection(&aggregate_bracket, arguments.requested_iterations)
    {
        Ok(isolation) => isolation,
        Err(error) => {
            return Ok(Err(ProductFailure::execution(ProductError::Calculus(
                error,
            ))));
        }
    };
    let mut current_lower = match aggregate_bracket.enclosure().lower().try_clone() {
        Ok(value) => value,
        Err(error) => {
            return Ok(Err(ProductFailure::execution(ProductError::Calculus(
                CalculusFailure::Arithmetic(error),
            ))));
        }
    };
    let mut current_upper = match aggregate_bracket.enclosure().upper().try_clone() {
        Ok(value) => value,
        Err(error) => {
            return Ok(Err(ProductFailure::execution(ProductError::Calculus(
                CalculusFailure::Arithmetic(error),
            ))));
        }
    };
    let mut trace = Vec::new();
    for _ in 0..arguments.requested_iterations {
        let step_polynomial = match construct_polynomial(&arguments.coefficients) {
            Ok(polynomial) => polynomial,
            Err(failure) => return Ok(Err(failure)),
        };
        let step_enclosure = match Enclosure::new(
            match current_lower.try_clone() {
                Ok(value) => value,
                Err(error) => {
                    return Ok(Err(ProductFailure::execution(ProductError::Calculus(
                        CalculusFailure::Arithmetic(error),
                    ))));
                }
            },
            match current_upper.try_clone() {
                Ok(value) => value,
                Err(error) => {
                    return Ok(Err(ProductFailure::execution(ProductError::Calculus(
                        CalculusFailure::Arithmetic(error),
                    ))));
                }
            },
        ) {
            Ok(enclosure) => enclosure,
            Err(error) => {
                return Ok(Err(ProductFailure::construction(ProductError::Calculus(
                    CalculusFailure::Interval(error),
                ))));
            }
        };
        let midpoint = match step_enclosure.midpoint() {
            Ok(midpoint) => midpoint,
            Err(error) => {
                return Ok(Err(ProductFailure::execution(ProductError::Calculus(
                    CalculusFailure::Interval(error),
                ))));
            }
        };
        let midpoint_algebraic = match dyadic_as_algebraic(&midpoint) {
            Ok(value) => value,
            Err(failure) => return Ok(Err(failure)),
        };
        let midpoint_sign = match evaluate_polynomial(&step_polynomial, &midpoint_algebraic) {
            Ok(value) => match value.value().sign() {
                Ok(sign) => sign,
                Err(error) => {
                    return Ok(Err(ProductFailure::execution(ProductError::Calculus(
                        CalculusFailure::Algebra(error),
                    ))));
                }
            },
            Err(error) => {
                return Ok(Err(ProductFailure::execution(ProductError::Calculus(
                    error,
                ))));
            }
        };
        let step_bracket = match RootBracket::sign_change(step_polynomial, step_enclosure) {
            Ok(bracket) => bracket,
            Err(error) => {
                return Ok(Err(ProductFailure::execution(ProductError::Root(error))));
            }
        };
        let step_isolation = match refine_root_bisection(&step_bracket, 1) {
            Ok(isolation) => isolation,
            Err(error) => {
                return Ok(Err(ProductFailure::execution(ProductError::Calculus(
                    error,
                ))));
            }
        };
        let updated = step_isolation.enclosure();
        let selection = if updated.lower() == &midpoint && updated.upper() == &midpoint {
            "exact-root"
        } else if updated.lower() == &midpoint {
            "upper-half"
        } else if updated.upper() == &midpoint {
            "lower-half"
        } else {
            return Err(AdapterFailure::new(
                "unexpectedPublicApiOutcome",
                "invokeBisectionStep",
                case.provenance(context),
                "one-step public API result is not a bisection half",
            ));
        };
        current_lower = updated.lower().try_clone().map_err(|error| {
            AdapterFailure::new(
                "unexpectedPublicApiOutcome",
                "invokeBisectionStep",
                case.provenance(context),
                format!("cannot retain updated lower endpoint: {error:?}"),
            )
        })?;
        current_upper = updated.upper().try_clone().map_err(|error| {
            AdapterFailure::new(
                "unexpectedPublicApiOutcome",
                "invokeBisectionStep",
                case.provenance(context),
                format!("cannot retain updated upper endpoint: {error:?}"),
            )
        })?;
        let updated_enclosure = Enclosure::new(
            current_lower.try_clone().map_err(|error| {
                AdapterFailure::new(
                    "unexpectedPublicApiOutcome",
                    "invokeBisectionStep",
                    case.provenance(context),
                    format!("cannot retain trace lower endpoint: {error:?}"),
                )
            })?,
            current_upper.try_clone().map_err(|error| {
                AdapterFailure::new(
                    "unexpectedPublicApiOutcome",
                    "invokeBisectionStep",
                    case.provenance(context),
                    format!("cannot retain trace upper endpoint: {error:?}"),
                )
            })?,
        )
        .map_err(|error| {
            AdapterFailure::new(
                "unexpectedPublicApiOutcome",
                "invokeBisectionStep",
                case.provenance(context),
                format!("cannot retain trace enclosure: {error:?}"),
            )
        })?;
        trace.push(BisectionStepOutcome {
            midpoint,
            midpoint_sign,
            selection,
            updated_enclosure,
        });
        if selection == "exact-root" {
            break;
        }
    }
    let witness = aggregate.witness();
    let final_matches = trace
        .last()
        .is_some_and(|step| step.updated_enclosure == *aggregate.enclosure());
    if witness.iterations() != trace.len() || !final_matches {
        return Err(AdapterFailure::new(
            "unexpectedPublicApiOutcome",
            "invokeBisection",
            case.provenance(context),
            "aggregate and repeated one-step public API results differ",
        )
        .with_witness(object([
            (
                "aggregateIterations",
                Value::Number(witness.iterations().into()),
            ),
            ("traceIterations", Value::Number(trace.len().into())),
        ])));
    }
    Ok(Ok(BisectionProductOutcome { aggregate, trace }))
}

fn encode_product_success(
    case: &GeneratedRequestCase,
    invocation: &ProductInvocation,
    success: ProductSuccess,
    context: &AdapterContext,
) -> Result<Value, AdapterFailure> {
    match (&invocation.arguments, success) {
        (ProductArguments::PolynomialExact(_), ProductSuccess::PolynomialExact(value)) => Ok(
            object([("value", exact_integer_value(value.value(), case, context)?)]),
        ),
        (ProductArguments::PolynomialEnclosure(_), ProductSuccess::PolynomialEnclosure(value)) => {
            Ok(encode_enclosure(value.enclosure()))
        }
        (ProductArguments::RationalExact(_), ProductSuccess::RationalExact(value)) => {
            Ok(object([(
                "value",
                exact_integer_value(value.value(), case, context)?,
            )]))
        }
        (ProductArguments::RationalInterval(_), ProductSuccess::RationalInterval(value)) => {
            Ok(encode_enclosure(value.enclosure()))
        }
        (ProductArguments::RootBracket(_), ProductSuccess::RootBracket(bracket)) => {
            match bracket.evidence() {
                RootEvidence::SignChange { .. } => Ok(object([(
                    "evidence",
                    Value::String("sign-change".to_owned()),
                )])),
                RootEvidence::EvenMultiplicityTouch { .. } => Err(AdapterFailure::new(
                    "unexpectedPublicApiOutcome",
                    "encodeRootBracket",
                    case.provenance(context),
                    "root bracket returned non-sign-change evidence",
                )),
            }
        }
        (ProductArguments::Bisection(_), ProductSuccess::Bisection(value)) => {
            encode_bisection(value, case, context)
        }
        (ProductArguments::EvenTouch(_), ProductSuccess::EvenTouch(bracket)) => {
            match bracket.evidence() {
                RootEvidence::EvenMultiplicityTouch { multiplicity, .. } => {
                    let order = match multiplicity {
                        RootMultiplicity::Simple => 1,
                        RootMultiplicity::Multiple(order) => order.get(),
                    };
                    Ok(object([("multiplicity", Value::Number(order.into()))]))
                }
                RootEvidence::SignChange { .. } => Err(AdapterFailure::new(
                    "unexpectedPublicApiOutcome",
                    "encodeEvenTouch",
                    case.provenance(context),
                    "even-touch call returned sign-change evidence",
                )),
            }
        }
        (
            ProductArguments::LocalRegionTransition(_),
            ProductSuccess::LocalRegionTransition(value),
        ) => Ok(encode_local_region(value)),
        (ProductArguments::SurfaceExact(_), ProductSuccess::SurfaceExact(value)) => {
            encode_surface_exact(value, case, context)
        }
        (ProductArguments::SurfaceChecked(_), ProductSuccess::SurfaceChecked(value)) => {
            encode_surface_checked(value, case, context)
        }
        (arguments, success_value) => Err(AdapterFailure::new(
            "unexpectedPublicApiOutcome",
            "encodeProductOutcome",
            case.provenance(context),
            "ProductOutcome variant does not match ProductInvocation",
        )
        .with_witness(object([
            (
                "invocationVariant",
                Value::String(arguments.kind().to_owned()),
            ),
            (
                "outcomeVariant",
                Value::String(success_value.kind().to_owned()),
            ),
        ]))),
    }
}

fn encode_bisection(
    value: BisectionProductOutcome,
    case: &GeneratedRequestCase,
    context: &AdapterContext,
) -> Result<Value, AdapterFailure> {
    let witness = value.aggregate.witness();
    if witness.method() != RootRefinementMethod::Bisection {
        return Err(AdapterFailure::new(
            "unexpectedPublicApiOutcome",
            "encodeBisection",
            case.provenance(context),
            "root refinement method is not bisection",
        ));
    }
    let status = match witness.status() {
        ConvergenceStatus::ExactDyadicRoot => "exact-dyadic-root",
        ConvergenceStatus::Refined => "refined",
        ConvergenceStatus::IterationLimit => {
            return Err(AdapterFailure::new(
                "unexpectedPublicApiOutcome",
                "encodeBisection",
                case.provenance(context),
                "successful positive-iteration bisection retained iteration-limit status",
            ));
        }
    };
    let trace = value
        .trace
        .iter()
        .map(|step| {
            object([
                ("midpoint", encode_dyadic(&step.midpoint)),
                (
                    "midpointSign",
                    Value::String(sign_name(step.midpoint_sign).to_owned()),
                ),
                ("selection", Value::String(step.selection.to_owned())),
                (
                    "updatedLower",
                    encode_dyadic(step.updated_enclosure.lower()),
                ),
                (
                    "updatedUpper",
                    encode_dyadic(step.updated_enclosure.upper()),
                ),
            ])
        })
        .collect();
    Ok(object([
        (
            "finalLower",
            encode_dyadic(value.aggregate.enclosure().lower()),
        ),
        (
            "finalUpper",
            encode_dyadic(value.aggregate.enclosure().upper()),
        ),
        ("iterations", Value::Number(witness.iterations().into())),
        ("method", Value::String("bisection".to_owned())),
        ("status", Value::String(status.to_owned())),
        ("trace", Value::Array(trace)),
    ]))
}

fn encode_local_region(value: LocalRegionProductOutcome) -> Value {
    let sign = match value.sign.sign() {
        RegionSign::ConstantPositive => "constant-positive",
        RegionSign::ConstantNegative => "constant-negative",
        RegionSign::ConstantZero => "constant-zero",
        RegionSign::SignVaries => "varies",
        RegionSign::ContainsZero => "contains-zero",
        RegionSign::Undetermined => "undetermined",
    };
    let loci = value
        .transitions
        .loci()
        .iter()
        .map(|locus| {
            let kind = match locus.kind() {
                MembershipLocusKind::Transition => "transition",
                MembershipLocusKind::SingularContact => "singular-contact",
            };
            let multiplicity = match locus.multiplicity() {
                RootMultiplicity::Simple => 1,
                RootMultiplicity::Multiple(order) => order.get(),
            };
            object([
                ("kind", Value::String(kind.to_owned())),
                ("multiplicity", Value::Number(multiplicity.into())),
                ("pointIndex", Value::Number(locus.point_index().into())),
            ])
        })
        .collect::<Vec<_>>();
    object([
        ("loci", Value::Array(loci.clone())),
        ("locusCount", Value::Number(loci.len().into())),
        ("sign", Value::String(sign.to_owned())),
    ])
}

fn encode_surface_exact(
    value: SurfaceEvaluationProducer,
    case: &GeneratedRequestCase,
    context: &AdapterContext,
) -> Result<Value, AdapterFailure> {
    match value {
        SurfaceEvaluationProducer::Exact { x, y, z } => Ok(object([
            ("x", exact_integer_value(&x, case, context)?),
            ("y", exact_integer_value(&y, case, context)?),
            ("z", exact_integer_value(&z, case, context)?),
        ])),
        SurfaceEvaluationProducer::Certified { .. } => Err(AdapterFailure::new(
            "unexpectedPublicApiOutcome",
            "encodeSurfaceExact",
            case.provenance(context),
            "exact surface evaluation returned certified coordinates",
        )),
    }
}

fn encode_surface_checked(
    value: SurfaceEvaluationProducer,
    case: &GeneratedRequestCase,
    context: &AdapterContext,
) -> Result<Value, AdapterFailure> {
    match value {
        SurfaceEvaluationProducer::Certified { coordinates } => Ok(object([
            (
                "provenance",
                encode_provenance(
                    coordinates.owner(),
                    coordinates.requirement(),
                    coordinates.source(),
                ),
            ),
            ("x", encode_enclosure(coordinates.x())),
            ("y", encode_enclosure(coordinates.y())),
            ("z", encode_enclosure(coordinates.z())),
        ])),
        SurfaceEvaluationProducer::Exact { .. } => Err(AdapterFailure::new(
            "unexpectedPublicApiOutcome",
            "encodeSurfaceChecked",
            case.provenance(context),
            "checked surface evaluation returned exact coordinates",
        )),
    }
}

fn exact_integer_value(
    value: &RealAlgebraic,
    case: &GeneratedRequestCase,
    context: &AdapterContext,
) -> Result<Value, AdapterFailure> {
    let coefficients = value.minimal_polynomial().polynomial().coefficients();
    if coefficients.len() != 2
        || coefficients[1].sign() != Sign::Positive
        || coefficients[1].magnitude().limbs_le() != [1]
    {
        return Err(AdapterFailure::new(
            "unexpectedPublicApiOutcome",
            "encodeExactInteger",
            case.provenance(context),
            "public API exact result is not an integer",
        ));
    }
    let constant = bigint_decimal(&coefficients[0]);
    let integer = if constant == "0" {
        constant
    } else if let Some(magnitude) = constant.strip_prefix('-') {
        magnitude.to_owned()
    } else {
        format!("-{constant}")
    };
    Ok(Value::String(integer))
}

fn bigint_decimal(value: &BigInt) -> String {
    const DECIMAL_BASE: u64 = 1_000_000_000;
    if value.sign() == Sign::Zero {
        return "0".to_owned();
    }
    let mut digits = Vec::new();
    for limb in value.magnitude().limbs_le().iter().rev() {
        let mut carry = u64::from(*limb);
        for digit in &mut digits {
            let expanded = u64::from(*digit) * (1_u64 << 32) + carry;
            *digit = (expanded % DECIMAL_BASE) as u32;
            carry = expanded / DECIMAL_BASE;
        }
        while carry > 0 {
            digits.push((carry % DECIMAL_BASE) as u32);
            carry /= DECIMAL_BASE;
        }
    }
    let Some(first) = digits.last() else {
        return "0".to_owned();
    };
    let mut output = if value.sign() == Sign::Negative {
        "-".to_owned()
    } else {
        String::new()
    };
    output.push_str(&first.to_string());
    for digit in digits.iter().rev().skip(1) {
        output.push_str(&format!("{digit:09}"));
    }
    output
}

fn construct_polynomial(coefficients: &[i64]) -> Result<Polynomial, ProductFailure> {
    let algebraic_coefficients = coefficients
        .iter()
        .map(|coefficient| construct_algebraic(*coefficient))
        .collect::<Result<Vec<_>, _>>()?;
    Polynomial::from_coefficients(algebraic_coefficients)
        .map_err(|error| ProductFailure::construction(ProductError::Polynomial(error)))
}

fn construct_algebraic(value: i64) -> Result<RealAlgebraic, ProductFailure> {
    let integer = BigInt::try_from(value).map_err(|error| {
        ProductFailure::construction(ProductError::Calculus(CalculusFailure::Arithmetic(error)))
    })?;
    RealAlgebraic::from_integer(integer).map_err(|error| {
        ProductFailure::construction(ProductError::Calculus(CalculusFailure::Algebra(error)))
    })
}

fn construct_dyadic(value: DyadicArgument) -> Result<Dyadic, ProductFailure> {
    let integer = BigInt::try_from(value.integer).map_err(|error| {
        ProductFailure::construction(ProductError::Calculus(CalculusFailure::Arithmetic(error)))
    })?;
    Ok(Dyadic::new(integer, value.exponent))
}

fn construct_integer_enclosure(lower: i64, upper: i64) -> Result<Enclosure, ProductFailure> {
    construct_dyadic_enclosure(
        DyadicArgument {
            integer: lower,
            exponent: 0,
        },
        DyadicArgument {
            integer: upper,
            exponent: 0,
        },
    )
}

fn construct_dyadic_enclosure(
    lower: DyadicArgument,
    upper: DyadicArgument,
) -> Result<Enclosure, ProductFailure> {
    Enclosure::new(construct_dyadic(lower)?, construct_dyadic(upper)?).map_err(|error| {
        ProductFailure::construction(ProductError::Calculus(CalculusFailure::Interval(error)))
    })
}

fn construct_rational_function(
    numerator: &[i64],
    denominator: &[i64],
) -> Result<RationalFunction, ProductFailure> {
    RationalFunction::new(
        construct_polynomial(numerator)?,
        construct_polynomial(denominator)?,
    )
    .map_err(|error| ProductFailure::construction(ProductError::Rational(error)))
}

fn construct_root_bracket(
    coefficients: &[i64],
    lower: i64,
    upper: i64,
) -> Result<RootBracket, ProductFailure> {
    RootBracket::sign_change(
        construct_polynomial(coefficients)?,
        construct_integer_enclosure(lower, upper)?,
    )
    .map_err(|error| ProductFailure::construction(ProductError::Root(error)))
}

fn construct_membership_request(
    arguments: &LocalRegionArguments,
) -> Result<MembershipJudgmentRequest, ProductFailure> {
    let candidate = construct_dyadic(DyadicArgument {
        integer: arguments.candidate,
        exponent: 0,
    })?;
    MembershipJudgmentRequest::new(
        construct_polynomial(&arguments.coefficients)?,
        construct_integer_enclosure(arguments.lower, arguments.upper)?,
        vec![candidate],
        neco_calculus::DerivativeOrder::new(arguments.max_order),
        arguments.precision_bits,
    )
    .map_err(|error| ProductFailure::construction(ProductError::Root(error)))
}

fn construct_surface_polynomial(
    terms: &[SurfaceTermArgument],
) -> Result<SurfacePolynomial, ProductFailure> {
    let product_terms = terms
        .iter()
        .map(|term| {
            let integer = BigInt::try_from(term.coefficient).map_err(|error| {
                ProductFailure::construction(ProductError::Calculus(CalculusFailure::Arithmetic(
                    error,
                )))
            })?;
            let coefficient = ReducedRational::from_bigint(integer).map_err(|error| {
                ProductFailure::construction(ProductError::Calculus(CalculusFailure::Arithmetic(
                    error,
                )))
            })?;
            Ok(SurfaceTerm {
                coefficient,
                exponent_u: term.exponent_u,
                exponent_v: term.exponent_v,
            })
        })
        .collect::<Result<Vec<_>, ProductFailure>>()?;
    SurfacePolynomial::try_from_terms(product_terms)
        .map_err(|error| ProductFailure::construction(ProductError::Calculus(error)))
}

fn construct_surface_polynomials(
    surface: &SurfaceArguments,
) -> Result<RationalSurfacePolynomials, ProductFailure> {
    Ok(RationalSurfacePolynomials {
        x: construct_surface_polynomial(&surface.x)?,
        y: construct_surface_polynomial(&surface.y)?,
        z: construct_surface_polynomial(&surface.z)?,
        denominator: construct_surface_polynomial(&surface.denominator)?,
    })
}

fn construct_surface_exact_input(
    arguments: &SurfaceExactArguments,
) -> Result<SurfaceEvaluationInput, ProductFailure> {
    let surface = construct_surface_polynomials(&arguments.surface)?;
    Ok(SurfaceEvaluationInput {
        x: surface.x,
        y: surface.y,
        z: surface.z,
        denominator: surface.denominator,
        u: construct_algebraic(arguments.u)?,
        v: construct_algebraic(arguments.v)?,
    })
}

fn construct_surface_checked_input(
    arguments: &SurfaceCheckedArguments,
) -> Result<SurfaceEvaluationInput2, ProductFailure> {
    let requirement = ExactExpressionRequirement::new(
        arguments.requirement.consumer_id(),
        arguments.requirement.expressions(),
        arguments.requirement.decision(),
        arguments.requirement.precision(),
    )
    .map_err(|error| ProductFailure::construction(ProductError::Expression(error)))?;
    SurfaceEvaluationInput2::new(
        construct_surface_polynomials(&arguments.surface)?,
        construct_integer_enclosure(arguments.lower_u, arguments.upper_u)?,
        construct_integer_enclosure(arguments.lower_v, arguments.upper_v)?,
        arguments.owner,
        requirement,
        arguments.source,
)
    .map_err(|error| ProductFailure::construction(ProductError::Calculus(error)))
}

fn dyadic_as_algebraic(value: &Dyadic) -> Result<RealAlgebraic, ProductFailure> {
    let denominator = BigUint::one()
        .and_then(|one| one.shl_bits(value.exponent() as usize))
        .map_err(|error| {
            ProductFailure::construction(ProductError::Calculus(CalculusFailure::Arithmetic(error)))
        })?;
    let numerator = value.integer().try_clone().map_err(|error| {
        ProductFailure::construction(ProductError::Calculus(CalculusFailure::Arithmetic(error)))
    })?;
    let rational = RawRational::new(numerator, denominator)
        .reduce()
        .map_err(|error| {
            ProductFailure::construction(ProductError::Calculus(CalculusFailure::Arithmetic(error)))
        })?
        .into_reduced();
    RealAlgebraic::from_reduced_rational(&rational).map_err(|error| {
        ProductFailure::construction(ProductError::Calculus(CalculusFailure::Algebra(error)))
    })
}

fn project_polynomial_exact(
    case: &GeneratedRequestCase,
    declaration: &ProjectionFamilyDeclaration,
    _context: &AdapterContext,
) -> Result<ProductInvocation, AdapterFailure> {
    let formula_index = bucket(case.selectors.formula_variant, 2);
    let formula_condition = selected_projected_condition(
        declaration,
        "formulaVariant",
        case.selectors.formula_variant,
    )?;
    let coefficients = if case.requests_failure(declaration) {
        Vec::new()
    } else if formula_index == 0 {
        vec![0, 0, 1]
    } else {
        vec![1, 0, 1]
    };
    Ok(ProductInvocation {
        arguments: ProductArguments::PolynomialExact(PolynomialExactArguments {
            coefficients,
            point: selected_value(case.selectors.value),
            formula_condition,
        }),
    })
}

fn project_polynomial_enclosure(
    case: &GeneratedRequestCase,
    declaration: &ProjectionFamilyDeclaration,
    _context: &AdapterContext,
) -> Result<ProductInvocation, AdapterFailure> {
    let formula_index = bucket(case.selectors.formula_variant, 2);
    let formula_condition = selected_projected_condition(
        declaration,
        "formulaVariant",
        case.selectors.formula_variant,
    )?;
    let lower = selected_value(case.selectors.value);
    let upper = lower + selected_width(case.selectors.interval_width);
    let selected_precision_bits = selected_precision(case.selectors.family_parameter);
    let precision_bits = if case.requests_failure(declaration) {
        MAX_PRECISION_BITS + selected_precision_bits
    } else {
        selected_precision_bits
    };
    Ok(ProductInvocation {
        arguments: ProductArguments::PolynomialEnclosure(PolynomialEnclosureArguments {
            coefficients: if formula_index == 0 {
                vec![0, 0, 1]
            } else {
                vec![1, 0, 1]
            },
            lower,
            upper,
            precision_bits,
            formula_condition,
        }),
    })
}

fn project_rational_exact(
    case: &GeneratedRequestCase,
    declaration: &ProjectionFamilyDeclaration,
    _context: &AdapterContext,
) -> Result<ProductInvocation, AdapterFailure> {
    let formula_index = bucket(case.selectors.formula_variant, 2);
    let formula_condition = selected_projected_condition(
        declaration,
        "formulaVariant",
        case.selectors.formula_variant,
    )?;
    let point = selected_value(case.selectors.value);
    let (numerator, denominator) = if case.requests_failure(declaration) {
        if formula_index == 0 {
            (vec![2, 1], vec![-point, 1])
        } else {
            (vec![6, 2], vec![-2 * point, 2])
        }
    } else if formula_index == 0 {
        (vec![2, 1], vec![1])
    } else {
        (vec![6, 2], vec![2])
    };
    Ok(ProductInvocation {
        arguments: ProductArguments::RationalExact(RationalExactArguments {
            numerator,
            denominator,
            point,
            formula_condition,
        }),
    })
}

fn project_rational_interval(
    case: &GeneratedRequestCase,
    declaration: &ProjectionFamilyDeclaration,
    _context: &AdapterContext,
) -> Result<ProductInvocation, AdapterFailure> {
    let formula_index = bucket(case.selectors.formula_variant, 2);
    let formula_condition = selected_projected_condition(
        declaration,
        "formulaVariant",
        case.selectors.formula_variant,
    )?;
    let lower = selected_value(case.selectors.value);
    let upper = lower + selected_width(case.selectors.interval_width);
    let (numerator, denominator) = if case.requests_failure(declaration) {
        if formula_index == 0 {
            (vec![2, 1], vec![-lower, 1])
        } else {
            (vec![6, 2], vec![-2 * lower, 2])
        }
    } else if formula_index == 0 {
        (vec![2, 1], vec![1])
    } else {
        (vec![6, 2], vec![2])
    };
    Ok(ProductInvocation {
        arguments: ProductArguments::RationalInterval(RationalIntervalArguments {
            numerator,
            denominator,
            lower,
            upper,
            precision_bits: selected_precision(case.selectors.family_parameter),
            formula_condition,
        }),
    })
}

fn project_root_bracket(
    case: &GeneratedRequestCase,
    declaration: &ProjectionFamilyDeclaration,
    _context: &AdapterContext,
) -> Result<ProductInvocation, AdapterFailure> {
    let formula_index = bucket(case.selectors.formula_variant, 2);
    let formula_condition = selected_projected_condition(
        declaration,
        "formulaVariant",
        case.selectors.formula_variant,
    )?;
    let sign_index = bucket(case.selectors.sign, 2);
    let sign_condition = selected_projected_condition(declaration, "sign", case.selectors.sign)?;
    let root_constant = if formula_index == 0 { 2 } else { 3 };
    let mut coefficients = if case.requests_failure(declaration) {
        if formula_index == 0 {
            vec![0, 0, 1]
        } else {
            vec![1, 0, 1]
        }
    } else {
        vec![-root_constant, 0, 1]
    };
    if sign_index == 1 {
        coefficients
            .iter_mut()
            .for_each(|coefficient| *coefficient = -*coefficient);
    }
    let lower = 1;
    let upper = lower + selected_width(case.selectors.interval_width);
    Ok(ProductInvocation {
        arguments: ProductArguments::RootBracket(RootBracketArguments {
            coefficients,
            lower,
            upper,
            sign_condition,
            formula_condition,
        }),
    })
}

fn project_bisection(
    case: &GeneratedRequestCase,
    declaration: &ProjectionFamilyDeclaration,
    _context: &AdapterContext,
) -> Result<ProductInvocation, AdapterFailure> {
    let formula_index = bucket(case.selectors.formula_variant, 2);
    let formula_condition = selected_projected_condition(
        declaration,
        "formulaVariant",
        case.selectors.formula_variant,
    )?;
    let sign_index = bucket(case.selectors.sign, 2);
    let sign_condition = selected_projected_condition(declaration, "sign", case.selectors.sign)?;
    let iteration_condition = selected_projected_condition(
        declaration,
        "familyParameter",
        case.selectors.family_parameter,
    )?;
    let root_constant = if formula_index == 0 { 2 } else { 3 };
    let mut coefficients = vec![-root_constant, 0, 1];
    if sign_index == 1 {
        coefficients
            .iter_mut()
            .for_each(|coefficient| *coefficient = -*coefficient);
    }
    let lower = 1;
    let upper = lower + selected_width(case.selectors.interval_width);
    let iteration_index = bucket(case.selectors.family_parameter, 3);
    let requested_iterations = if case.requests_failure(declaration) {
        [20, 23, 27][iteration_index]
    } else {
        selected_iterations(case.selectors.family_parameter)
    };
    Ok(ProductInvocation {
        arguments: ProductArguments::Bisection(BisectionArguments {
            coefficients,
            lower,
            upper,
            requested_iterations,
            iteration_condition,
            sign_condition,
            formula_condition,
        }),
    })
}

fn project_even_touch(
    case: &GeneratedRequestCase,
    declaration: &ProjectionFamilyDeclaration,
    _context: &AdapterContext,
) -> Result<ProductInvocation, AdapterFailure> {
    let formula_index = bucket(case.selectors.formula_variant, 2);
    let formula_condition = selected_projected_condition(
        declaration,
        "formulaVariant",
        case.selectors.formula_variant,
    )?;
    let multiplicity = selected_multiplicity(case.selectors.multiplicity);
    let actual_order = if case.requests_failure(declaration) {
        multiplicity + 1
    } else {
        multiplicity
    };
    let leading = if formula_index == 0 { 1 } else { 2 };
    let mut coefficients = vec![0; actual_order + 1];
    if let Some(coefficient) = coefficients.get_mut(actual_order) {
        *coefficient = leading;
    }
    let width = selected_width(case.selectors.interval_width);
    Ok(ProductInvocation {
        arguments: ProductArguments::EvenTouch(EvenTouchArguments {
            coefficients,
            lower: DyadicArgument {
                integer: -width,
                exponent: 1,
            },
            upper: DyadicArgument {
                integer: width,
                exponent: 1,
            },
            candidate: DyadicArgument {
                integer: 0,
                exponent: 0,
            },
            max_order: actual_order,
            declared_multiplicity: multiplicity,
            formula_condition,
        }),
    })
}

fn project_local_region_transition(
    case: &GeneratedRequestCase,
    declaration: &ProjectionFamilyDeclaration,
    _context: &AdapterContext,
) -> Result<ProductInvocation, AdapterFailure> {
    let formula_index = bucket(case.selectors.formula_variant, 2);
    let formula_condition = selected_projected_condition(
        declaration,
        "formulaVariant",
        case.selectors.formula_variant,
    )?;
    let multiplicity = selected_transition_multiplicity(case.selectors.multiplicity);
    let leading = if formula_index == 0 { 1 } else { 2 };
    let mut coefficients = vec![0; multiplicity + 1];
    if let Some(coefficient) = coefficients.get_mut(multiplicity) {
        *coefficient = leading;
    }
    let width = selected_width(case.selectors.interval_width);
    let lower = -(width / 2);
    let upper = width / 2;
    let candidate = if case.requests_failure(declaration) {
        upper + 1
    } else {
        0
    };
    Ok(ProductInvocation {
        arguments: ProductArguments::LocalRegionTransition(LocalRegionArguments {
            coefficients,
            lower,
            upper,
            candidate,
            max_order: multiplicity,
            precision_bits: selected_precision(case.selectors.family_parameter),
            declared_multiplicity: multiplicity,
            formula_condition,
        }),
    })
}

fn project_surface_exact(
    case: &GeneratedRequestCase,
    declaration: &ProjectionFamilyDeclaration,
    _context: &AdapterContext,
) -> Result<ProductInvocation, AdapterFailure> {
    let formula_index = bucket(case.selectors.formula_variant, 2);
    let formula_condition = selected_projected_condition(
        declaration,
        "formulaVariant",
        case.selectors.formula_variant,
    )?;
    let mut surface = surface_exact_formula(formula_index, formula_condition);
    if case.requests_failure(declaration) {
        surface.denominator.clear();
    }
    Ok(ProductInvocation {
        arguments: ProductArguments::SurfaceExact(SurfaceExactArguments {
            surface,
            u: selected_value(case.selectors.value),
            v: selected_value(case.selectors.family_parameter),
        }),
    })
}

fn project_surface_checked(
    case: &GeneratedRequestCase,
    declaration: &ProjectionFamilyDeclaration,
    context: &AdapterContext,
) -> Result<ProductInvocation, AdapterFailure> {
    let formula_index = bucket(case.selectors.formula_variant, 2);
    let formula_condition = selected_projected_condition(
        declaration,
        "formulaVariant",
        case.selectors.formula_variant,
    )?;
    let mut surface = surface_checked_formula(formula_index, formula_condition);
    if case.requests_failure(declaration) {
        surface.denominator.clear();
    }
    let lower_u = selected_value(case.selectors.value);
    let lower_v = selected_value(case.selectors.family_parameter);
    let width = selected_width(case.selectors.interval_width);
    Ok(ProductInvocation {
        arguments: ProductArguments::SurfaceChecked(SurfaceCheckedArguments {
            surface,
            lower_u,
            upper_u: lower_u + width,
            lower_v,
            upper_v: lower_v + width,
            owner: OwnerId::new(1),
            requirement: ExactExpressionRequirement::new(
                ConsumerId::new(11),
                &[ExprId::new(20)],
                ExactDecisionRequest::Zero(ExprId::new(20)),
                AbsoluteBits::new(8),
            )
            .map_err(|error| {
                AdapterFailure::new(
                    "projectSurfaceChecked",
                    "construct requirement",
                    case.provenance(context),
                    format!("{error:?}"),
                )
            })?,
            source: NecoImplementationSource::current(),
        }),
    })
}

fn surface_exact_formula(index: usize, formula_condition: String) -> SurfaceArguments {
    if index == 0 {
        SurfaceArguments {
            x: vec![surface_term(1, 1, 0)],
            y: vec![surface_term(1, 0, 1)],
            z: vec![surface_term(1, 1, 0), surface_term(1, 0, 1)],
            denominator: vec![surface_term(1, 0, 0)],
            formula_condition: formula_condition.clone(),
        }
    } else {
        SurfaceArguments {
            x: vec![surface_term(2, 1, 0)],
            y: vec![surface_term(2, 0, 1)],
            z: vec![
                surface_term(1, 1, 0),
                surface_term(1, 0, 1),
                surface_term(1, 0, 0),
            ],
            denominator: vec![surface_term(1, 0, 0)],
            formula_condition: formula_condition.clone(),
        }
    }
}

fn surface_checked_formula(index: usize, formula_condition: String) -> SurfaceArguments {
    if index == 0 {
        SurfaceArguments {
            x: vec![surface_term(1, 1, 0)],
            y: vec![surface_term(1, 0, 1)],
            z: vec![surface_term(1, 1, 0), surface_term(1, 0, 1)],
            denominator: vec![surface_term(1, 0, 0)],
            formula_condition: formula_condition.clone(),
        }
    } else {
        SurfaceArguments {
            x: vec![surface_term(1, 1, 0), surface_term(1, 1, 0)],
            y: vec![surface_term(2, 0, 1)],
            z: vec![surface_term(1, 1, 1), surface_term(1, 0, 0)],
            denominator: vec![surface_term(2, 0, 0)],
            formula_condition: formula_condition.clone(),
        }
    }
}

fn surface_term(coefficient: i64, exponent_u: u32, exponent_v: u32) -> SurfaceTermArgument {
    SurfaceTermArgument {
        coefficient,
        exponent_u,
        exponent_v,
    }
}

fn selected_value(selector: u64) -> i64 {
    [-2, 0, 2][bucket(selector, 3)]
}

fn selected_width(selector: u64) -> i64 {
    [2, 4, 6][bucket(selector, 3)]
}

fn selected_precision(selector: u64) -> u32 {
    [8, 12, 16][bucket(selector, 3)]
}

fn selected_iterations(selector: u64) -> usize {
    [1, 4, 8][bucket(selector, 3)]
}

fn selected_multiplicity(selector: u64) -> usize {
    [2, 4][bucket(selector, 2)]
}

fn selected_transition_multiplicity(selector: u64) -> usize {
    [1, 3][bucket(selector, 2)]
}

fn bucket(selector: u64, count: usize) -> usize {
    (selector % count as u64) as usize
}

fn integer_array(values: &[i64]) -> Value {
    Value::Array(values.iter().map(decimal).collect())
}

fn dyadic_decimal(value: &DyadicArgument) -> Value {
    if value.exponent == 0 {
        decimal(value.integer)
    } else {
        decimal(value.integer / (1_i64 << value.exponent))
    }
}

fn dyadic_argument_width(lower: DyadicArgument, upper: DyadicArgument) -> i64 {
    if lower.exponent == upper.exponent {
        (upper.integer - lower.integer) / (1_i64 << lower.exponent)
    } else {
        0
    }
}

fn encode_dyadic(value: &Dyadic) -> Value {
    object([
        ("exponent", Value::Number(value.exponent().into())),
        ("integer", Value::String(bigint_decimal(value.integer()))),
    ])
}

fn encode_enclosure(value: &Enclosure) -> Value {
    object([
        ("lower", encode_dyadic(value.lower())),
        ("upper", encode_dyadic(value.upper())),
    ])
}

fn enclosure_argument_value(lower: i64, upper: i64) -> Value {
    object([
        (
            "lower",
            object([
                ("exponent", Value::Number(0_u32.into())),
                ("integer", decimal(lower)),
            ]),
        ),
        (
            "upper",
            object([
                ("exponent", Value::Number(0_u32.into())),
                ("integer", decimal(upper)),
            ]),
        ),
    ])
}

fn encode_provenance(
    owner: OwnerId,
    requirement: &ExactExpressionRequirement,
    source: NecoImplementationSource,
) -> Value {
    object([
        ("consumerId", Value::Number(requirement.consumer_id().get().into())),
        (
            "decision",
            encode_decision(requirement.decision()),
        ),
        (
            "expressions",
            Value::Array(
                requirement
                    .expressions()
                    .iter()
                    .map(|expression| Value::Number(expression.get().into()))
                    .collect(),
            ),
        ),
        ("owner", Value::Number(owner.get().into())),
        (
            "precision",
            Value::Number(requirement.precision().get().into()),
        ),
        (
            "source",
            object([
                (
                    "publicApiIdentity",
                    Value::String(source.public_api_identity().to_owned()),
                ),
                (
                    "revisionIdentity",
                    Value::String(source.revision_identity().to_owned()),
                ),
            ]),
        ),
    ])
}

fn encode_decision(decision: ExactDecisionRequest) -> Value {
    match decision {
        ExactDecisionRequest::Zero(expression) => object([
            ("expression", Value::Number(expression.get().into())),
            ("kind", Value::String("zero".to_owned())),
        ]),
        ExactDecisionRequest::Equality(left, right) => object([
            (
                "expressions",
                Value::Array(vec![
                    Value::Number(left.get().into()),
                    Value::Number(right.get().into()),
                ]),
            ),
            ("kind", Value::String("equality".to_owned())),
        ]),
        ExactDecisionRequest::Sign(expression) => object([
            ("expression", Value::Number(expression.get().into())),
            ("kind", Value::String("sign".to_owned())),
        ]),
        ExactDecisionRequest::Degeneracy(expression) => object([
            ("expression", Value::Number(expression.get().into())),
            ("kind", Value::String("degeneracy".to_owned())),
        ]),
    }
}

fn surface_input_value(
    surface: &SurfaceArguments,
    u: Value,
    v: Value,
    checked: Option<(
        &OwnerId,
        &ExactExpressionRequirement,
        &NecoImplementationSource,
    )>,
) -> Value {
    let mut fields = BTreeMap::new();
    fields.insert(
        "denominator".to_owned(),
        surface_terms_value(&surface.denominator),
    );
    if let Some((owner, requirement, source)) = checked {
        fields.insert(
            "provenance".to_owned(),
            encode_provenance(*owner, requirement, *source),
        );
    }
    fields.insert("u".to_owned(), u);
    fields.insert("v".to_owned(), v);
    fields.insert("x".to_owned(), surface_terms_value(&surface.x));
    fields.insert("y".to_owned(), surface_terms_value(&surface.y));
    fields.insert("z".to_owned(), surface_terms_value(&surface.z));
    Value::Object(fields.into_iter().collect())
}

fn surface_terms_value(terms: &[SurfaceTermArgument]) -> Value {
    Value::Array(
        terms
            .iter()
            .map(|term| {
                object([
                    (
                        "coefficient",
                        object([
                            ("denominator", decimal(1)),
                            ("numerator", decimal(term.coefficient)),
                        ]),
                    ),
                    ("exponentU", Value::Number(term.exponent_u.into())),
                    ("exponentV", Value::Number(term.exponent_v.into())),
                ])
            })
            .collect(),
    )
}

fn sign_name(sign: Sign) -> &'static str {
    match sign {
        Sign::Negative => "negative",
        Sign::Zero => "zero",
        Sign::Positive => "positive",
    }
}

fn require_exact_fields(
    object_value: &serde_json::Map<String, Value>,
    expected: &[&str],
    operation: &str,
    provenance: Value,
) -> Result<(), AdapterFailure> {
    if object_value.len() == expected.len()
        && expected
            .iter()
            .all(|field| object_value.contains_key(*field))
    {
        Ok(())
    } else {
        Err(AdapterFailure::new(
            "invalidRequest",
            operation,
            provenance,
            "object fields do not match canonical schema",
        ))
    }
}

fn require_string(
    object_value: &serde_json::Map<String, Value>,
    field: &str,
    expected: &str,
    operation: &str,
    provenance: Value,
) -> Result<(), AdapterFailure> {
    if object_value.get(field).and_then(Value::as_str) == Some(expected) {
        Ok(())
    } else {
        Err(AdapterFailure::new(
            "invalidRequest",
            operation,
            provenance,
            "string field does not match canonical schema",
        ))
    }
}

fn parse_hex_u64(value: &str, name: &str, provenance: Value) -> Result<u64, AdapterFailure> {
    if value.len() != 16
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(AdapterFailure::new(
            "invalidRequest",
            "decodeSelector",
            provenance,
            "selector must be a fixed-width lowercase hexadecimal u64",
        ));
    }
    u64::from_str_radix(value, 16)
        .map_err(|_| AdapterFailure::new("invalidRequest", "decodeSelector", object([]), name))
}

fn hex_selector(value: u64) -> Value {
    Value::String(format!("{value:016x}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use calculus_observation_types::canonical_value_bytes;

    fn identity() -> BoundBuildIdentity {
        let projection = projection_declaration_catalog().expect("projection declaration");
        BoundBuildIdentity {
            rustc_identity: "rustc test identity".to_owned(),
            target_triple: "test-target".to_owned(),
            semantics_digest: digest_hex(SEMANTICS_DECLARATION_BYTES),
            semantics_revision: calculus_observation_types::validate_canonical_value(SEMANTICS_DECLARATION_BYTES)
                .expect("semantics declaration")["revision"].as_str().expect("revision").to_owned(),
            projection_digest: projection.projection_digest,
        }
    }

    fn context() -> AdapterContext {
        AdapterContext {
            identity: identity(),
            projection: projection_declaration_catalog().expect("projection declaration"),
            resource: ResourceDeclaration {
                max_batch_cases: 4096,
                revision: "test-resource-revision".to_owned(),
                declaration_digest: "test-resource-digest".to_owned(),
            },
        }
    }

    fn selectors() -> Selectors {
        Selectors {
            value: 0,
            family_parameter: 0,
            observation_class: 0,
            sign: 0,
            multiplicity: 0,
            interval_width: 0,
            formula_variant: 0,
            mutation: 0,
            reserved: 0,
        }
    }

    fn case(
        declaration: &ProjectionFamilyDeclaration,
        requested_observation_kind: String,
    ) -> GeneratedRequestCase {
        GeneratedRequestCase {
            case_id: "case-0000000000000000".to_owned(),
            family: declaration.family.clone(),
            requested_observation_kind,
            selectors: selectors(),
        }
    }

    fn project_and_invoke(
        request: &GeneratedRequestCase,
        context: &AdapterContext,
    ) -> (ProductInvocation, OutcomeKind, Value) {
        let declaration =
            declaration_for(&context.projection, &request.family).expect("declaration");
        let implementation = implementation_for(&declaration.family).expect("implementation");
        let invocation =
            (implementation.project)(request, declaration, context).expect("project invocation");
        validate_projected_conditions(request, declaration, &invocation, context)
            .expect("projected conditions");
        let outcome = invoke_public_api(&invocation, request, context).expect("invoke public API");
        let outcome_kind = match &outcome {
            ProductOutcome::Success(_) => OutcomeKind::Success,
            ProductOutcome::Failure(failure) => {
                assert!(expected_typed_failure(declaration, failure));
                OutcomeKind::Failure
            }
        };
        let observation = observe_product_outcome(
            request,
            implementation,
            declaration,
            &invocation,
            outcome,
            context,
        )
        .expect("encode observation");
        (invocation, outcome_kind, observation)
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum OutcomeKind {
        Success,
        Failure,
    }

    #[test]
    fn projection_declaration_is_canonical_schema_valid_and_digest_bound() {
        let value =
            calculus_observation_types::validate_canonical_value(PROJECTION_DECLARATION_BYTES)
                .expect("canonical projection declaration");
        assert_eq!(
            canonical_value_bytes(&value).expect("canonical bytes"),
            PROJECTION_DECLARATION_BYTES
        );
        validate_projection_metadata(&value).expect("projection schema");
        let catalog = projection_declaration_catalog().expect("projection catalog");
        assert_eq!(
            catalog.projection_digest,
            digest_hex(&canonical_value_bytes(&value["families"]).expect("families bytes"))
        );
        assert_eq!(
            value["schema"],
            "neco.calculus.generated-projection-declaration.v1"
        );
        assert_eq!(value["domain"], GENERATED_REQUEST_DOMAIN);
        for family in value["families"].as_array().expect("families") {
            for binding in family["selectorBindings"]
                .as_array()
                .expect("selector bindings")
            {
                let buckets = binding["buckets"].as_array().expect("buckets");
                let rule = &binding["selectorRule"];
                assert_eq!(rule["modulus"].as_u64(), Some(buckets.len() as u64));
                assert_eq!(
                    rule["bucketsByRemainder"]
                        .as_object()
                        .expect("remainder map")
                        .len(),
                    buckets.len()
                );
                assert_eq!(rule["selector"], binding["selector"]);
            }
        }
    }

    #[test]
    fn resource_declaration_is_canonical_schema_valid_and_digest_bound() {
        let value =
            calculus_observation_types::validate_canonical_value(RESOURCE_DECLARATION_BYTES)
                .expect("canonical resource declaration");
        assert_eq!(
            canonical_value_bytes(&value).expect("canonical bytes"),
            RESOURCE_DECLARATION_BYTES
        );
        decode_resource_declaration(&value).expect("resource schema");
        assert_eq!(
            value["schema"],
            "neco.verification.resource-declaration.v1"
        );
        assert_eq!(value["domain"], GENERATED_REQUEST_DOMAIN);
        let declaration = resource_declaration().expect("resource declaration");
        assert_eq!(
            Value::Number(declaration.max_batch_cases.into()),
            value["maxBatchCases"]
        );
        assert_eq!(declaration.max_batch_cases, 2000);
        assert_eq!(declaration.revision, "calculus-resource-v1");
        assert_eq!(
            declaration.declaration_digest,
            digest_hex(RESOURCE_DECLARATION_BYTES)
        );
    }

    #[test]
    fn resource_declaration_rejects_incomplete_and_nonpositive_metadata() {
        let mut missing =
            calculus_observation_types::validate_canonical_value(RESOURCE_DECLARATION_BYTES)
                .expect("resource declaration");
        missing
            .as_object_mut()
            .expect("resource declaration object")
            .remove("resourceRevision");
        assert!(decode_resource_declaration(&missing).is_err());

        let mut exhausted =
            calculus_observation_types::validate_canonical_value(RESOURCE_DECLARATION_BYTES)
                .expect("resource declaration");
        exhausted["maxBatchCases"] = Value::Number(0u64.into());
        assert!(decode_resource_declaration(&exhausted).is_err());
    }

    #[test]
    fn projection_declarations_are_unique_and_bijective_with_implementations() {
        let catalog = projection_declaration_catalog().expect("projection declaration");
        let declared = catalog
            .families
            .iter()
            .map(|declaration| declaration.family.as_str())
            .collect::<BTreeSet<_>>();
        let implemented = PROJECTION_IMPLEMENTATIONS
            .iter()
            .map(|implementation| implementation.family)
            .collect::<BTreeSet<_>>();
        assert_eq!(declared, implemented);
        assert_eq!(catalog.families.len(), declared.len());
        for declaration in &catalog.families {
            assert!(!declaration.typed_failure.is_empty());
            assert!(catalog.value["families"]
                .as_array()
                .expect("family catalog")
                .iter()
                .any(|value| {
                    value["family"].as_str() == Some(declaration.family.as_str())
                        && value["checkerPredicate"].is_string()
                }));
            assert_eq!(
                declaration.requested_observation_kinds,
                vec!["success".to_owned(), declaration.typed_failure.clone()]
            );
            assert!(!declaration.bindings.is_empty());
            for binding in &declaration.bindings {
                assert_eq!(binding.selector_rule.selector, binding.selector);
                assert_eq!(
                    usize::try_from(binding.selector_rule.modulus).ok(),
                    Some(binding.buckets.len())
                );
                assert_eq!(
                    binding.selector_rule.buckets_by_remainder.len(),
                    binding.buckets.len()
                );
            }
        }
    }

    #[test]
    fn projection_declaration_rejects_missing_and_duplicate_metadata() {
        let mut missing =
            calculus_observation_types::validate_canonical_value(PROJECTION_DECLARATION_BYTES)
                .expect("projection declaration");
        missing["families"][0]
            .as_object_mut()
            .expect("family object")
            .remove("checkerPredicate");
        assert!(validate_projection_metadata(&missing).is_err());

        let mut duplicate =
            calculus_observation_types::validate_canonical_value(PROJECTION_DECLARATION_BYTES)
                .expect("projection declaration");
        duplicate["families"][1]["family"] = duplicate["families"][0]["family"].clone();
        assert!(validate_projection_metadata(&duplicate).is_err());

        let mut selector_rule =
            calculus_observation_types::validate_canonical_value(PROJECTION_DECLARATION_BYTES)
                .expect("projection declaration");
        selector_rule["families"][0]["selectorBindings"][0]["selectorRule"] =
            Value::String("selector modulo finite bucket count".to_owned());
        assert!(validate_projection_metadata(&selector_rule).is_err());
    }

    #[test]
    fn applicable_buckets_are_reachable_from_manifest_metadata() {
        let context = context();
        for declaration in &context.projection.families {
            for requested_kind in ["success".to_owned(), declaration.typed_failure.clone()] {
                for binding in &declaration.bindings {
                    for (index, bucket) in binding.buckets.iter().enumerate() {
                        let mut request = case(declaration, requested_kind.clone());
                        set_selector(&mut request.selectors, &binding.selector, index as u64);
                        let (invocation, outcome, observation) =
                            project_and_invoke(&request, &context);
                        assert_eq!(
                            invocation
                                .projected_conditions()
                                .get(binding.axis.as_str())
                                .map(String::as_str),
                            Some(bucket.projected_condition.as_str())
                        );
                        assert_eq!(observation["operation"], declaration.family.as_str());
                        assert_eq!(observation["caseId"], request.case_id);
                        match (requested_kind.as_str(), outcome) {
                            ("success", OutcomeKind::Success) => {
                                assert_eq!(observation["observation"]["kind"], "success");
                            }
                            (_, OutcomeKind::Failure) => {
                                assert_eq!(
                                    observation["observation"]["failure"],
                                    declaration.typed_failure.as_str()
                                );
                            }
                            _ => panic!("request kind and public outcome differ"),
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn selector_axes_are_orthogonal_and_unbound_selectors_preserve_observations() {
        let context = context();
        for declaration in &context.projection.families {
            let baseline = case(declaration, "success".to_owned());
            let (baseline_invocation, _, baseline_observation) =
                project_and_invoke(&baseline, &context);
            let applicable = declaration
                .bindings
                .iter()
                .map(|binding| binding.selector.as_str())
                .collect::<BTreeSet<_>>();
            for binding in &declaration.bindings {
                if binding.buckets.len() > 1 {
                    let mut changed = baseline.clone();
                    set_selector(&mut changed.selectors, &binding.selector, 1);
                    let (changed_invocation, _, _) = project_and_invoke(&changed, &context);
                    let baseline_conditions = baseline_invocation.projected_conditions();
                    let changed_conditions = changed_invocation.projected_conditions();
                    for axis in baseline_conditions.keys() {
                        if axis == &binding.axis {
                            assert_ne!(baseline_conditions.get(axis), changed_conditions.get(axis));
                        } else {
                            assert_eq!(baseline_conditions.get(axis), changed_conditions.get(axis));
                        }
                    }
                }
            }
            for selector in [
                "value",
                "familyParameter",
                "observationClass",
                "sign",
                "multiplicity",
                "intervalWidth",
                "formulaVariant",
                "mutation",
                "reserved",
            ] {
                if !applicable.contains(selector) {
                    let mut changed = baseline.clone();
                    set_selector(&mut changed.selectors, selector, 1);
                    let (changed_invocation, _, changed_observation) =
                        project_and_invoke(&changed, &context);
                    assert_eq!(baseline_invocation, changed_invocation);
                    assert_eq!(baseline_observation, changed_observation);
                }
            }
        }
    }

    fn set_selector(selectors: &mut Selectors, selector: &str, value: u64) {
        match selector {
            "value" => selectors.value = value,
            "familyParameter" => selectors.family_parameter = value,
            "observationClass" => selectors.observation_class = value,
            "sign" => selectors.sign = value,
            "multiplicity" => selectors.multiplicity = value,
            "intervalWidth" => selectors.interval_width = value,
            "formulaVariant" => selectors.formula_variant = value,
            "mutation" => selectors.mutation = value,
            "reserved" => selectors.reserved = value,
            _ => panic!("unknown selector"),
        }
    }

    #[test]
    fn projection_and_resource_claims_are_separate() {
        let context = context();
        let projection_claim = projection_claim_value(&context.projection);
        let resource_claim = resource_claim_value(&context.resource);
        assert!(projection_claim.get("maxBatchCases").is_none());
        assert!(resource_claim.get("projectionDeclarations").is_none());
        assert_eq!(
            projection_claim["projectionDeclarations"],
            context.projection.value["families"]
        );
        assert_eq!(
            projection_claim["projectionRevision"],
            projection_claim["projectionDeclarationDigest"]
        );
    }


    #[test]
    fn generate_runs_with_self_observed_identity() {
        let request = object([
            (
                "adapterRevision",
                Value::String(ADAPTER_REVISION.to_owned()),
            ),
            ("cases", Value::Array(Vec::new())),
            ("domain", Value::String(GENERATED_REQUEST_DOMAIN.to_owned())),
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
                Value::String(GENERATED_REQUEST_SCHEMA.to_owned()),
            ),
        ]);
        let bytes = canonical_value_bytes(&request).expect("canonical request");
        let batch = generate_request_batch(&bytes).expect("generate");
        assert!(batch.cases().is_empty());
    }


    #[test]
    fn generated_path_has_no_panic_expect_or_unwrap() {
        let source = include_str!("generated.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source");
        for forbidden in ["panic!(", "assert!(", ".expect(", ".unwrap(", "unwrap_or"] {
            assert!(!production.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn product_failure_canonical_value_keeps_the_debug_text() {
        let failure = ProductFailure::execution(ProductError::Calculus(CalculusFailure::from(
            EvaluationFailure::DuplicateImplicitExponent,
        )));
        assert_eq!(
            failure.canonical_value(),
            object([
                (
                    "debug",
                    Value::String("Evaluation(DuplicateImplicitExponent)".to_owned()),
                ),
                (
                    "schema",
                    Value::String("neco.verification.product-failure.v1".to_owned()),
                ),
                (
                    "source",
                    Value::String("neco_calculus::CalculusFailure".to_owned()),
                ),
                ("stage", Value::String("execution".to_owned())),
            ])
        );
    }
}
