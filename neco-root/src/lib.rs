#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

extern crate alloc;

use core::cmp::Ordering;

use neco_algnum::{AlgnumError, RealAlgebraic};
use neco_bigint::{
    BigInt, BigUint, BigintError, Dyadic, DyadicEnclosure, RawRational, ReducedRational, Sign,
};
use neco_formsum::{FormSumErrorKind, RawFormSum, RawTerm};
use neco_monomial::RawMonomial;
use neco_polynomial::{DerivativeOrder, Enclosure, IntervalFailure, Polynomial, PolynomialFailure};

pub const MAX_CANDIDATE_POINTS: usize = 4_096;
pub const MAX_DERIVATIVE_ORDER: usize = 64;
pub const MAX_ROOT_REFINEMENT_ITERATIONS: usize = 19;
pub const MAX_ROOT_REFINEMENT_PRECISION_BITS: u32 = 256;

#[derive(Debug, Eq, PartialEq)]
pub struct RootBracket {
    polynomial: Polynomial,
    enclosure: Enclosure,
    evidence: RootEvidence,
}

impl RootBracket {
    pub fn sign_change(polynomial: Polynomial, enclosure: Enclosure) -> Result<Self, RootFailure> {
        let lower = sign_at(&polynomial, enclosure.lower())?;
        let upper = sign_at(&polynomial, enclosure.upper())?;
        if !opposite_nonzero(lower, upper) {
            return Err(RootFailure::NoSignChange);
        }
        Ok(Self {
            polynomial,
            enclosure,
            evidence: RootEvidence::SignChange { lower, upper },
        })
    }

    pub fn even_multiplicity_touch(
        polynomial: Polynomial,
        enclosure: Enclosure,
        point: Dyadic,
        max_order: DerivativeOrder,
    ) -> Result<Self, RootFailure> {
        if !enclosure.contains(&point) {
            return Err(RootFailure::PointOutsideEnclosure);
        }
        let multiplicity = classify_multiplicity(&polynomial, &point, max_order)?;
        match multiplicity {
            RootMultiplicity::Multiple(order) if order.get() >= 2 && order.get() % 2 == 0 => {
                Ok(Self {
                    polynomial,
                    enclosure,
                    evidence: RootEvidence::EvenMultiplicityTouch {
                        point,
                        multiplicity,
                    },
                })
            }
            _ => Err(RootFailure::NoEvenMultiplicityTouch),
        }
    }

    pub fn polynomial(&self) -> &Polynomial {
        &self.polynomial
    }

    pub fn enclosure(&self) -> &Enclosure {
        &self.enclosure
    }

    pub fn evidence(&self) -> &RootEvidence {
        &self.evidence
    }

    pub fn refine_bisection(&self, iterations: usize) -> Result<RootIsolation, RootFailure> {
        validate_resource_limit(
            RootResource::RefinementIterations,
            iterations,
            MAX_ROOT_REFINEMENT_ITERATIONS,
        )?;
        match &self.evidence {
            RootEvidence::SignChange { lower, .. } => {
                refine_sign_change(self, iterations, *lower, RootRefinementMethod::Bisection, 0)
            }
            RootEvidence::EvenMultiplicityTouch { point, .. } => {
                exact_root_isolation(self, point, RootRefinementMethod::Bisection)
            }
        }
    }

    pub fn refine_newton_bisection(
        &self,
        iterations: usize,
        precision_bits: u32,
    ) -> Result<RootIsolation, RootFailure> {
        validate_resource_limit(
            RootResource::RefinementIterations,
            iterations,
            MAX_ROOT_REFINEMENT_ITERATIONS,
        )?;
        validate_resource_limit(
            RootResource::RefinementPrecisionBits,
            precision_bits as usize,
            MAX_ROOT_REFINEMENT_PRECISION_BITS as usize,
        )?;
        match &self.evidence {
            RootEvidence::SignChange { lower, .. } => refine_sign_change(
                self,
                iterations,
                *lower,
                RootRefinementMethod::NewtonBisection,
                precision_bits,
            ),
            RootEvidence::EvenMultiplicityTouch { point, .. } => {
                exact_root_isolation(self, point, RootRefinementMethod::NewtonBisection)
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum RootEvidence {
    SignChange {
        lower: Sign,
        upper: Sign,
    },
    EvenMultiplicityTouch {
        point: Dyadic,
        multiplicity: RootMultiplicity,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootMultiplicity {
    Simple,
    Multiple(DerivativeOrder),
}

#[derive(Debug, Eq, PartialEq)]
pub struct MembershipJudgmentRequest {
    target_polynomial: Polynomial,
    target_enclosure: Enclosure,
    candidate_points: alloc::vec::Vec<Dyadic>,
    max_order: DerivativeOrder,
    zero_exclusion_precision_bits: u32,
}

impl MembershipJudgmentRequest {
    pub fn new(
        target_polynomial: Polynomial,
        target_enclosure: Enclosure,
        candidate_points: alloc::vec::Vec<Dyadic>,
        max_order: DerivativeOrder,
        zero_exclusion_precision_bits: u32,
    ) -> Result<Self, RootFailure> {
        validate_resource_limit(
            RootResource::CandidatePoints,
            candidate_points.len(),
            MAX_CANDIDATE_POINTS,
        )?;
        validate_resource_limit(
            RootResource::DerivativeOrder,
            max_order.get(),
            MAX_DERIVATIVE_ORDER,
        )?;
        validate_resource_limit(
            RootResource::RegionSignPrecisionBits,
            zero_exclusion_precision_bits as usize,
            MAX_ROOT_REFINEMENT_PRECISION_BITS as usize,
        )?;
        Ok(Self {
            target_polynomial,
            target_enclosure,
            candidate_points,
            max_order,
            zero_exclusion_precision_bits,
        })
    }

    pub fn target_polynomial(&self) -> &Polynomial {
        &self.target_polynomial
    }

    pub fn target_enclosure(&self) -> &Enclosure {
        &self.target_enclosure
    }

    pub fn candidate_points(&self) -> &[Dyadic] {
        &self.candidate_points
    }

    pub fn max_order(&self) -> DerivativeOrder {
        self.max_order
    }

    pub fn zero_exclusion_precision_bits(&self) -> u32 {
        self.zero_exclusion_precision_bits
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionSign {
    ConstantPositive,
    ConstantNegative,
    ConstantZero,
    SignVaries,
    ContainsZero,
    Undetermined,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionSignJudgment {
    sign: RegionSign,
}

impl RegionSignJudgment {
    fn new(sign: RegionSign) -> Self {
        Self { sign }
    }

    pub fn sign(&self) -> RegionSign {
        self.sign
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MembershipTransitionReport {
    loci: alloc::vec::Vec<MembershipLocus>,
}
impl MembershipTransitionReport {
    fn new(loci: alloc::vec::Vec<MembershipLocus>) -> Self {
        Self { loci }
    }

    pub fn loci(&self) -> &[MembershipLocus] {
        &self.loci
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MembershipLocus {
    point_index: usize,
    kind: MembershipLocusKind,
    multiplicity: RootMultiplicity,
}

impl MembershipLocus {
    fn new(point_index: usize, kind: MembershipLocusKind, multiplicity: RootMultiplicity) -> Self {
        Self {
            point_index,
            kind,
            multiplicity,
        }
    }

    pub fn point_index(&self) -> usize {
        self.point_index
    }

    pub fn kind(&self) -> MembershipLocusKind {
        self.kind
    }

    pub fn multiplicity(&self) -> RootMultiplicity {
        self.multiplicity
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MembershipLocusKind {
    Transition,
    SingularContact,
}

fn classify_local_multiplicity(
    request: &MembershipJudgmentRequest,
) -> Result<alloc::vec::Vec<(usize, RootMultiplicity)>, RootFailure> {
    for point in request.candidate_points() {
        if !request.target_enclosure().contains(point) {
            return Err(RootFailure::PointOutsideEnclosure);
        }
    }

    let mut roots = alloc::vec::Vec::new();
    for (index, point) in request.candidate_points().iter().enumerate() {
        let algebraic_point = dyadic_to_algebraic(point)?;
        if request
            .target_polynomial()
            .evaluate(&algebraic_point)?
            .is_zero()
        {
            let multiplicity =
                classify_multiplicity(request.target_polynomial(), point, request.max_order())?;
            roots.push((index, multiplicity));
        }
    }
    Ok(roots)
}

pub fn classify_region_sign(
    request: &MembershipJudgmentRequest,
) -> Result<RegionSignJudgment, RootFailure> {
    if request.target_polynomial().degree() == 0 {
        let coefficient = request
            .target_polynomial()
            .coefficients()
            .first()
            .ok_or(PolynomialFailure::EmptyCoefficients)?;
        return match coefficient.sign()? {
            Sign::Positive => Ok(RegionSignJudgment::new(RegionSign::ConstantPositive)),
            Sign::Negative => Ok(RegionSignJudgment::new(RegionSign::ConstantNegative)),
            Sign::Zero => Ok(RegionSignJudgment::new(RegionSign::ConstantZero)),
        };
    }

    let roots = classify_local_multiplicity(request)?;
    if !roots.is_empty() {
        if roots.iter().any(|(point_index, multiplicity)| {
            let point = &request.candidate_points()[*point_index];
            let is_interior = request.target_enclosure().lower() < point
                && point < request.target_enclosure().upper();
            let has_odd_multiplicity = match multiplicity {
                RootMultiplicity::Simple => true,
                RootMultiplicity::Multiple(order) => order.get() % 2 == 1,
            };
            is_interior && has_odd_multiplicity
        }) {
            return Ok(RegionSignJudgment::new(RegionSign::SignVaries));
        }
        return Ok(RegionSignJudgment::new(RegionSign::ContainsZero));
    }

    let range = request.target_polynomial().evaluate_enclosure(
        request.target_enclosure(),
        request.zero_exclusion_precision_bits(),
    )?;
    if range.lower().integer().sign() == Sign::Positive {
        return Ok(RegionSignJudgment::new(RegionSign::ConstantPositive));
    }
    if range.upper().integer().sign() == Sign::Negative {
        return Ok(RegionSignJudgment::new(RegionSign::ConstantNegative));
    }

    let lower_sign = sign_at(
        request.target_polynomial(),
        request.target_enclosure().lower(),
    )?;
    let upper_sign = sign_at(
        request.target_polynomial(),
        request.target_enclosure().upper(),
    )?;

    match (lower_sign, upper_sign) {
        (Sign::Positive, Sign::Negative) | (Sign::Negative, Sign::Positive) => {
            Ok(RegionSignJudgment::new(RegionSign::SignVaries))
        }
        _ => Ok(RegionSignJudgment::new(RegionSign::Undetermined)),
    }
}

pub fn locate_membership_transitions(
    request: &MembershipJudgmentRequest,
) -> Result<MembershipTransitionReport, RootFailure> {
    let roots = classify_local_multiplicity(request)?;
    let mut loci = alloc::vec::Vec::new();

    for (point_index, multiplicity) in roots {
        let kind = match multiplicity {
            RootMultiplicity::Simple => MembershipLocusKind::Transition,
            RootMultiplicity::Multiple(order) => {
                if order.get() % 2 == 0 {
                    MembershipLocusKind::SingularContact
                } else {
                    MembershipLocusKind::Transition
                }
            }
        };
        loci.push(MembershipLocus::new(point_index, kind, multiplicity));
    }

    Ok(MembershipTransitionReport::new(loci))
}

#[derive(Debug, Eq, PartialEq)]
pub struct RootIsolation {
    enclosure: Enclosure,
    witness: ConvergenceWitness,
}

impl RootIsolation {
    fn new(enclosure: Enclosure, witness: ConvergenceWitness) -> Self {
        Self { enclosure, witness }
    }

    pub fn enclosure(&self) -> &Enclosure {
        &self.enclosure
    }

    pub fn witness(&self) -> &ConvergenceWitness {
        &self.witness
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ConvergenceWitness {
    method: RootRefinementMethod,
    status: ConvergenceStatus,
    iterations: usize,
    newton_steps: usize,
    newton_fallbacks: usize,
    initial: Enclosure,
    final_enclosure: Enclosure,
}

impl ConvergenceWitness {
    fn new(
        method: RootRefinementMethod,
        status: ConvergenceStatus,
        iterations: usize,
        newton_steps: usize,
        newton_fallbacks: usize,
        initial: Enclosure,
        final_enclosure: Enclosure,
    ) -> Self {
        Self {
            method,
            status,
            iterations,
            newton_steps,
            newton_fallbacks,
            initial,
            final_enclosure,
        }
    }

    pub fn method(&self) -> RootRefinementMethod {
        self.method
    }

    pub fn status(&self) -> ConvergenceStatus {
        self.status
    }

    pub fn iterations(&self) -> usize {
        self.iterations
    }

    pub fn newton_steps(&self) -> usize {
        self.newton_steps
    }

    pub fn newton_fallback_count(&self) -> usize {
        self.newton_fallbacks
    }

    pub fn newton_fallback_reason(&self) -> Option<NewtonFallbackReason> {
        (self.newton_fallbacks > 0).then_some(NewtonFallbackReason::DerivativeEnclosureContainsZero)
    }

    pub fn initial(&self) -> &Enclosure {
        &self.initial
    }

    pub fn final_enclosure(&self) -> &Enclosure {
        &self.final_enclosure
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootRefinementMethod {
    Bisection,
    NewtonBisection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NewtonFallbackReason {
    DerivativeEnclosureContainsZero,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConvergenceStatus {
    ExactDyadicRoot,
    Refined,
    IterationLimit,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RootFailure {
    Polynomial(PolynomialFailure),
    Interval(IntervalFailure),
    Algebra(AlgnumError),
    Arithmetic(BigintError),
    FormSum(FormSumErrorKind),
    NoSignChange,
    NoRootAtPoint,
    NoEvenMultiplicityTouch,
    PointOutsideEnclosure,
    MultiplicityUndetermined,
    NewtonDerivativeContainsZero,
    ResourceLimit {
        resource: RootResource,
        requested: usize,
        maximum: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootResource {
    CandidatePoints,
    DerivativeOrder,
    RefinementIterations,
    RefinementPrecisionBits,
    RegionSignPrecisionBits,
}

pub fn classify_multiplicity(
    polynomial: &Polynomial,
    point: &Dyadic,
    max_order: DerivativeOrder,
) -> Result<RootMultiplicity, RootFailure> {
    validate_resource_limit(
        RootResource::DerivativeOrder,
        max_order.get(),
        MAX_DERIVATIVE_ORDER,
    )?;
    let algebraic_point = dyadic_to_algebraic(point)?;
    if polynomial.evaluate(&algebraic_point)?.sign()? != Sign::Zero {
        return Err(RootFailure::NoRootAtPoint);
    }
    for order in 1..=max_order.get() {
        let derivative = polynomial.derivative(DerivativeOrder::new(order))?;
        if derivative.evaluate(&algebraic_point)?.sign()? != Sign::Zero {
            return if order == 1 {
                Ok(RootMultiplicity::Simple)
            } else {
                Ok(RootMultiplicity::Multiple(DerivativeOrder::new(order)))
            };
        }
    }
    Err(RootFailure::MultiplicityUndetermined)
}

impl From<PolynomialFailure> for RootFailure {
    fn from(error: PolynomialFailure) -> Self {
        Self::Polynomial(error)
    }
}

impl From<IntervalFailure> for RootFailure {
    fn from(error: IntervalFailure) -> Self {
        Self::Interval(error)
    }
}

impl From<AlgnumError> for RootFailure {
    fn from(error: AlgnumError) -> Self {
        Self::Algebra(error)
    }
}

impl From<BigintError> for RootFailure {
    fn from(error: BigintError) -> Self {
        Self::Arithmetic(error)
    }
}

impl From<FormSumErrorKind> for RootFailure {
    fn from(error: FormSumErrorKind) -> Self {
        Self::FormSum(error)
    }
}

fn refine_sign_change(
    bracket: &RootBracket,
    iterations: usize,
    initial_lower_sign: Sign,
    method: RootRefinementMethod,
    precision_bits: u32,
) -> Result<RootIsolation, RootFailure> {
    let initial = clone_enclosure(bracket.enclosure())?;
    let mut lower = bracket.enclosure().lower().try_clone()?;
    let mut upper = bracket.enclosure().upper().try_clone()?;
    let mut lower_sign = initial_lower_sign;
    let mut status = ConvergenceStatus::IterationLimit;
    let mut used_iterations = 0;
    let mut newton_steps = 0;
    let mut newton_fallbacks = 0;

    for _ in 0..iterations {
        used_iterations += 1;
        let candidate = if method == RootRefinementMethod::NewtonBisection {
            match newton_candidate(&bracket.polynomial, &lower, &upper, precision_bits) {
                Ok(candidate) if lower < candidate && candidate < upper => {
                    newton_steps += 1;
                    candidate
                }
                Ok(_) => lower.midpoint(&upper)?,
                Err(RootFailure::NewtonDerivativeContainsZero) => {
                    newton_fallbacks += 1;
                    lower.midpoint(&upper)?
                }
                Err(error) => return Err(error),
            }
        } else {
            lower.midpoint(&upper)?
        };
        let sign = sign_at(&bracket.polynomial, &candidate)?;
        if sign == Sign::Zero {
            lower = candidate.try_clone()?;
            upper = candidate;
            status = ConvergenceStatus::ExactDyadicRoot;
            break;
        }
        if sign == lower_sign {
            lower = candidate;
            lower_sign = sign;
        } else {
            upper = candidate;
        }
        status = ConvergenceStatus::Refined;
    }

    let final_enclosure = Enclosure::new(lower, upper)?;
    Ok(RootIsolation::new(
        clone_enclosure(&final_enclosure)?,
        ConvergenceWitness::new(
            method,
            status,
            used_iterations,
            newton_steps,
            newton_fallbacks,
            initial,
            final_enclosure,
        ),
    ))
}

fn exact_root_isolation(
    bracket: &RootBracket,
    point: &Dyadic,
    method: RootRefinementMethod,
) -> Result<RootIsolation, RootFailure> {
    let exact = Enclosure::new(point.try_clone()?, point.try_clone()?)?;
    Ok(RootIsolation::new(
        clone_enclosure(&exact)?,
        ConvergenceWitness::new(
            method,
            ConvergenceStatus::ExactDyadicRoot,
            0,
            0,
            0,
            clone_enclosure(&bracket.enclosure)?,
            exact,
        ),
    ))
}

fn newton_candidate(
    polynomial: &Polynomial,
    lower: &Dyadic,
    upper: &Dyadic,
    precision_bits: u32,
) -> Result<Dyadic, RootFailure> {
    let midpoint = lower.midpoint(upper)?;
    let midpoint_value = dyadic_to_algebraic(&midpoint)?;
    let derivative = polynomial.derivative(DerivativeOrder::new(1))?;
    let value_enclosure = polynomial
        .evaluate(&midpoint_value)?
        .enclose(precision_bits)?
        .enclosure()
        .try_clone()?;
    let derivative_enclosure = derivative
        .evaluate(&midpoint_value)?
        .enclose(precision_bits)?
        .enclosure()
        .try_clone()?;
    if contains_zero(&derivative_enclosure) {
        return Err(RootFailure::NewtonDerivativeContainsZero);
    }
    let quotient = enclosure_div(&value_enclosure, &derivative_enclosure, precision_bits)?;
    midpoint.sub(&quotient.midpoint()?).map_err(Into::into)
}

fn sign_at(polynomial: &Polynomial, point: &Dyadic) -> Result<Sign, RootFailure> {
    Ok(polynomial.evaluate(&dyadic_to_algebraic(point)?)?.sign()?)
}

fn dyadic_to_algebraic(value: &Dyadic) -> Result<RealAlgebraic, RootFailure> {
    if value.is_zero() {
        return Ok(RealAlgebraic::from_form_sum(&neco_formsum::FormSum::zero())?);
    }
    let denominator = BigUint::one()?.shl_bits(value.exponent() as usize)?;
    let raw = RawFormSum::new(alloc::vec![RawTerm::new(
        RawRational::new(value.integer().try_clone()?, denominator),
        RawMonomial::positive(alloc::vec![]),
    )]);
    let sum = raw
        .normalize()
        .map_err(|errors| RootFailure::FormSum(errors.into_parts().0))?;
    Ok(RealAlgebraic::from_form_sum(&sum)?)
}

fn clone_enclosure(enclosure: &Enclosure) -> Result<Enclosure, RootFailure> {
    Ok(Enclosure::from_dyadic_enclosure(
        enclosure.as_dyadic_enclosure().try_clone()?,
    ))
}

fn contains_zero(enclosure: &DyadicEnclosure) -> bool {
    enclosure.contains_dyadic(&Dyadic::new(BigInt::zero(), 0))
}

fn enclosure_div(
    numerator: &DyadicEnclosure,
    denominator: &DyadicEnclosure,
    precision_bits: u32,
) -> Result<DyadicEnclosure, RootFailure> {
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
    Ok(DyadicEnclosure::new(
        lower.dyadic_floor(precision_bits)?,
        upper.dyadic_ceil(precision_bits)?,
    )?)
}

fn dyadic_to_rational(value: &Dyadic) -> Result<ReducedRational, BigintError> {
    let denominator = BigUint::one()?.shl_bits(value.exponent() as usize)?;
    RawRational::new(value.integer().try_clone()?, denominator)
        .reduce()
        .map(|reduction| reduction.into_reduced())
}

fn opposite_nonzero(left: Sign, right: Sign) -> bool {
    matches!(
        (left, right),
        (Sign::Negative, Sign::Positive) | (Sign::Positive, Sign::Negative)
    )
}

fn validate_resource_limit(
    resource: RootResource,
    requested: usize,
    maximum: usize,
) -> Result<(), RootFailure> {
    if requested > maximum {
        return Err(RootFailure::ResourceLimit {
            resource,
            requested,
            maximum,
        });
    }
    Ok(())
}
