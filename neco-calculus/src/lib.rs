#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

extern crate alloc;

use alloc::vec::Vec;

use neco_algnum::{AlgnumError, RationalPolynomial, RealAlgebraic};
use neco_bigint::{BigInt, BigintError, Dyadic};
use neco_expr::ExprId;
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
pub enum EvaluationFailure {
    ZeroSurfaceTerm,
    DuplicateSurfaceExponent,
    ZeroImplicitTerm,
    DuplicateImplicitExponent,
    SurfaceTermLimitExceeded { requested: usize, maximum: usize },
    ImplicitTermLimitExceeded { requested: usize, maximum: usize },
}

pub const MAX_SURFACE_TERMS: usize = 4_096;
pub const MAX_IMPLICIT_TERMS: usize = 4_096;

#[derive(Debug, Eq, PartialEq)]
pub enum CalculusFailure {
    Polynomial(PolynomialFailure),
    Rational(RationalFailure),
    Jet(JetFailure),
    Interval(IntervalFailure),
    Root(RootFailure),
    Transcendental(TranscendentalConnectorFailure),
    Evaluation(EvaluationFailure),
    Algebra(AlgnumError),
    Arithmetic(BigintError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MethodDescriptor {
    descriptor: u64,
}

impl MethodDescriptor {
    pub const fn new(descriptor: u64) -> Self {
        Self { descriptor }
    }

    pub const fn get(self) -> u64 {
        self.descriptor
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranscendentalEvaluationRequest {
    expression: ExprId,
    requested_precision_bits: u32,
}

impl TranscendentalEvaluationRequest {
    pub const fn new(expression: ExprId, requested_precision_bits: u32) -> Self {
        Self {
            expression,
            requested_precision_bits,
        }
    }

    pub const fn expression(&self) -> ExprId {
        self.expression
    }

    pub const fn requested_precision_bits(&self) -> u32 {
        self.requested_precision_bits
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvaluationProvenance {
    method_descriptor: MethodDescriptor,
    source_expression: ExprId,
}

impl EvaluationProvenance {
    const fn new(method_descriptor: MethodDescriptor, source_expression: ExprId) -> Self {
        Self {
            method_descriptor,
            source_expression,
        }
    }

    pub const fn method_descriptor(self) -> MethodDescriptor {
        self.method_descriptor
    }

    pub const fn source_expression(self) -> ExprId {
        self.source_expression
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum TranscendentalEvaluationCandidate {
    ExactAlgebraic {
        value: RealAlgebraic,
        source_expression: ExprId,
    },
    Enclosure {
        enclosure: Enclosure,
        method_descriptor: MethodDescriptor,
        source_expression: ExprId,
    },
    Undetermined {
        source_expression: ExprId,
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
    ExactAlgebraic(RealAlgebraic),
    Enclosure {
        enclosure: Enclosure,
        provenance: EvaluationProvenance,
    },
    Undetermined,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TranscendentalEvaluation {
    value: TranscendentalEvaluationValue,
}

impl TranscendentalEvaluation {
    pub const fn kind(&self) -> TranscendentalEvaluationKind {
        match self.value {
            TranscendentalEvaluationValue::ExactAlgebraic(_) => {
                TranscendentalEvaluationKind::ExactAlgebraic
            }
            TranscendentalEvaluationValue::Enclosure { .. } => {
                TranscendentalEvaluationKind::Enclosure
            }
            TranscendentalEvaluationValue::Undetermined => {
                TranscendentalEvaluationKind::Undetermined
            }
        }
    }

    pub fn as_exact_algebraic(&self) -> Option<&RealAlgebraic> {
        match &self.value {
            TranscendentalEvaluationValue::ExactAlgebraic(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_enclosure(&self) -> Option<(&Enclosure, EvaluationProvenance)> {
        match &self.value {
            TranscendentalEvaluationValue::Enclosure {
                enclosure,
                provenance,
            } => Some((enclosure, *provenance)),
            _ => None,
        }
    }

    pub const fn is_undetermined(&self) -> bool {
        matches!(self.value, TranscendentalEvaluationValue::Undetermined)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranscendentalConnectorFailure {
    CandidateExpressionMismatch {
        candidate_expression: ExprId,
        requested: ExprId,
    },
    PrecisionLimitExceeded {
        requested: u32,
        maximum: u32,
    },
    PrecisionMustBePositive,
    EnclosureTooWide {
        requested_precision_bits: u32,
    },
}

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
        terms.sort_unstable_by_key(|term| (term.exponent_u, term.exponent_v));
        if terms.iter().any(|term| term.coefficient.is_zero()) {
            return Err(EvaluationFailure::ZeroSurfaceTerm.into());
        }
        if terms.windows(2).any(|pair| {
            (pair[0].exponent_u, pair[0].exponent_v) == (pair[1].exponent_u, pair[1].exponent_v)
        }) {
            return Err(EvaluationFailure::DuplicateSurfaceExponent.into());
        }
        Ok(Self { terms })
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
pub enum SurfaceEvaluationProducer {
    Exact {
        x: RealAlgebraic,
        y: RealAlgebraic,
        z: RealAlgebraic,
    },
}

pub fn evaluate_surface(
    input: SurfaceEvaluationInput,
) -> Result<SurfaceEvaluationProducer, CalculusFailure> {
    let denominator = evaluate_surface_polynomial(&input.denominator, &input.u, &input.v)?;
    if denominator.is_zero() {
        return Err(CalculusFailure::Rational(RationalFailure::UndefinedDomain));
    }
    let x = evaluate_surface_polynomial(&input.x, &input.u, &input.v)?;
    let y = evaluate_surface_polynomial(&input.y, &input.u, &input.v)?;
    let z = evaluate_surface_polynomial(&input.z, &input.u, &input.v)?;
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
        let term_value = u
            .pow_integer(&neco_bigint::BigInt::try_from(term.exponent_u)?)?
            .mul(&v.pow_integer(&neco_bigint::BigInt::try_from(term.exponent_v)?)?)?
            .mul(&coefficient)?;
        result = result.add(&term_value)?;
    }
    Ok(result)
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
    if request.requested_precision_bits() == 0 {
        return Err(TranscendentalConnectorFailure::PrecisionMustBePositive.into());
    }
    if request.requested_precision_bits() > MAX_PRECISION_BITS {
        return Err(TranscendentalConnectorFailure::PrecisionLimitExceeded {
            requested: request.requested_precision_bits(),
            maximum: MAX_PRECISION_BITS,
        }
        .into());
    }
    match candidate {
        TranscendentalEvaluationCandidate::ExactAlgebraic {
            value,
            source_expression,
        } => {
            if source_expression == request.expression() {
                Ok(TranscendentalEvaluation {
                    value: TranscendentalEvaluationValue::ExactAlgebraic(value),
                })
            } else {
                Err(CalculusFailure::from(
                    TranscendentalConnectorFailure::CandidateExpressionMismatch {
                        candidate_expression: source_expression,
                        requested: request.expression(),
                    },
                ))
            }
        }
        TranscendentalEvaluationCandidate::Enclosure {
            enclosure,
            method_descriptor,
            source_expression,
        } => {
            if source_expression == request.expression() {
                let maximum_width =
                    Dyadic::new(BigInt::try_from(1_u8)?, request.requested_precision_bits());
                if enclosure.width()? > maximum_width {
                    return Err(TranscendentalConnectorFailure::EnclosureTooWide {
                        requested_precision_bits: request.requested_precision_bits(),
                    }
                    .into());
                }
                Ok(TranscendentalEvaluation {
                    value: TranscendentalEvaluationValue::Enclosure {
                        enclosure,
                        provenance: EvaluationProvenance::new(method_descriptor, source_expression),
                    },
                })
            } else {
                Err(CalculusFailure::from(
                    TranscendentalConnectorFailure::CandidateExpressionMismatch {
                        candidate_expression: source_expression,
                        requested: request.expression(),
                    },
                ))
            }
        }
        TranscendentalEvaluationCandidate::Undetermined { source_expression } => {
            if source_expression == request.expression() {
                Ok(TranscendentalEvaluation {
                    value: TranscendentalEvaluationValue::Undetermined,
                })
            } else {
                Err(CalculusFailure::from(
                    TranscendentalConnectorFailure::CandidateExpressionMismatch {
                        candidate_expression: source_expression,
                        requested: request.expression(),
                    },
                ))
            }
        }
    }
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
        Self::Evaluation(error)
    }
}

impl From<TranscendentalConnectorFailure> for CalculusFailure {
    fn from(error: TranscendentalConnectorFailure) -> Self {
        Self::Transcendental(error)
    }
}
