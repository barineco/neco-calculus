#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

extern crate alloc;

mod quadrature;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::num::NonZeroUsize;

use neco_algnum::{AlgnumError, RationalPolynomial, RealAlgebraic};
use neco_bigint::{
    BigInt, BigUint, BigintError, Dyadic, DyadicEnclosure, RawRational, ReducedRational, Sign,
};
use neco_expr::project_enclosure_f64;
pub use neco_expr::{
    AbsoluteBits, CertifiedF64, ConsumerId, EnclosureProjectionError, ExactDecisionRequest,
    ExactExpressionRequirement, ExprId, FailureLocation, FailureOperation, NecoFailure,
    NecoImplementationSource, OwnerId, PrecisionFailurePayload, ProvenanceMismatchField,
    ProvenanceMismatchPayload, ProvenanceRecord,
};
pub use neco_jet::{Jet, JetFailure, MAX_JET_ORDER};
pub use neco_polynomial::{
    DerivativeOrder, Enclosure, IntervalFailure, Polynomial, PolynomialFailure,
    ResourceFailure as PolynomialResourceFailure, MAX_COEFFICIENTS, MAX_DERIVATIVE_ORDER,
    MAX_PRECISION_BITS,
};
pub use neco_rational::{RationalFailure, RationalFunction};
pub use neco_root::{
    classify_multiplicity, classify_region_sign, locate_membership_transitions, ConvergenceStatus,
    ConvergenceWitness, MembershipJudgmentRequest, MembershipLocus, MembershipLocusKind,
    MembershipTransitionReport, NewtonFallbackReason, RegionSign, RegionSignJudgment, RootBracket,
    RootEvidence, RootFailure, RootIsolation, RootMultiplicity, RootRefinementMethod,
};
pub use quadrature::{
    evaluate_quadrature, IntervalDifferentiable, QuadratureEvaluation, QuadratureEvidence,
    QuadratureInput, QuadratureOrder, QuadratureRefinement,
};

pub fn expose_region_sign(
    request: &MembershipJudgmentRequest,
) -> Result<RegionSignJudgment, CalculusFailure> {
    Ok(classify_region_sign(request)?)
}

pub fn expose_membership_transitions(
    request: &MembershipJudgmentRequest,
) -> Result<MembershipTransitionReport, CalculusFailure> {
    Ok(locate_membership_transitions(request)?)
}

#[derive(Debug, Eq, PartialEq)]
pub struct PolynomialEvaluation {
    value: RealAlgebraic,
}

impl PolynomialEvaluation {
    fn new(value: RealAlgebraic) -> Self {
        Self { value }
    }

    pub fn value(&self) -> &RealAlgebraic {
        &self.value
    }

    pub fn into_value(self) -> RealAlgebraic {
        self.value
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RationalEvaluation {
    value: RealAlgebraic,
}

impl RationalEvaluation {
    fn new(value: RealAlgebraic) -> Self {
        Self { value }
    }

    pub fn value(&self) -> &RealAlgebraic {
        &self.value
    }

    pub fn into_value(self) -> RealAlgebraic {
        self.value
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct IntervalEvaluation {
    enclosure: Enclosure,
}

impl IntervalEvaluation {
    fn new(enclosure: Enclosure) -> Self {
        Self { enclosure }
    }

    pub fn enclosure(&self) -> &Enclosure {
        &self.enclosure
    }

    pub fn into_enclosure(self) -> Enclosure {
        self.enclosure
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranscendentalFunction {
    Sine,
    Cosine,
    Exponential,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationOperation {
    SurfaceExact,
    SurfaceChecked,
    Transcendental,
    TranscendentalFunction(TranscendentalFunction),
    TranscendentalPower,
    Quadrature,
}

#[derive(Debug, Eq, PartialEq)]
pub enum EvaluationInputSnapshot {
    SurfaceChecked {
        surface: RationalSurfacePolynomials,
        u: Enclosure,
        v: Enclosure,
    },
    Transcendental {
        candidate: TranscendentalEvaluationCandidate,
    },
    TranscendentalFunction {
        function: TranscendentalFunction,
        argument: Enclosure,
    },
    TranscendentalPower {
        base: Enclosure,
        exponent: Enclosure,
    },
    Quadrature {
        lower: Dyadic,
        upper: Dyadic,
        order: QuadratureOrder,
        maximum_pieces: NonZeroUsize,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum DomainWitness {
    ZeroDenominatorPolynomial,
    DomainContainsZero {
        denominator: Enclosure,
    },
    PointOutsideDomain {
        point: RealAlgebraic,
    },
    NonPositiveBase {
        base: Enclosure,
        exponent: Enclosure,
    },
    NonIncreasingBounds {
        lower: Dyadic,
        upper: Dyadic,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum PrecisionWitness {
    Limit { maximum: u32 },
    Enclosure { enclosure: Enclosure },
}

#[derive(Debug, Eq, PartialEq)]
pub enum PrecisionShortageCause {
    RequestedBeyondLimit {
        requested: AbsoluteBits,
        maximum: u32,
    },
    Exhausted(NecoFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LowerWitness {
    operation: EvaluationOperation,
}

impl LowerWitness {
    pub const fn new(operation: EvaluationOperation) -> Self {
        Self { operation }
    }

    pub const fn operation(&self) -> EvaluationOperation {
        self.operation
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum LowerEvaluationFailure {
    Calculus(Box<CalculusFailure>),
    Expression(NecoFailure),
}

#[derive(Debug, Eq, PartialEq)]
pub enum EvaluationFailure {
    ZeroImplicitTerm,
    DuplicateImplicitExponent,
    SurfaceTermLimitExceeded {
        requested: usize,
        maximum: usize,
    },
    SurfaceExponentLimitExceeded {
        requested: u32,
        maximum: u32,
    },
    ImplicitTermLimitExceeded {
        requested: usize,
        maximum: usize,
    },
    DomainViolation {
        operation: EvaluationOperation,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
        input: EvaluationInputSnapshot,
        witness: DomainWitness,
        lower: RationalFailure,
    },
    TranscendentalDomainViolation {
        operation: EvaluationOperation,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
        input: EvaluationInputSnapshot,
        witness: DomainWitness,
    },
    PrecisionShortage {
        operation: EvaluationOperation,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
        input: EvaluationInputSnapshot,
        requested: AbsoluteBits,
        witness: PrecisionWitness,
        lower: PrecisionShortageCause,
    },
    Lower {
        operation: EvaluationOperation,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
        input: EvaluationInputSnapshot,
        witness: LowerWitness,
        lower: LowerEvaluationFailure,
    },
    QuadraturePieceLimitExceeded {
        requested: usize,
        maximum: usize,
    },
    QuadratureDomainViolation {
        operation: EvaluationOperation,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
        input: EvaluationInputSnapshot,
        witness: DomainWitness,
    },
    QuadratureNotConverged {
        operation: EvaluationOperation,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
        input: EvaluationInputSnapshot,
        achieved: Enclosure,
        evidence: QuadratureEvidence,
    },
}

pub const MAX_SURFACE_TERMS: usize = 4_096;
pub const MAX_SURFACE_EXPONENT: u32 = 256;
pub const MAX_IMPLICIT_TERMS: usize = 4_096;
pub const MAX_QUADRATURE_PIECES: usize = 65_536;

#[derive(Debug, Eq, PartialEq)]
pub enum CalculusFailure {
    Polynomial(PolynomialFailure),
    Rational(RationalFailure),
    Jet(JetFailure),
    Interval(IntervalFailure),
    Root(RootFailure),
    Transcendental(TranscendentalConnectorFailure),
    Evaluation(Box<EvaluationFailure>),
    Algebra(AlgnumError),
    Arithmetic(BigintError),
}

#[derive(Debug, Eq, PartialEq)]
pub struct TranscendentalEvaluationRequest {
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
}

impl TranscendentalEvaluationRequest {
    pub fn new(
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    ) -> Self {
        Self {
            owner,
            requirement,
            source,
        }
    }

    pub const fn owner(&self) -> OwnerId {
        self.owner
    }

    pub fn requirement(&self) -> &ExactExpressionRequirement {
        &self.requirement
    }

    pub const fn source(&self) -> NecoImplementationSource {
        self.source
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum TranscendentalEvaluationCandidate {
    ExactAlgebraic {
        value: RealAlgebraic,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    },
    Enclosure {
        enclosure: Enclosure,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    },
    Undetermined {
        achieved: Enclosure,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct TranscendentalEvaluationConstructionInput {
    candidate: TranscendentalEvaluationCandidate,
    request: TranscendentalEvaluationRequest,
}

impl TranscendentalEvaluationConstructionInput {
    pub const fn new(
        candidate: TranscendentalEvaluationCandidate,
        request: TranscendentalEvaluationRequest,
    ) -> Self {
        Self { candidate, request }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranscendentalEvaluationKind {
    ExactAlgebraic,
    Enclosure,
    Undetermined,
}

#[derive(Debug, Eq, PartialEq)]
enum TranscendentalEvaluationValue {
    ExactAlgebraic {
        value: RealAlgebraic,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    },
    Enclosure {
        enclosure: Enclosure,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    },
    Undetermined {
        achieved: Enclosure,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct TranscendentalEvaluation {
    value: TranscendentalEvaluationValue,
}

impl TranscendentalEvaluation {
    pub const fn kind(&self) -> TranscendentalEvaluationKind {
        match &self.value {
            TranscendentalEvaluationValue::ExactAlgebraic { .. } => {
                TranscendentalEvaluationKind::ExactAlgebraic
            }
            TranscendentalEvaluationValue::Enclosure { .. } => {
                TranscendentalEvaluationKind::Enclosure
            }
            TranscendentalEvaluationValue::Undetermined { .. } => {
                TranscendentalEvaluationKind::Undetermined
            }
        }
    }

    pub fn as_exact_algebraic(&self) -> Option<&RealAlgebraic> {
        match &self.value {
            TranscendentalEvaluationValue::ExactAlgebraic { value, .. } => Some(value),
            _ => None,
        }
    }

    pub fn as_enclosure(&self) -> Option<&Enclosure> {
        match &self.value {
            TranscendentalEvaluationValue::Enclosure { enclosure, .. } => Some(enclosure),
            _ => None,
        }
    }

    pub const fn is_undetermined(&self) -> bool {
        matches!(
            self.value,
            TranscendentalEvaluationValue::Undetermined { .. }
        )
    }

    pub const fn owner(&self) -> OwnerId {
        match &self.value {
            TranscendentalEvaluationValue::ExactAlgebraic { owner, .. }
            | TranscendentalEvaluationValue::Enclosure { owner, .. }
            | TranscendentalEvaluationValue::Undetermined { owner, .. } => *owner,
        }
    }

    pub fn requirement(&self) -> &ExactExpressionRequirement {
        match &self.value {
            TranscendentalEvaluationValue::ExactAlgebraic { requirement, .. }
            | TranscendentalEvaluationValue::Enclosure { requirement, .. }
            | TranscendentalEvaluationValue::Undetermined { requirement, .. } => requirement,
        }
    }

    pub const fn source(&self) -> NecoImplementationSource {
        match &self.value {
            TranscendentalEvaluationValue::ExactAlgebraic { source, .. }
            | TranscendentalEvaluationValue::Enclosure { source, .. }
            | TranscendentalEvaluationValue::Undetermined { source, .. } => *source,
        }
    }

    /// Interval kept when the width estimate exceeds the target width. Its
    /// actual width can still meet the target.
    pub fn undetermined_achieved(&self) -> Option<&Enclosure> {
        match &self.value {
            TranscendentalEvaluationValue::Undetermined { achieved, .. } => Some(achieved),
            _ => None,
        }
    }

    /// Nearest f64 with the enclosure and absolute error of the rounding.
    pub fn certify_f64(&self) -> Result<CertifiedF64, TranscendentalCertificationFailure> {
        let (enclosure, requirement) = match &self.value {
            TranscendentalEvaluationValue::ExactAlgebraic {
                value, requirement, ..
            } => {
                let isolating = value
                    .enclose(requirement.precision().get())
                    .map_err(TranscendentalCertificationFailure::Algebraic)?;
                (isolating.enclosure().try_clone(), requirement)
            }
            TranscendentalEvaluationValue::Enclosure {
                enclosure,
                requirement,
                ..
            } => (enclosure.as_dyadic_enclosure().try_clone(), requirement),
            TranscendentalEvaluationValue::Undetermined { .. } => {
                return Err(TranscendentalCertificationFailure::Undetermined)
            }
        };
        let enclosure = enclosure.map_err(|error| {
            TranscendentalCertificationFailure::Projection(EnclosureProjectionError::Bigint(error))
        })?;
        project_enclosure_f64(enclosure, requirement.precision())
            .map_err(TranscendentalCertificationFailure::Projection)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranscendentalConnectorFailure {
    PrecisionLimitExceeded { requested: u32, maximum: u32 },
    PrecisionMustBePositive,
    EnclosureTooWide { requested_precision_bits: u32 },
}

#[derive(Debug, Eq, PartialEq)]
pub enum TranscendentalCertificationFailure {
    Undetermined,
    Algebraic(AlgnumError),
    Projection(EnclosureProjectionError),
}

#[derive(Debug, Eq, PartialEq)]
pub struct TranscendentalFunctionEvaluationInput {
    pub function: TranscendentalFunction,
    pub argument: Enclosure,
    pub request: TranscendentalEvaluationRequest,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TranscendentalPowerEvaluationInput {
    pub base: Enclosure,
    pub exponent: Enclosure,
    pub request: TranscendentalEvaluationRequest,
}

/// Term limit of one series evaluation.
pub const MAX_TRANSCENDENTAL_SERIES_TERMS: u32 = 256;

pub fn evaluate_polynomial(
    polynomial: &Polynomial,
    point: &RealAlgebraic,
) -> Result<PolynomialEvaluation, CalculusFailure> {
    Ok(PolynomialEvaluation::new(polynomial.evaluate(point)?))
}

pub struct RationalPolynomialInput {
    pub polynomial: RationalPolynomial,
    pub point: RealAlgebraic,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RationalPolynomialEvaluator {
    value: RealAlgebraic,
}

impl RationalPolynomialEvaluator {
    pub fn value(&self) -> &RealAlgebraic {
        &self.value
    }

    pub fn into_value(self) -> RealAlgebraic {
        self.value
    }
}

pub fn evaluate_rational_polynomial(
    input: RationalPolynomialInput,
) -> Result<RationalPolynomialEvaluator, CalculusFailure> {
    if input.polynomial.coefficients().len() > MAX_COEFFICIENTS {
        return Err(PolynomialFailure::Resource(
            PolynomialResourceFailure::CoefficientLimitExceeded {
                requested: input.polynomial.coefficients().len(),
                maximum: MAX_COEFFICIENTS,
            },
        )
        .into());
    }
    let coefficients = input.polynomial.to_real_algebraic_coefficients()?;
    let polynomial = Polynomial::from_coefficients(coefficients.into_coefficients())?;
    Ok(RationalPolynomialEvaluator {
        value: polynomial.evaluate(&input.point)?,
    })
}

pub fn evaluate_rational(
    function: &RationalFunction,
    point: &RealAlgebraic,
) -> Result<RationalEvaluation, CalculusFailure> {
    Ok(RationalEvaluation::new(function.evaluate(point)?))
}

pub fn differentiate(
    polynomial: &Polynomial,
    order: DerivativeOrder,
) -> Result<Polynomial, CalculusFailure> {
    Ok(polynomial.derivative(order)?)
}

pub trait JetSource {
    fn jet_at(&self, point: &RealAlgebraic, order: DerivativeOrder)
        -> Result<Jet, CalculusFailure>;
}

impl JetSource for Polynomial {
    fn jet_at(
        &self,
        point: &RealAlgebraic,
        order: DerivativeOrder,
    ) -> Result<Jet, CalculusFailure> {
        validate_jet_order(order)?;
        let mut derivatives = Vec::new();
        for current in 0..=order.get() {
            derivatives.push(
                self.derivative(DerivativeOrder::new(current))?
                    .evaluate(point)?,
            );
        }
        let value = derivatives[0].try_clone()?;
        Ok(Jet::from_value_and_derivatives(value, derivatives)?)
    }
}

pub fn jet_at<F>(
    function: &F,
    point: &RealAlgebraic,
    order: DerivativeOrder,
) -> Result<Jet, CalculusFailure>
where
    F: JetSource + ?Sized,
{
    validate_jet_order(order)?;
    function.jet_at(point, order)
}

fn validate_jet_order(order: DerivativeOrder) -> Result<(), CalculusFailure> {
    if order.get() > MAX_JET_ORDER {
        return Err(JetFailure::OrderLimitExceeded {
            requested: order.get(),
            maximum: MAX_JET_ORDER,
        }
        .into());
    }
    Ok(())
}

pub trait IntervalEvaluable {
    fn evaluate_interval(
        &self,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure>;
}

impl IntervalEvaluable for Polynomial {
    fn evaluate_interval(
        &self,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        Ok(self.evaluate_enclosure(domain, precision_bits)?)
    }
}

impl IntervalEvaluable for RationalFunction {
    fn evaluate_interval(
        &self,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        Ok(self.evaluate_on(domain, precision_bits)?)
    }
}

pub fn evaluate_interval<F>(
    function: &F,
    domain: &Enclosure,
    precision_bits: u32,
) -> Result<IntervalEvaluation, CalculusFailure>
where
    F: IntervalEvaluable + ?Sized,
{
    Ok(IntervalEvaluation::new(
        function.evaluate_interval(domain, precision_bits)?,
    ))
}

#[derive(Debug, Eq, PartialEq)]
pub struct SurfaceTerm {
    pub coefficient: neco_bigint::ReducedRational,
    pub exponent_u: u32,
    pub exponent_v: u32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SurfacePolynomial {
    terms: Vec<SurfaceTerm>,
}

impl SurfacePolynomial {
    pub fn try_from_terms(mut terms: Vec<SurfaceTerm>) -> Result<Self, CalculusFailure> {
        if terms.len() > MAX_SURFACE_TERMS {
            return Err(EvaluationFailure::SurfaceTermLimitExceeded {
                requested: terms.len(),
                maximum: MAX_SURFACE_TERMS,
            }
            .into());
        }
        for term in &terms {
            if term.exponent_u > MAX_SURFACE_EXPONENT {
                return Err(EvaluationFailure::SurfaceExponentLimitExceeded {
                    requested: term.exponent_u,
                    maximum: MAX_SURFACE_EXPONENT,
                }
                .into());
            }
            if term.exponent_v > MAX_SURFACE_EXPONENT {
                return Err(EvaluationFailure::SurfaceExponentLimitExceeded {
                    requested: term.exponent_v,
                    maximum: MAX_SURFACE_EXPONENT,
                }
                .into());
            }
        }
        terms.sort_unstable_by_key(|term| (term.exponent_u, term.exponent_v));

        let mut normalized: Vec<SurfaceTerm> = Vec::with_capacity(terms.len());
        for term in terms {
            if term.coefficient.is_zero() {
                continue;
            }
            if let Some(previous) = normalized.last_mut() {
                if (previous.exponent_u, previous.exponent_v) == (term.exponent_u, term.exponent_v)
                {
                    previous.coefficient = previous.coefficient.add(&term.coefficient)?;
                    if previous.coefficient.is_zero() {
                        normalized.pop();
                    }
                    continue;
                }
            }
            normalized.push(term);
        }
        Ok(Self { terms: normalized })
    }

    pub fn terms(&self) -> &[SurfaceTerm] {
        &self.terms
    }
}

pub struct SurfaceEvaluationInput {
    pub x: SurfacePolynomial,
    pub y: SurfacePolynomial,
    pub z: SurfacePolynomial,
    pub denominator: SurfacePolynomial,
    pub u: RealAlgebraic,
    pub v: RealAlgebraic,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RationalSurfacePolynomials {
    pub x: SurfacePolynomial,
    pub y: SurfacePolynomial,
    pub z: SurfacePolynomial,
    pub denominator: SurfacePolynomial,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SurfaceEvaluationInput2 {
    surface: RationalSurfacePolynomials,
    u: Enclosure,
    v: Enclosure,
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
}

impl SurfaceEvaluationInput2 {
    pub fn new(
        surface: RationalSurfacePolynomials,
        u: Enclosure,
        v: Enclosure,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    ) -> Result<Self, CalculusFailure> {
        let requested = requirement.precision();
        if requested.get() == 0 || requested.get() > MAX_PRECISION_BITS {
            return Err(EvaluationFailure::PrecisionShortage {
                operation: EvaluationOperation::SurfaceChecked,
                owner,
                requirement,
                source,
                input: EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
                requested,
                witness: PrecisionWitness::Limit {
                    maximum: MAX_PRECISION_BITS,
                },
                lower: PrecisionShortageCause::RequestedBeyondLimit {
                    requested,
                    maximum: MAX_PRECISION_BITS,
                },
            }
            .into());
        }
        Ok(Self {
            surface,
            u,
            v,
            owner,
            requirement,
            source,
        })
    }

    pub fn surface(&self) -> &RationalSurfacePolynomials {
        &self.surface
    }

    pub fn u(&self) -> &Enclosure {
        &self.u
    }

    pub fn v(&self) -> &Enclosure {
        &self.v
    }

    pub const fn owner(&self) -> OwnerId {
        self.owner
    }

    pub fn requirement(&self) -> &ExactExpressionRequirement {
        &self.requirement
    }

    pub const fn source(&self) -> NecoImplementationSource {
        self.source
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CoordinateEnclosure3 {
    x: Enclosure,
    y: Enclosure,
    z: Enclosure,
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
}

impl CoordinateEnclosure3 {
    fn new(
        x: Enclosure,
        y: Enclosure,
        z: Enclosure,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    ) -> Self {
        Self {
            x,
            y,
            z,
            owner,
            requirement,
            source,
        }
    }

    pub fn x(&self) -> &Enclosure {
        &self.x
    }

    pub fn y(&self) -> &Enclosure {
        &self.y
    }

    pub fn z(&self) -> &Enclosure {
        &self.z
    }

    pub const fn owner(&self) -> OwnerId {
        self.owner
    }

    pub fn requirement(&self) -> &ExactExpressionRequirement {
        &self.requirement
    }

    pub const fn source(&self) -> NecoImplementationSource {
        self.source
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SurfaceEvaluationProducer {
    Exact {
        x: RealAlgebraic,
        y: RealAlgebraic,
        z: RealAlgebraic,
    },
    Certified {
        coordinates: Box<CoordinateEnclosure3>,
    },
}

pub fn evaluate_surface_checked(
    input: SurfaceEvaluationInput2,
) -> Result<SurfaceEvaluationProducer, CalculusFailure> {
    let SurfaceEvaluationInput2 {
        surface,
        u,
        v,
        owner,
        requirement,
        source,
    } = input;
    if surface.denominator.terms.is_empty() {
        return Err(EvaluationFailure::DomainViolation {
            operation: EvaluationOperation::SurfaceChecked,
            owner,
            requirement,
            source,
            input: EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
            witness: DomainWitness::ZeroDenominatorPolynomial,
            lower: RationalFailure::ZeroDenominatorPolynomial,
        }
        .into());
    }
    let denominator = match evaluate_surface_polynomial_interval(
        &surface.denominator,
        &u,
        &v,
        requirement.precision().get(),
    ) {
        Ok(value) => value,
        Err(error) => {
            return Err(evaluation_lower_failure(
                EvaluationOperation::SurfaceChecked,
                owner,
                requirement,
                source,
                EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
                error,
            ))
        }
    };
    if denominator.contains_zero() {
        return Err(EvaluationFailure::DomainViolation {
            operation: EvaluationOperation::SurfaceChecked,
            owner,
            requirement,
            source,
            input: EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
            witness: DomainWitness::DomainContainsZero { denominator },
            lower: RationalFailure::DomainContainsZero,
        }
        .into());
    }
    let x = match evaluate_surface_polynomial_interval(
        &surface.x,
        &u,
        &v,
        requirement.precision().get(),
    ) {
        Ok(value) => value,
        Err(error) => {
            return Err(evaluation_lower_failure(
                EvaluationOperation::SurfaceChecked,
                owner,
                requirement,
                source,
                EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
                error,
            ))
        }
    };
    let y = match evaluate_surface_polynomial_interval(
        &surface.y,
        &u,
        &v,
        requirement.precision().get(),
    ) {
        Ok(value) => value,
        Err(error) => {
            return Err(evaluation_lower_failure(
                EvaluationOperation::SurfaceChecked,
                owner,
                requirement,
                source,
                EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
                error,
            ))
        }
    };
    let z = match evaluate_surface_polynomial_interval(
        &surface.z,
        &u,
        &v,
        requirement.precision().get(),
    ) {
        Ok(value) => value,
        Err(error) => {
            return Err(evaluation_lower_failure(
                EvaluationOperation::SurfaceChecked,
                owner,
                requirement,
                source,
                EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
                error,
            ))
        }
    };
    let x = match enclosure_div(
        x.as_dyadic_enclosure(),
        denominator.as_dyadic_enclosure(),
        requirement.precision().get(),
    ) {
        Ok(value) => Enclosure::from_dyadic_enclosure(value),
        Err(error) => {
            return Err(evaluation_lower_failure(
                EvaluationOperation::SurfaceChecked,
                owner,
                requirement,
                source,
                EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
                error.into(),
            ))
        }
    };
    let y = match enclosure_div(
        y.as_dyadic_enclosure(),
        denominator.as_dyadic_enclosure(),
        requirement.precision().get(),
    ) {
        Ok(value) => Enclosure::from_dyadic_enclosure(value),
        Err(error) => {
            return Err(evaluation_lower_failure(
                EvaluationOperation::SurfaceChecked,
                owner,
                requirement,
                source,
                EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
                error.into(),
            ))
        }
    };
    let z = match enclosure_div(
        z.as_dyadic_enclosure(),
        denominator.as_dyadic_enclosure(),
        requirement.precision().get(),
    ) {
        Ok(value) => Enclosure::from_dyadic_enclosure(value),
        Err(error) => {
            return Err(evaluation_lower_failure(
                EvaluationOperation::SurfaceChecked,
                owner,
                requirement,
                source,
                EvaluationInputSnapshot::SurfaceChecked { surface, u, v },
                error.into(),
            ))
        }
    };
    Ok(SurfaceEvaluationProducer::Certified {
        coordinates: Box::new(CoordinateEnclosure3::new(
            x,
            y,
            z,
            owner,
            requirement,
            source,
        )),
    })
}

fn precision_shortage_location(requirement: &ExactExpressionRequirement) -> FailureLocation {
    FailureLocation::operation(FailureOperation::Resolve)
        .with_consumer(requirement.consumer_id())
        .with_expr(
            requirement
                .expressions()
                .first()
                .copied()
                .unwrap_or_else(|| ExprId::new(0)),
        )
}

fn evaluation_lower_failure(
    operation: EvaluationOperation,
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
    input: EvaluationInputSnapshot,
    error: CalculusFailure,
) -> CalculusFailure {
    EvaluationFailure::Lower {
        operation,
        owner,
        requirement,
        source,
        input,
        witness: LowerWitness::new(operation),
        lower: LowerEvaluationFailure::Calculus(Box::new(error)),
    }
    .into()
}

fn evaluate_surface_polynomial_interval(
    polynomial: &SurfacePolynomial,
    u: &Enclosure,
    v: &Enclosure,
    precision_bits: u32,
) -> Result<Enclosure, CalculusFailure> {
    let mut result = DyadicEnclosure::new(
        Dyadic::new(BigInt::zero(), 0),
        Dyadic::new(BigInt::zero(), 0),
    )?;
    for term in &polynomial.terms {
        let coefficient = RealAlgebraic::from_reduced_rational(&term.coefficient)?
            .enclose(precision_bits)?
            .enclosure()
            .try_clone()?;
        let u_power = interval_power_horner(u.as_dyadic_enclosure(), term.exponent_u)?;
        let v_power = interval_power_horner(v.as_dyadic_enclosure(), term.exponent_v)?;
        let term_value = enclosure_mul(&enclosure_mul(&u_power, &v_power)?, &coefficient)?;
        result = enclosure_add(&result, &term_value)?;
    }
    Ok(Enclosure::from_dyadic_enclosure(result))
}

fn interval_power_horner(
    base: &DyadicEnclosure,
    exponent: u32,
) -> Result<DyadicEnclosure, CalculusFailure> {
    let mut result = DyadicEnclosure::new(
        Dyadic::new(BigInt::try_from(1_u8)?, 0),
        Dyadic::new(BigInt::try_from(1_u8)?, 0),
    )?;
    for _ in 0..exponent {
        result = enclosure_mul(&result, base)?;
    }
    Ok(result)
}

fn enclosure_add(
    left: &DyadicEnclosure,
    right: &DyadicEnclosure,
) -> Result<DyadicEnclosure, BigintError> {
    DyadicEnclosure::new(
        left.lower().add(right.lower())?,
        left.upper().add(right.upper())?,
    )
}

fn enclosure_mul(
    left: &DyadicEnclosure,
    right: &DyadicEnclosure,
) -> Result<DyadicEnclosure, BigintError> {
    let candidates = [
        left.lower().mul(right.lower())?,
        left.lower().mul(right.upper())?,
        left.upper().mul(right.lower())?,
        left.upper().mul(right.upper())?,
    ];
    let mut lower = candidates[0].try_clone()?;
    let mut upper = candidates[0].try_clone()?;
    for candidate in candidates.iter().skip(1) {
        if candidate.cmp(&lower) == Ordering::Less {
            lower = candidate.try_clone()?;
        }
        if candidate.cmp(&upper) == Ordering::Greater {
            upper = candidate.try_clone()?;
        }
    }
    DyadicEnclosure::new(lower, upper)
}

fn enclosure_div(
    numerator: &DyadicEnclosure,
    denominator: &DyadicEnclosure,
    precision_bits: u32,
) -> Result<DyadicEnclosure, BigintError> {
    let candidates = [
        dyadic_to_rational(numerator.lower())?.div(&dyadic_to_rational(denominator.lower())?)?,
        dyadic_to_rational(numerator.lower())?.div(&dyadic_to_rational(denominator.upper())?)?,
        dyadic_to_rational(numerator.upper())?.div(&dyadic_to_rational(denominator.lower())?)?,
        dyadic_to_rational(numerator.upper())?.div(&dyadic_to_rational(denominator.upper())?)?,
    ];
    let mut lower = candidates[0].try_clone()?;
    let mut upper = candidates[0].try_clone()?;
    for candidate in candidates.iter().skip(1) {
        if candidate.cmp(&lower) == Ordering::Less {
            lower = candidate.try_clone()?;
        }
        if candidate.cmp(&upper) == Ordering::Greater {
            upper = candidate.try_clone()?;
        }
    }
    DyadicEnclosure::new(
        lower.dyadic_floor(precision_bits)?,
        upper.dyadic_ceil(precision_bits)?,
    )
}

fn dyadic_to_rational(value: &Dyadic) -> Result<ReducedRational, BigintError> {
    let denominator = BigUint::one()?.shl_bits(value.exponent() as usize)?;
    RawRational::new(value.integer().try_clone()?, denominator)
        .reduce()
        .map(|reduction| reduction.into_reduced())
}

pub fn evaluate_surface(
    input: SurfaceEvaluationInput,
) -> Result<SurfaceEvaluationProducer, CalculusFailure> {
    let denominator = evaluate_surface_polynomial(&input.denominator, &input.u, &input.v)?;
    if denominator.is_zero() {
        return Err(CalculusFailure::Rational(RationalFailure::UndefinedDomain));
    }
    let x = evaluate_surface_polynomial(&input.x, &input.u, &input.v)?.div(&denominator)?;
    let y = evaluate_surface_polynomial(&input.y, &input.u, &input.v)?.div(&denominator)?;
    let z = evaluate_surface_polynomial(&input.z, &input.u, &input.v)?.div(&denominator)?;
    Ok(SurfaceEvaluationProducer::Exact { x, y, z })
}

fn evaluate_surface_polynomial(
    polynomial: &SurfacePolynomial,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<RealAlgebraic, CalculusFailure> {
    let mut result = RealAlgebraic::from_integer(neco_bigint::BigInt::zero())?;
    for term in &polynomial.terms {
        let coefficient = RealAlgebraic::from_reduced_rational(&term.coefficient)?;
        let u_power = algebraic_power(u, term.exponent_u)?;
        let v_power = algebraic_power(v, term.exponent_v)?;
        let term_value = u_power.mul(&v_power)?.mul(&coefficient)?;
        result = result.add(&term_value)?;
    }
    Ok(result)
}

fn algebraic_power(base: &RealAlgebraic, exponent: u32) -> Result<RealAlgebraic, CalculusFailure> {
    if exponent == 0 {
        Ok(RealAlgebraic::from_integer(BigInt::try_from(1_u8)?)?)
    } else {
        Ok(base.pow_integer(&BigInt::try_from(exponent)?)?)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ImplicitTerm {
    pub coefficient: RealAlgebraic,
    pub exponent_x: u32,
    pub exponent_y: u32,
    pub exponent_z: u32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ImplicitPolynomial3 {
    terms: Vec<ImplicitTerm>,
}

impl ImplicitPolynomial3 {
    pub fn try_from_terms(mut terms: Vec<ImplicitTerm>) -> Result<Self, CalculusFailure> {
        if terms.len() > MAX_IMPLICIT_TERMS {
            return Err(EvaluationFailure::ImplicitTermLimitExceeded {
                requested: terms.len(),
                maximum: MAX_IMPLICIT_TERMS,
            }
            .into());
        }
        terms.sort_unstable_by_key(|term| (term.exponent_x, term.exponent_y, term.exponent_z));
        if terms.iter().any(|term| term.coefficient.is_zero()) {
            return Err(EvaluationFailure::ZeroImplicitTerm.into());
        }
        if terms.windows(2).any(|pair| {
            (pair[0].exponent_x, pair[0].exponent_y, pair[0].exponent_z)
                == (pair[1].exponent_x, pair[1].exponent_y, pair[1].exponent_z)
        }) {
            return Err(EvaluationFailure::DuplicateImplicitExponent.into());
        }
        Ok(Self { terms })
    }

    pub fn terms(&self) -> &[ImplicitTerm] {
        &self.terms
    }
}

pub struct ImplicitEvaluationInput {
    pub equation: ImplicitPolynomial3,
    pub x: RealAlgebraic,
    pub y: RealAlgebraic,
    pub z: RealAlgebraic,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ImplicitEvaluationProducer {
    Exact { scalar: RealAlgebraic },
}

pub fn evaluate_implicit(
    input: ImplicitEvaluationInput,
) -> Result<ImplicitEvaluationProducer, CalculusFailure> {
    let mut scalar = RealAlgebraic::from_integer(neco_bigint::BigInt::zero())?;
    for term in input.equation.terms {
        let value = input
            .x
            .pow_integer(&neco_bigint::BigInt::try_from(term.exponent_x)?)?
            .mul(
                &input
                    .y
                    .pow_integer(&neco_bigint::BigInt::try_from(term.exponent_y)?)?,
            )?
            .mul(
                &input
                    .z
                    .pow_integer(&neco_bigint::BigInt::try_from(term.exponent_z)?)?,
            )?
            .mul(&term.coefficient)?;
        scalar = scalar.add(&value)?;
    }
    Ok(ImplicitEvaluationProducer::Exact { scalar })
}

pub fn refine_root_bisection(
    bracket: &RootBracket,
    iterations: usize,
) -> Result<RootIsolation, CalculusFailure> {
    Ok(bracket.refine_bisection(iterations)?)
}

pub fn refine_root_newton_bisection(
    bracket: &RootBracket,
    iterations: usize,
    precision_bits: u32,
) -> Result<RootIsolation, CalculusFailure> {
    Ok(bracket.refine_newton_bisection(iterations, precision_bits)?)
}

pub fn construct_transcendental_evaluation(
    input: TranscendentalEvaluationConstructionInput,
) -> Result<TranscendentalEvaluation, CalculusFailure> {
    let TranscendentalEvaluationConstructionInput { candidate, request } = input;
    let TranscendentalEvaluationRequest {
        owner,
        requirement,
        source,
    } = request;
    let requested = requirement.precision();
    if requested.get() == 0 || requested.get() > MAX_PRECISION_BITS {
        return Err(precision_limit_failure(
            EvaluationOperation::Transcendental,
            owner,
            requirement,
            source,
            EvaluationInputSnapshot::Transcendental { candidate },
            requested,
        ));
    }
    if let Err(error) = candidate_matches_request(&candidate, owner, &requirement, source) {
        return Err(evaluation_expression_failure(
            EvaluationOperation::Transcendental,
            owner,
            requirement,
            source,
            candidate,
            error,
        ));
    }
    if let TranscendentalEvaluationCandidate::Enclosure { enclosure, .. } = &candidate {
        let maximum_width = Dyadic::new(BigInt::try_from(1_u8)?, requested.get());
        let width = enclosure.width()?;
        if width > maximum_width {
            let witness = clone_enclosure_value(enclosure)?;
            let location = precision_shortage_location(&requirement);
            let lower = PrecisionShortageCause::Exhausted(NecoFailure::PrecisionExhaustion {
                location,
                requested,
                payload: Box::new(PrecisionFailurePayload::EnclosureWidth {
                    actual_width: width,
                }),
            });
            return Err(EvaluationFailure::PrecisionShortage {
                operation: EvaluationOperation::Transcendental,
                owner,
                requirement,
                source,
                input: EvaluationInputSnapshot::Transcendental { candidate },
                requested,
                witness: PrecisionWitness::Enclosure { enclosure: witness },
                lower,
            }
            .into());
        }
    }
    match candidate {
        TranscendentalEvaluationCandidate::ExactAlgebraic { value, .. } => {
            Ok(TranscendentalEvaluation {
                value: TranscendentalEvaluationValue::ExactAlgebraic {
                    value,
                    owner,
                    requirement,
                    source,
                },
            })
        }
        TranscendentalEvaluationCandidate::Enclosure { enclosure, .. } => {
            Ok(TranscendentalEvaluation {
                value: TranscendentalEvaluationValue::Enclosure {
                    enclosure,
                    owner,
                    requirement,
                    source,
                },
            })
        }
        TranscendentalEvaluationCandidate::Undetermined { achieved, .. } => {
            Ok(TranscendentalEvaluation {
                value: TranscendentalEvaluationValue::Undetermined {
                    achieved,
                    owner,
                    requirement,
                    source,
                },
            })
        }
    }
}

fn precision_limit_failure(
    operation: EvaluationOperation,
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
    input: EvaluationInputSnapshot,
    requested: AbsoluteBits,
) -> CalculusFailure {
    EvaluationFailure::PrecisionShortage {
        operation,
        owner,
        requirement,
        source,
        input,
        requested,
        witness: PrecisionWitness::Limit {
            maximum: MAX_PRECISION_BITS,
        },
        lower: PrecisionShortageCause::RequestedBeyondLimit {
            requested,
            maximum: MAX_PRECISION_BITS,
        },
    }
    .into()
}

const SERIES_WORKING_GUARD_BITS: u32 = 16;
const SERIES_GUARD_BITS: u32 = 96;
const COARSE_BITS: u32 = 32;
const MAX_BINARY_EXPONENT: u32 = 1 << 20;

#[derive(Debug)]
enum SeriesFailure {
    Arithmetic(BigintError),
    PrecisionLimit { requested: u32, maximum: u32 },
    ReductionRange,
}

impl From<BigintError> for SeriesFailure {
    fn from(error: BigintError) -> Self {
        Self::Arithmetic(error)
    }
}

impl From<SeriesFailure> for CalculusFailure {
    fn from(error: SeriesFailure) -> Self {
        match error {
            SeriesFailure::Arithmetic(error) => Self::Arithmetic(error),
            SeriesFailure::PrecisionLimit { requested, maximum } => {
                Self::Polynomial(PolynomialFailure::Resource(
                    PolynomialResourceFailure::PrecisionLimitExceeded { requested, maximum },
                ))
            }
            SeriesFailure::ReductionRange => {
                Self::Interval(IntervalFailure::Arithmetic(BigintError::InvalidInterval))
            }
        }
    }
}

#[derive(Clone, Copy)]
enum SeriesKind {
    Sine,
    Cosine,
    Exponential,
    DoubledArtanh,
}

enum TranscendentalOutcome {
    Exact(u8),
    Enclosure(Enclosure),
    Undetermined(Enclosure),
}

const SINE_QUARTER_TURNS: [(SeriesKind, bool); 4] = [
    (SeriesKind::Sine, false),
    (SeriesKind::Cosine, false),
    (SeriesKind::Sine, true),
    (SeriesKind::Cosine, true),
];

const COSINE_QUARTER_TURNS: [(SeriesKind, bool); 4] = [
    (SeriesKind::Cosine, false),
    (SeriesKind::Sine, true),
    (SeriesKind::Cosine, true),
    (SeriesKind::Sine, false),
];

/// Evaluates a sine, cosine, or exponential over an interval argument.
/// Returns an undetermined result when the width estimate exceeds the target
/// width. The interval kept by that result may still meet the target width.
pub fn evaluate_transcendental_function(
    input: TranscendentalFunctionEvaluationInput,
) -> Result<TranscendentalEvaluation, CalculusFailure> {
    let TranscendentalFunctionEvaluationInput {
        function,
        argument,
        request,
    } = input;
    let TranscendentalEvaluationRequest {
        owner,
        requirement,
        source,
    } = request;
    let operation = EvaluationOperation::TranscendentalFunction(function);
    let requested = requirement.precision();
    if requested.get() == 0 || requested.get() > MAX_PRECISION_BITS {
        return Err(precision_limit_failure(
            operation,
            owner,
            requirement,
            source,
            EvaluationInputSnapshot::TranscendentalFunction { function, argument },
            requested,
        ));
    }
    let outcome = match function_outcome(function, &argument, requested) {
        Ok(outcome) => outcome,
        Err(error) => {
            return Err(evaluation_lower_failure(
                operation,
                owner,
                requirement,
                source,
                EvaluationInputSnapshot::TranscendentalFunction { function, argument },
                error.into(),
            ))
        }
    };
    construct_from_outcome(
        operation,
        EvaluationInputSnapshot::TranscendentalFunction { function, argument },
        (owner, requirement, source),
        outcome,
    )
    .map_err(|error| *error)
}

/// Evaluates a real power of a strictly positive base interval.
/// A base reaching zero or below fails with a domain violation.
pub fn evaluate_transcendental_power(
    input: TranscendentalPowerEvaluationInput,
) -> Result<TranscendentalEvaluation, CalculusFailure> {
    let TranscendentalPowerEvaluationInput {
        base,
        exponent,
        request,
    } = input;
    let TranscendentalEvaluationRequest {
        owner,
        requirement,
        source,
    } = request;
    let operation = EvaluationOperation::TranscendentalPower;
    let requested = requirement.precision();
    if requested.get() == 0 || requested.get() > MAX_PRECISION_BITS {
        return Err(precision_limit_failure(
            operation,
            owner,
            requirement,
            source,
            EvaluationInputSnapshot::TranscendentalPower { base, exponent },
            requested,
        ));
    }
    if base.lower().integer().sign() != Sign::Positive {
        let witness = match (
            clone_enclosure_value(&base),
            clone_enclosure_value(&exponent),
        ) {
            (Ok(base), Ok(exponent)) => DomainWitness::NonPositiveBase { base, exponent },
            (Err(error), _) | (_, Err(error)) => {
                return Err(evaluation_lower_failure(
                    operation,
                    owner,
                    requirement,
                    source,
                    EvaluationInputSnapshot::TranscendentalPower { base, exponent },
                    error.into(),
                ))
            }
        };
        return Err(EvaluationFailure::TranscendentalDomainViolation {
            operation,
            owner,
            requirement,
            source,
            input: EvaluationInputSnapshot::TranscendentalPower { base, exponent },
            witness,
        }
        .into());
    }
    let outcome = match power_outcome(&base, &exponent, requested) {
        Ok(outcome) => outcome,
        Err(error) => {
            return Err(evaluation_lower_failure(
                operation,
                owner,
                requirement,
                source,
                EvaluationInputSnapshot::TranscendentalPower { base, exponent },
                error.into(),
            ))
        }
    };
    construct_from_outcome(
        operation,
        EvaluationInputSnapshot::TranscendentalPower { base, exponent },
        (owner, requirement, source),
        outcome,
    )
    .map_err(|error| *error)
}

fn construct_from_outcome(
    operation: EvaluationOperation,
    input: EvaluationInputSnapshot,
    lineage: (
        OwnerId,
        ExactExpressionRequirement,
        NecoImplementationSource,
    ),
    outcome: TranscendentalOutcome,
) -> Result<TranscendentalEvaluation, Box<CalculusFailure>> {
    let (owner, requirement, source) = lineage;
    let candidate_requirement = match ExactExpressionRequirement::new(
        requirement.consumer_id(),
        requirement.expressions(),
        requirement.decision(),
        requirement.precision(),
    ) {
        Ok(duplicate) => duplicate,
        Err(error) => {
            return Err(Box::new(
                EvaluationFailure::Lower {
                    operation,
                    owner,
                    requirement,
                    source,
                    input,
                    witness: LowerWitness::new(operation),
                    lower: LowerEvaluationFailure::Expression(error),
                }
                .into(),
            ))
        }
    };
    let candidate = match outcome {
        TranscendentalOutcome::Exact(integer) => {
            let integer = match BigInt::try_from(integer) {
                Ok(integer) => integer,
                Err(error) => {
                    return Err(Box::new(evaluation_lower_failure(
                        operation,
                        owner,
                        requirement,
                        source,
                        input,
                        error.into(),
                    )))
                }
            };
            let value = match RealAlgebraic::from_integer(integer) {
                Ok(value) => value,
                Err(error) => {
                    return Err(Box::new(evaluation_lower_failure(
                        operation,
                        owner,
                        requirement,
                        source,
                        input,
                        error.into(),
                    )))
                }
            };
            TranscendentalEvaluationCandidate::ExactAlgebraic {
                value,
                owner,
                requirement: candidate_requirement,
                source,
            }
        }
        TranscendentalOutcome::Enclosure(enclosure) => {
            TranscendentalEvaluationCandidate::Enclosure {
                enclosure,
                owner,
                requirement: candidate_requirement,
                source,
            }
        }
        TranscendentalOutcome::Undetermined(achieved) => {
            TranscendentalEvaluationCandidate::Undetermined {
                achieved,
                owner,
                requirement: candidate_requirement,
                source,
            }
        }
    };
    construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
        candidate,
        TranscendentalEvaluationRequest::new(owner, requirement, source),
    ))
    .map_err(Box::new)
}

fn function_outcome(
    function: TranscendentalFunction,
    argument: &Enclosure,
    requested: AbsoluteBits,
) -> Result<TranscendentalOutcome, SeriesFailure> {
    if argument.lower().is_zero() && argument.upper().is_zero() {
        let exact = match function {
            TranscendentalFunction::Sine => 0_u8,
            TranscendentalFunction::Cosine | TranscendentalFunction::Exponential => 1_u8,
        };
        return Ok(TranscendentalOutcome::Exact(exact));
    }
    let precision = requested.get();
    let value = function_enclosure(
        function,
        argument.as_dyadic_enclosure(),
        precision + SERIES_GUARD_BITS,
    )?;
    let width = argument.as_dyadic_enclosure().width()?;
    let estimate = estimate_reachable_width(function, &width, &value)?;
    let value = Enclosure::from_dyadic_enclosure(value);
    if estimate_reaches_target(&estimate, precision)? {
        Ok(TranscendentalOutcome::Enclosure(value))
    } else {
        Ok(TranscendentalOutcome::Undetermined(value))
    }
}

fn power_outcome(
    base: &Enclosure,
    exponent: &Enclosure,
    requested: AbsoluteBits,
) -> Result<TranscendentalOutcome, SeriesFailure> {
    let one = Dyadic::new(BigInt::one()?, 0);
    let base_is_one = base.lower() == &one && base.upper() == &one;
    if base_is_one || (exponent.lower().is_zero() && exponent.upper().is_zero()) {
        return Ok(TranscendentalOutcome::Exact(1));
    }
    let precision = requested.get();
    let bits = precision + SERIES_GUARD_BITS;
    let coarse = enclosure_mul(
        &natural_log_interval(base, COARSE_BITS)?,
        exponent.as_dyadic_enclosure(),
    )?;
    let growth = round_dyadic(coarse.upper(), 0, true)?;
    let growth = if growth.integer().sign() == Sign::Positive {
        unsigned_bound(growth.integer())?
    } else {
        0
    };
    let exponent_ceiling = round_dyadic(&max_magnitude(exponent.as_dyadic_enclosure())?, 0, true)?;
    let exponent_bits = exponent_ceiling.integer().magnitude().bit_len() as u64;
    let log_bits = bits + bounded_limit(growth + growth / 2 + exponent_bits + 4)?;
    let logarithm = natural_log_interval(base, log_bits)?;
    let product = round_enclosure(
        &enclosure_mul(&logarithm, exponent.as_dyadic_enclosure())?,
        log_bits,
    )?;
    let value = exponential_interval(&product, bits)?;

    let base_width = base.as_dyadic_enclosure().width()?;
    let exponent_width = exponent.as_dyadic_enclosure().width()?;
    let base_term = base_width
        .mul(&max_magnitude(exponent.as_dyadic_enclosure())?)?
        .mul(value.upper())?;
    let base_term = enclosure_div(
        &point_enclosure(&base_term)?,
        &point_enclosure(base.lower())?,
        bits,
    )?;
    let exponent_term = exponent_width
        .mul(value.upper())?
        .mul(&max_magnitude(&logarithm)?)?;
    let estimate = base_term.upper().add(&exponent_term)?;
    let value = Enclosure::from_dyadic_enclosure(value);
    if estimate_reaches_target(&estimate, precision)? {
        Ok(TranscendentalOutcome::Enclosure(value))
    } else {
        Ok(TranscendentalOutcome::Undetermined(value))
    }
}

fn estimate_reaches_target(estimate: &Dyadic, precision: u32) -> Result<bool, BigintError> {
    let slack = Dyadic::new(BigInt::one()?, precision + SERIES_GUARD_BITS / 2);
    let target = Dyadic::new(BigInt::one()?, precision);
    Ok(estimate.add(&slack)? <= target)
}

fn estimate_reachable_width(
    function: TranscendentalFunction,
    width: &Dyadic,
    value: &DyadicEnclosure,
) -> Result<Dyadic, BigintError> {
    match function {
        TranscendentalFunction::Sine | TranscendentalFunction::Cosine => value.width(),
        TranscendentalFunction::Exponential => width.mul(value.upper()),
    }
}

fn binary_magnitude(value: &Dyadic) -> i64 {
    value.integer().magnitude().bit_len() as i64 - i64::from(value.exponent())
}

fn unsigned_bound(value: &BigInt) -> Result<u64, BigintError> {
    match value.magnitude().to_u32() {
        Some(bound) => Ok(u64::from(bound)),
        None => Err(BigintError::ExponentOverflow {
            required: value.magnitude().try_clone()?,
            maximum: MAX_BINARY_EXPONENT,
        }),
    }
}

fn bounded_limit(value: u64) -> Result<u32, BigintError> {
    match u32::try_from(value) {
        Ok(bounded) if bounded <= MAX_BINARY_EXPONENT => Ok(bounded),
        _ => Err(BigintError::ExponentOverflow {
            required: BigUint::try_from(value)?,
            maximum: MAX_BINARY_EXPONENT,
        }),
    }
}

fn integer_part_bits(value: &Dyadic) -> u64 {
    (value.integer().magnitude().bit_len() as u64).saturating_sub(u64::from(value.exponent()))
}

fn power_of_two(exponent: u32) -> Result<BigInt, BigintError> {
    Ok(BigInt::from_sign_magnitude(
        Sign::Positive,
        BigUint::one()?.shl_bits(exponent as usize)?,
    ))
}

fn round_dyadic(value: &Dyadic, bits: u32, upward: bool) -> Result<Dyadic, BigintError> {
    if value.exponent() <= bits {
        return value.try_clone();
    }
    let divisor = power_of_two(value.exponent() - bits)?;
    let (mut quotient, remainder) = value.integer().div_rem_euclid(&divisor)?;
    if upward && !remainder.is_zero() {
        quotient = quotient.add(&BigInt::one()?)?;
    }
    Ok(Dyadic::new(quotient, bits))
}

fn round_enclosure(enclosure: &DyadicEnclosure, bits: u32) -> Result<DyadicEnclosure, BigintError> {
    DyadicEnclosure::new(
        round_dyadic(enclosure.lower(), bits, false)?,
        round_dyadic(enclosure.upper(), bits, true)?,
    )
}

fn scale_by_power_of_two(value: &Dyadic, shift: i64) -> Result<Dyadic, BigintError> {
    if value.is_zero() {
        return value.try_clone();
    }
    let exponent = i64::from(value.exponent());
    let remaining = exponent - shift;
    if remaining >= 0 {
        let Ok(remaining_exponent) = u32::try_from(remaining) else {
            return Err(BigintError::ExponentOverflow {
                required: BigUint::try_from(remaining.unsigned_abs())?,
                maximum: u32::MAX,
            });
        };
        return Ok(Dyadic::new(
            value.integer().try_clone()?,
            remaining_exponent,
        ));
    }
    let left =
        usize::try_from(remaining.unsigned_abs()).map_err(|_| BigintError::CapacityOverflow)?;
    Ok(Dyadic::new(
        BigInt::from_sign_magnitude(
            value.integer().sign(),
            value.integer().magnitude().shl_bits(left)?,
        ),
        0,
    ))
}

fn point_enclosure(value: &Dyadic) -> Result<DyadicEnclosure, BigintError> {
    DyadicEnclosure::new(value.try_clone()?, value.try_clone()?)
}

fn negate_dyadic(value: &Dyadic) -> Result<Dyadic, BigintError> {
    Ok(Dyadic::new(value.integer().negated()?, value.exponent()))
}

fn negate_enclosure(enclosure: &DyadicEnclosure) -> Result<DyadicEnclosure, BigintError> {
    DyadicEnclosure::new(
        negate_dyadic(enclosure.upper())?,
        negate_dyadic(enclosure.lower())?,
    )
}

fn enclosure_sub(
    left: &DyadicEnclosure,
    right: &DyadicEnclosure,
) -> Result<DyadicEnclosure, BigintError> {
    enclosure_add(left, &negate_enclosure(right)?)
}

fn max_magnitude(enclosure: &DyadicEnclosure) -> Result<Dyadic, BigintError> {
    let negated_lower = negate_dyadic(enclosure.lower())?;
    if negated_lower > *enclosure.upper() {
        Ok(negated_lower)
    } else {
        enclosure.upper().try_clone()
    }
}

fn inverse_series_fixed_point(
    divisor: u32,
    scale: u32,
    alternating: bool,
) -> Result<(BigInt, u64), BigintError> {
    let square = BigInt::try_from(u64::from(divisor) * u64::from(divisor))?;
    let mut power = power_of_two(scale)?
        .div_rem_euclid(&BigInt::try_from(divisor)?)?
        .0;
    let mut sum = BigInt::zero();
    let mut count = 0_u64;
    let mut denominator = 1_u64;
    while !power.is_zero() {
        let term = power.div_rem_euclid(&BigInt::try_from(denominator)?)?.0;
        sum = if alternating && count % 2 == 1 {
            sum.sub(&term)?
        } else {
            sum.add(&term)?
        };
        power = power.div_rem_euclid(&square)?.0;
        denominator += 2;
        count += 1;
    }
    Ok((sum, count))
}

fn pi_enclosure(bits: u32) -> Result<DyadicEnclosure, BigintError> {
    let scale = bits + 24;
    let (fifth, fifth_terms) = inverse_series_fixed_point(5, scale, true)?;
    let (small, small_terms) = inverse_series_fixed_point(239, scale, true)?;
    let center = fifth
        .mul(&BigInt::try_from(16_u8)?)?
        .sub(&small.mul(&BigInt::try_from(4_u8)?)?)?;
    let error = BigInt::try_from(16 * (fifth_terms + 2) + 4 * (small_terms + 2))?;
    DyadicEnclosure::new(
        Dyadic::new(center.sub(&error)?, scale),
        Dyadic::new(center.add(&error)?, scale),
    )
}

fn ln2_enclosure(bits: u32) -> Result<DyadicEnclosure, BigintError> {
    let scale = bits + 24;
    let (sum, terms) = inverse_series_fixed_point(3, scale, false)?;
    let center = sum.mul(&BigInt::try_from(2_u8)?)?;
    let error = BigInt::try_from(2 * (terms + 2))?;
    DyadicEnclosure::new(
        Dyadic::new(center.sub(&error)?, scale),
        Dyadic::new(center.add(&error)?, scale),
    )
}

fn series_terms(kind: SeriesKind, bits: u32) -> Result<u32, SeriesFailure> {
    if let SeriesKind::DoubledArtanh = kind {
        let terms = (bits + 4).div_ceil(4);
        if terms > MAX_TRANSCENDENTAL_SERIES_TERMS {
            return Err(SeriesFailure::PrecisionLimit {
                requested: bits,
                maximum: MAX_TRANSCENDENTAL_SERIES_TERMS * 4 - 4,
            });
        }
        return Ok(terms);
    }
    let growth = match kind {
        SeriesKind::Exponential => 2,
        _ => 0,
    };
    let mut factorial = BigUint::one()?;
    for terms in 1..=MAX_TRANSCENDENTAL_SERIES_TERMS {
        factorial = factorial.mul(&BigUint::try_from(terms)?)?;
        if factorial.bit_len() as u64 >= u64::from(bits) + 5 + growth {
            return Ok(terms);
        }
    }
    let capacity = (factorial.bit_len() as u64).saturating_sub(5 + growth);
    Err(SeriesFailure::PrecisionLimit {
        requested: bits,
        maximum: u32::try_from(capacity).unwrap_or(u32::MAX),
    })
}

fn shift_left(value: &BigInt, bits: u32) -> Result<BigInt, BigintError> {
    Ok(BigInt::from_sign_magnitude(
        value.sign(),
        value.magnitude().shl_bits(bits as usize)?,
    ))
}

fn divide_dyadic(
    value: &Dyadic,
    divisor: u64,
    bits: u32,
    upward: bool,
) -> Result<Dyadic, BigintError> {
    let divisor = BigInt::try_from(divisor)?;
    let (numerator, denominator) = if value.exponent() <= bits {
        (
            shift_left(value.integer(), bits - value.exponent())?,
            divisor,
        )
    } else {
        (
            value.integer().try_clone()?,
            divisor.mul(&power_of_two(value.exponent() - bits)?)?,
        )
    };
    let (mut quotient, remainder) = numerator.div_rem_euclid(&denominator)?;
    if upward && !remainder.is_zero() {
        quotient = quotient.add(&BigInt::one()?)?;
    }
    Ok(Dyadic::new(quotient, bits))
}

fn divide_enclosure(
    enclosure: &DyadicEnclosure,
    divisor: u64,
    bits: u32,
) -> Result<DyadicEnclosure, BigintError> {
    DyadicEnclosure::new(
        divide_dyadic(enclosure.lower(), divisor, bits, false)?,
        divide_dyadic(enclosure.upper(), divisor, bits, true)?,
    )
}

fn evaluate_bounded_series(
    kind: SeriesKind,
    domain: &DyadicEnclosure,
    bits: u32,
) -> Result<DyadicEnclosure, SeriesFailure> {
    let bound = match kind {
        SeriesKind::DoubledArtanh => Dyadic::new(BigInt::one()?, 2),
        _ => Dyadic::new(BigInt::one()?, 0),
    };
    if max_magnitude(domain)? > bound {
        return Err(SeriesFailure::ReductionRange);
    }
    let terms = series_terms(kind, bits)?;
    let working = bits + SERIES_WORKING_GUARD_BITS;
    let one = point_enclosure(&Dyadic::new(BigInt::one()?, 0))?;
    let value = match kind {
        SeriesKind::Exponential => {
            let mut accumulator = one.try_clone()?;
            for divisor in (1..terms).rev() {
                let step = divide_enclosure(
                    &enclosure_mul(domain, &accumulator)?,
                    u64::from(divisor),
                    working,
                )?;
                accumulator = round_enclosure(&enclosure_add(&one, &step)?, working)?;
            }
            accumulator
        }
        SeriesKind::Sine | SeriesKind::Cosine => {
            let sine = matches!(kind, SeriesKind::Sine);
            let square = round_enclosure(&enclosure_mul(domain, domain)?, working)?;
            let mut accumulator = one.try_clone()?;
            for index in (1..=(terms - 1) / 2).rev() {
                let index = u64::from(index);
                let divisor = if sine {
                    2 * index * (2 * index + 1)
                } else {
                    (2 * index - 1) * (2 * index)
                };
                let step =
                    divide_enclosure(&enclosure_mul(&square, &accumulator)?, divisor, working)?;
                accumulator = round_enclosure(&enclosure_sub(&one, &step)?, working)?;
            }
            if sine {
                round_enclosure(&enclosure_mul(domain, &accumulator)?, working)?
            } else {
                accumulator
            }
        }
        SeriesKind::DoubledArtanh => {
            let square = round_enclosure(&enclosure_mul(domain, domain)?, working)?;
            let last = u64::from(terms - 1);
            let mut accumulator = divide_enclosure(&one, 2 * last + 1, working)?;
            for index in (0..last).rev() {
                let carried = enclosure_mul(&square, &accumulator)?;
                accumulator = round_enclosure(
                    &enclosure_add(&divide_enclosure(&one, 2 * index + 1, working)?, &carried)?,
                    working,
                )?;
            }
            let product = enclosure_mul(domain, &accumulator)?;
            round_enclosure(&enclosure_add(&product, &product)?, working)?
        }
    };
    let remainder = Dyadic::new(BigInt::one()?, bits + 2);
    let lower = value.lower().sub(&remainder)?;
    let upper = value.upper().add(&remainder)?;
    Ok(round_enclosure(&DyadicEnclosure::new(lower, upper)?, bits)?)
}

fn nearest_integer_ratio(
    numerator: &Dyadic,
    scale: &DyadicEnclosure,
) -> Result<BigInt, BigintError> {
    let quotient = enclosure_div(&point_enclosure(numerator)?, scale, 4)?;
    let half = Dyadic::new(BigInt::one()?, 1);
    let nearest = round_dyadic(&quotient.lower().add(&half)?, 0, false)?;
    nearest.integer().try_clone()
}

fn trigonometric_point(
    sine: bool,
    x: &Dyadic,
    bits: u32,
) -> Result<DyadicEnclosure, SeriesFailure> {
    let point = round_enclosure(&point_enclosure(x)?, bits)?;
    let magnitude_bits = bounded_limit(integer_part_bits(x))?;
    let pi = pi_enclosure(bits + magnitude_bits + 16)?;
    let multiple = nearest_integer_ratio(&scale_by_power_of_two(point.lower(), 1)?, &pi)?;
    let count = point_enclosure(&Dyadic::new(multiple.try_clone()?, 0))?;
    let turns = enclosure_mul(&count, &pi)?;
    let turns = DyadicEnclosure::new(
        scale_by_power_of_two(turns.lower(), -1)?,
        scale_by_power_of_two(turns.upper(), -1)?,
    )?;
    let reduced = round_enclosure(&enclosure_sub(&point, &turns)?, bits)?;
    let (_, remainder) = multiple.div_rem_euclid(&BigInt::try_from(4_u8)?)?;
    let phase = remainder.limbs_le().first().copied().unwrap_or(0) as usize & 3;
    let table = if sine {
        &SINE_QUARTER_TURNS
    } else {
        &COSINE_QUARTER_TURNS
    };
    let (kind, negate) = table[phase];
    let value = evaluate_bounded_series(kind, &reduced, bits)?;
    if negate {
        Ok(negate_enclosure(&value)?)
    } else {
        Ok(value)
    }
}

fn dyadic_ratio(numerator: &Dyadic, denominator: &Dyadic) -> Result<(BigInt, BigInt), BigintError> {
    let mut numerator_integer = numerator.integer().try_clone()?;
    let mut denominator_integer = BigInt::from_sign_magnitude(
        Sign::Positive,
        denominator.integer().magnitude().try_clone()?,
    );
    if denominator.exponent() >= numerator.exponent() {
        numerator_integer = shift_left(
            &numerator_integer,
            denominator.exponent() - numerator.exponent(),
        )?;
    } else {
        denominator_integer = shift_left(
            &denominator_integer,
            numerator.exponent() - denominator.exponent(),
        )?;
    }
    Ok((numerator_integer, denominator_integer))
}

fn integer_ratio_bound(
    numerator: &Dyadic,
    denominator: &Dyadic,
    half_shift: bool,
    upward: bool,
) -> Result<BigInt, BigintError> {
    let (mut numerator, mut denominator) = dyadic_ratio(numerator, denominator)?;
    if half_shift {
        numerator = shift_left(&numerator, 1)?.sub(&denominator)?;
        denominator = shift_left(&denominator, 1)?;
    }
    let (mut quotient, remainder) = numerator.div_rem_euclid(&denominator)?;
    if upward && !remainder.is_zero() {
        quotient = quotient.add(&BigInt::one()?)?;
    }
    Ok(quotient)
}

fn trigonometric_extrema(
    sine: bool,
    argument: &DyadicEnclosure,
    bits: u32,
) -> Result<(bool, bool), SeriesFailure> {
    let magnitude_bits = bounded_limit(
        integer_part_bits(argument.lower()).max(integer_part_bits(argument.upper())),
    )?;
    let pi_bits = bits
        .checked_add(magnitude_bits)
        .and_then(|value| value.checked_add(16))
        .ok_or(BigintError::CapacityOverflow)?;
    let pi_bits = bounded_limit(u64::from(pi_bits))?;
    let pi = pi_enclosure(pi_bits)?;
    let lower_pi = if argument.lower().integer().sign() == Sign::Negative {
        pi.lower()
    } else {
        pi.upper()
    };
    let upper_pi = if argument.upper().integer().sign() == Sign::Negative {
        pi.upper()
    } else {
        pi.lower()
    };
    let lower_index = integer_ratio_bound(argument.lower(), lower_pi, sine, true)?;
    let upper_index = integer_ratio_bound(argument.upper(), upper_pi, sine, false)?;
    if upper_index < lower_index {
        return Ok((false, false));
    }
    if upper_index > lower_index {
        return Ok((true, true));
    }
    let odd = lower_index.magnitude().bit(0);
    Ok((!odd, odd))
}

fn trigonometric_interval(
    sine: bool,
    argument: &DyadicEnclosure,
    bits: u32,
) -> Result<DyadicEnclosure, SeriesFailure> {
    let lower_point = trigonometric_point(sine, argument.lower(), bits)?;
    if argument.lower() == argument.upper() {
        return Ok(lower_point);
    }
    let upper_point = trigonometric_point(sine, argument.upper(), bits)?;
    let mut lower = if lower_point.lower() < upper_point.lower() {
        lower_point.lower().try_clone()?
    } else {
        upper_point.lower().try_clone()?
    };
    let mut upper = if lower_point.upper() > upper_point.upper() {
        lower_point.upper().try_clone()?
    } else {
        upper_point.upper().try_clone()?
    };
    let (contains_maximum, contains_minimum) = trigonometric_extrema(sine, argument, bits)?;
    if contains_maximum {
        upper = Dyadic::new(BigInt::one()?, 0);
    }
    if contains_minimum {
        lower = Dyadic::new(BigInt::one()?.negated()?, 0);
    }
    Ok(DyadicEnclosure::new(lower, upper)?)
}

fn small_integer(value: &BigInt) -> Result<i64, BigintError> {
    let bound = i64::from(bounded_limit(unsigned_bound(value)?)?);
    Ok(if value.sign() == Sign::Negative {
        -bound
    } else {
        bound
    })
}

fn exponential_point(x: &Dyadic, bits: u32) -> Result<DyadicEnclosure, SeriesFailure> {
    let magnitude_bits = bounded_limit(integer_part_bits(x))?;
    let coarse_ln2 = ln2_enclosure(magnitude_bits + 16)?;
    let multiple = small_integer(&nearest_integer_ratio(x, &coarse_ln2)?)?;
    let working =
        bits + u32::try_from(multiple.max(0)).map_err(|_| BigintError::CapacityOverflow)?;
    series_terms(SeriesKind::Exponential, working)?;
    let point = round_enclosure(&point_enclosure(x)?, working)?;
    let ln2 = ln2_enclosure(working + magnitude_bits + 16)?;
    let count = point_enclosure(&Dyadic::new(BigInt::try_from(multiple)?, 0))?;
    let reduced = round_enclosure(
        &enclosure_sub(&point, &enclosure_mul(&count, &ln2)?)?,
        working,
    )?;
    let series = evaluate_bounded_series(SeriesKind::Exponential, &reduced, working)?;
    Ok(round_enclosure(
        &DyadicEnclosure::new(
            scale_by_power_of_two(series.lower(), multiple)?,
            scale_by_power_of_two(series.upper(), multiple)?,
        )?,
        bits,
    )?)
}

fn exponential_interval(
    argument: &DyadicEnclosure,
    bits: u32,
) -> Result<DyadicEnclosure, SeriesFailure> {
    let lower_point = exponential_point(argument.lower(), bits)?;
    if argument.lower() == argument.upper() {
        return Ok(lower_point);
    }
    let upper_point = exponential_point(argument.upper(), bits)?;
    Ok(DyadicEnclosure::new(
        lower_point.lower().try_clone()?,
        upper_point.upper().try_clone()?,
    )?)
}

fn natural_log_point(value: &Dyadic, bits: u32) -> Result<DyadicEnclosure, SeriesFailure> {
    let floor_log2 = binary_magnitude(value) - 1;
    let leading = scale_by_power_of_two(value, -floor_log2)?;
    let three_halves = Dyadic::new(BigInt::try_from(3_u8)?, 1);
    let (binary_exponent, mantissa) = if leading >= three_halves {
        (floor_log2 + 1, scale_by_power_of_two(&leading, -1)?)
    } else {
        (floor_log2, leading)
    };
    let mantissa = round_enclosure(&point_enclosure(&mantissa)?, bits)?;
    let one = point_enclosure(&Dyadic::new(BigInt::one()?, 0))?;
    let minus_one = point_enclosure(&Dyadic::new(BigInt::one()?.negated()?, 0))?;
    let ratio = enclosure_div(
        &enclosure_add(&mantissa, &minus_one)?,
        &enclosure_add(&mantissa, &one)?,
        bits,
    )?;
    let series = evaluate_bounded_series(SeriesKind::DoubledArtanh, &ratio, bits)?;
    let exponent_bits = 64 - binary_exponent.unsigned_abs().leading_zeros();
    let ln2 = ln2_enclosure(bits + exponent_bits + 16)?;
    let count = point_enclosure(&Dyadic::new(BigInt::try_from(binary_exponent)?, 0))?;
    Ok(round_enclosure(
        &enclosure_add(&series, &enclosure_mul(&count, &ln2)?)?,
        bits,
    )?)
}

fn natural_log_interval(base: &Enclosure, bits: u32) -> Result<DyadicEnclosure, SeriesFailure> {
    let lower = natural_log_point(base.lower(), bits)?;
    if base.lower() == base.upper() {
        return Ok(lower);
    }
    let upper = natural_log_point(base.upper(), bits)?;
    Ok(DyadicEnclosure::new(
        lower.lower().try_clone()?,
        upper.upper().try_clone()?,
    )?)
}

fn function_enclosure(
    function: TranscendentalFunction,
    argument: &DyadicEnclosure,
    bits: u32,
) -> Result<DyadicEnclosure, SeriesFailure> {
    match function {
        TranscendentalFunction::Sine => trigonometric_interval(true, argument, bits),
        TranscendentalFunction::Cosine => trigonometric_interval(false, argument, bits),
        TranscendentalFunction::Exponential => exponential_interval(argument, bits),
    }
}

fn candidate_matches_request(
    candidate: &TranscendentalEvaluationCandidate,
    owner: OwnerId,
    requirement: &ExactExpressionRequirement,
    source: NecoImplementationSource,
) -> Result<(), NecoFailure> {
    match candidate {
        TranscendentalEvaluationCandidate::ExactAlgebraic {
            owner: actual_owner,
            requirement: actual_requirement,
            source: actual_source,
            ..
        }
        | TranscendentalEvaluationCandidate::Enclosure {
            owner: actual_owner,
            requirement: actual_requirement,
            source: actual_source,
            ..
        }
        | TranscendentalEvaluationCandidate::Undetermined {
            owner: actual_owner,
            requirement: actual_requirement,
            source: actual_source,
            ..
        } => validate_lineage(
            owner,
            requirement,
            source,
            *actual_owner,
            actual_requirement,
            *actual_source,
        ),
    }
}

fn validate_lineage(
    expected_owner: OwnerId,
    expected_requirement: &ExactExpressionRequirement,
    expected_source: NecoImplementationSource,
    actual_owner: OwnerId,
    actual_requirement: &ExactExpressionRequirement,
    actual_source: NecoImplementationSource,
) -> Result<(), NecoFailure> {
    let field = if expected_requirement.consumer_id() != actual_requirement.consumer_id() {
        ProvenanceMismatchField::Consumer
    } else if expected_owner != actual_owner {
        ProvenanceMismatchField::Owner
    } else if expected_requirement.expressions() != actual_requirement.expressions()
        || expected_requirement.decision() != actual_requirement.decision()
    {
        ProvenanceMismatchField::Expression
    } else if expected_requirement.precision() != actual_requirement.precision() {
        ProvenanceMismatchField::Precision
    } else if expected_source != actual_source {
        ProvenanceMismatchField::Source
    } else {
        return Ok(());
    };
    let actual_expression = actual_requirement
        .expressions()
        .first()
        .copied()
        .unwrap_or_else(|| ExprId::new(0));
    let location = FailureLocation::operation(FailureOperation::Assemble)
        .with_consumer(actual_requirement.consumer_id())
        .with_expr(actual_expression);
    Err(NecoFailure::ProvenanceMismatch {
        location,
        payload: Box::new(ProvenanceMismatchPayload::new(
            field,
            lineage_record(expected_owner, expected_requirement, expected_source),
            lineage_record(actual_owner, actual_requirement, actual_source),
        )),
    })
}

fn lineage_record(
    owner: OwnerId,
    requirement: &ExactExpressionRequirement,
    source: NecoImplementationSource,
) -> ProvenanceRecord {
    ProvenanceRecord::new(
        Some(owner),
        Some(requirement.consumer_id()),
        requirement.expressions().first().copied(),
        Some(requirement.precision()),
        Some(source.revision_identity()),
        Some(source.public_api_identity()),
    )
}

fn evaluation_expression_failure(
    operation: EvaluationOperation,
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
    candidate: TranscendentalEvaluationCandidate,
    error: NecoFailure,
) -> CalculusFailure {
    EvaluationFailure::Lower {
        operation,
        owner,
        requirement,
        source,
        input: EvaluationInputSnapshot::Transcendental { candidate },
        witness: LowerWitness::new(operation),
        lower: LowerEvaluationFailure::Expression(error),
    }
    .into()
}

fn clone_enclosure_value(enclosure: &Enclosure) -> Result<Enclosure, BigintError> {
    Ok(Enclosure::from_dyadic_enclosure(
        enclosure.as_dyadic_enclosure().try_clone()?,
    ))
}

impl From<neco_bigint::BigintError> for CalculusFailure {
    fn from(error: neco_bigint::BigintError) -> Self {
        Self::Arithmetic(error)
    }
}

impl From<AlgnumError> for CalculusFailure {
    fn from(error: AlgnumError) -> Self {
        Self::Algebra(error)
    }
}

impl From<PolynomialFailure> for CalculusFailure {
    fn from(error: PolynomialFailure) -> Self {
        Self::Polynomial(error)
    }
}

impl From<RationalFailure> for CalculusFailure {
    fn from(error: RationalFailure) -> Self {
        Self::Rational(error)
    }
}

impl From<JetFailure> for CalculusFailure {
    fn from(error: JetFailure) -> Self {
        Self::Jet(error)
    }
}

impl From<IntervalFailure> for CalculusFailure {
    fn from(error: IntervalFailure) -> Self {
        Self::Interval(error)
    }
}

impl From<RootFailure> for CalculusFailure {
    fn from(error: RootFailure) -> Self {
        Self::Root(error)
    }
}

impl From<EvaluationFailure> for CalculusFailure {
    fn from(error: EvaluationFailure) -> Self {
        Self::Evaluation(Box::new(error))
    }
}

impl From<TranscendentalConnectorFailure> for CalculusFailure {
    fn from(error: TranscendentalConnectorFailure) -> Self {
        Self::Transcendental(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PI_400: &str = "3243f6a8885a308d313198a2e03707344a4093822299f31d0082efa98ec4e6c89452821e638d01377be5466cf34e90c6cc0ac";
    const LN_TWO_400: &str = "b17217f7d1cf79abc9e3b39803f2f6af40f343267298b62d8a0d175b8baafa2be7b876206debac98559552fb4afa1b10ed2e";
    const SINE_QUARTER_300: &str =
        "3f55dda9e62aed7513bd7b8e6a3d1635dd5676648d7db525898d7086af9330f03c7f285442a";
    const COSINE_QUARTER_300: &str =
        "f80aa4fbef750ba783d33cb95f94f8a41426dbe79edc4a023ef9ec13c944551c0795b84fee1";
    const EXPONENTIAL_QUARTER_300: &str =
        "148b5e3c3e81866767bc3b69baabe534ec43887164bbe2b0a993e8cf15a620bebaf42e6764e4";
    const DOUBLED_ARTANH_QUARTER_300: &str =
        "82c577d408a28d393b5e17c1021db53e8b382de73efea01c477373c22acc8611c66c126c4be";

    fn hex(digits: &str, exponent: u32) -> Dyadic {
        let sixteen = BigInt::try_from(16_u8).unwrap();
        let mut value = BigInt::zero();
        for digit in digits.chars() {
            let digit = BigInt::try_from(digit.to_digit(16).unwrap()).unwrap();
            value = value.mul(&sixteen).unwrap().add(&digit).unwrap();
        }
        Dyadic::new(value, exponent)
    }

    fn unit(exponent: u32) -> Dyadic {
        Dyadic::new(BigInt::one().unwrap(), exponent)
    }

    fn assert_contains_reference(enclosure: &DyadicEnclosure, digits: &str, exponent: u32) {
        let floor = hex(digits, exponent);
        let ceiling = floor.add(&unit(exponent)).unwrap();
        assert!(enclosure.contains_dyadic(&floor));
        assert!(enclosure.contains_dyadic(&ceiling));
    }

    #[test]
    fn pi_and_ln2_enclosures_contain_independent_references() {
        for bits in [64, 128, 256] {
            assert_contains_reference(&pi_enclosure(bits).unwrap(), PI_400, 400);
            assert_contains_reference(&ln2_enclosure(bits).unwrap(), LN_TWO_400, 400);
        }
    }

    #[test]
    fn bounded_series_over_a_point_keeps_the_truncation_slack() {
        let quarter = point_enclosure(&Dyadic::new(BigInt::one().unwrap(), 2)).unwrap();
        let cases = [
            (SeriesKind::Sine, SINE_QUARTER_300),
            (SeriesKind::Cosine, COSINE_QUARTER_300),
            (SeriesKind::Exponential, EXPONENTIAL_QUARTER_300),
            (SeriesKind::DoubledArtanh, DOUBLED_ARTANH_QUARTER_300),
        ];
        for (kind, reference) in cases {
            for bits in [64, 200] {
                let result = evaluate_bounded_series(kind, &quarter, bits).unwrap();
                assert!(result.width().unwrap() >= unit(bits + 1));
                assert_contains_reference(&result, reference, 300);
            }
        }
        let zero = point_enclosure(&Dyadic::new(BigInt::zero(), 0)).unwrap();
        for (kind, exact) in [(SeriesKind::Exponential, 1_u8), (SeriesKind::Sine, 0_u8)] {
            for bits in [64, 200] {
                let result = evaluate_bounded_series(kind, &zero, bits).unwrap();
                assert!(result.width().unwrap() >= unit(bits + 1));
                let exact = Dyadic::new(BigInt::try_from(exact).unwrap(), 0);
                assert!(result.contains_dyadic(&exact));
            }
        }
    }

    #[test]
    fn bounded_series_rejects_out_of_range_domains_and_excess_precision() {
        let two = point_enclosure(&Dyadic::new(BigInt::try_from(2_u8).unwrap(), 0)).unwrap();
        assert!(matches!(
            evaluate_bounded_series(SeriesKind::Sine, &two, 64),
            Err(SeriesFailure::ReductionRange)
        ));
        let half = point_enclosure(&Dyadic::new(BigInt::one().unwrap(), 1)).unwrap();
        assert!(matches!(
            evaluate_bounded_series(SeriesKind::Exponential, &half, 2000),
            Err(SeriesFailure::PrecisionLimit {
                requested: 2000,
                maximum: 1677
            })
        ));
    }
}
