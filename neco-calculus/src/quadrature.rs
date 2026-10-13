use alloc::boxed::Box;
use alloc::collections::BinaryHeap;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::num::NonZeroUsize;

use neco_bigint::{BigInt, BigUint, BigintError, Dyadic, DyadicEnclosure, RawRational};
use neco_expr::project_enclosure_f64;

use crate::{
    enclosure_add, enclosure_mul, evaluation_lower_failure, point_enclosure,
    precision_limit_failure, scale_by_power_of_two, AbsoluteBits, CalculusFailure, CertifiedF64,
    DerivativeOrder, DomainWitness, Enclosure, EnclosureProjectionError, EvaluationFailure,
    EvaluationInputSnapshot, EvaluationOperation, ExactExpressionRequirement, IntervalEvaluable,
    NecoImplementationSource, OwnerId, Polynomial, PolynomialFailure, PolynomialResourceFailure,
    TranscendentalEvaluationRequest, MAX_DERIVATIVE_ORDER, MAX_PRECISION_BITS,
    MAX_QUADRATURE_PIECES,
};

const WORKING_GUARD_BITS: u32 = 8;

/// An integrand that also encloses its derivatives; `evaluate_interval` is the order-zero case
/// and encloses the value. The quadrature trusts every enclosure and does not verify it, so its
/// result encloses the integral only if every enclosure is true for the integrand it describes.
pub trait IntervalDifferentiable: IntervalEvaluable {
    /// Encloses the derivative of the given order, at least 1, at every point of the domain. An
    /// order the integrand cannot supply is a failure, and the quadrature never lowers the order.
    fn evaluate_derivative_interval(
        &self,
        order: DerivativeOrder,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure>;
}

impl IntervalDifferentiable for Polynomial {
    fn evaluate_derivative_interval(
        &self,
        order: DerivativeOrder,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        Ok(self
            .derivative(order)?
            .evaluate_enclosure(domain, precision_bits)?)
    }
}

/// Number of even-order Taylor terms kept before the remainder. The integrand must be continuously
/// differentiable up to twice this number and supplies those derivatives; zero uses value
/// enclosures alone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuadratureOrder {
    pairs: u32,
}

impl QuadratureOrder {
    pub fn new(pairs: u32) -> Result<Self, CalculusFailure> {
        let requested = usize::try_from(pairs)
            .unwrap_or(usize::MAX)
            .saturating_mul(2);
        if requested > MAX_DERIVATIVE_ORDER {
            return Err(PolynomialFailure::Resource(
                PolynomialResourceFailure::DerivativeOrderLimitExceeded {
                    requested,
                    maximum: MAX_DERIVATIVE_ORDER,
                },
            )
            .into());
        }
        Ok(Self { pairs })
    }

    pub const fn pairs(&self) -> u32 {
        self.pairs
    }

    /// Order of the derivative that encloses the remainder.
    pub const fn derivative_order(&self) -> DerivativeOrder {
        DerivativeOrder::new(2 * self.pairs as usize)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuadratureRefinement {
    order: QuadratureOrder,
    maximum_pieces: NonZeroUsize,
}

impl QuadratureRefinement {
    pub fn new(
        order: QuadratureOrder,
        maximum_pieces: NonZeroUsize,
    ) -> Result<Self, CalculusFailure> {
        if maximum_pieces.get() > MAX_QUADRATURE_PIECES {
            return Err(EvaluationFailure::QuadraturePieceLimitExceeded {
                requested: maximum_pieces.get(),
                maximum: MAX_QUADRATURE_PIECES,
            }
            .into());
        }
        Ok(Self {
            order,
            maximum_pieces,
        })
    }

    /// Order this refinement was built with, so a caller can read back a stored request.
    pub const fn order(&self) -> QuadratureOrder {
        self.order
    }

    /// Piece budget this refinement was built with, as recorded in the input of a failure.
    pub const fn maximum_pieces(&self) -> NonZeroUsize {
        self.maximum_pieces
    }
}

pub struct QuadratureInput<'a, F: ?Sized> {
    integrand: &'a F,
    lower: Dyadic,
    upper: Dyadic,
    refinement: QuadratureRefinement,
    request: TranscendentalEvaluationRequest,
}

impl<'a, F: ?Sized> QuadratureInput<'a, F> {
    pub const fn new(
        integrand: &'a F,
        lower: Dyadic,
        upper: Dyadic,
        refinement: QuadratureRefinement,
        request: TranscendentalEvaluationRequest,
    ) -> Self {
        Self {
            integrand,
            lower,
            upper,
            refinement,
            request,
        }
    }
}

/// Evidence for the width of a quadrature enclosure.
#[derive(Debug, Eq, PartialEq)]
pub struct QuadratureEvidence {
    pieces: usize,
    remainder_width: Dyadic,
}

impl QuadratureEvidence {
    /// Number of pieces in the final partition.
    pub const fn pieces(&self) -> usize {
        self.pieces
    }

    /// Sum over all pieces of the width of the remainder-term enclosure. The center terms account
    /// for the rest of the enclosure width.
    pub fn remainder_width(&self) -> &Dyadic {
        &self.remainder_width
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct QuadratureEvaluation {
    enclosure: Enclosure,
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
    evidence: QuadratureEvidence,
}

impl QuadratureEvaluation {
    fn new(
        enclosure: Enclosure,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
        evidence: QuadratureEvidence,
    ) -> Self {
        Self {
            enclosure,
            owner,
            requirement,
            source,
            evidence,
        }
    }

    /// Encloses the integral with a width of at most the requested absolute bits.
    pub fn enclosure(&self) -> &Enclosure {
        &self.enclosure
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

    pub fn evidence(&self) -> &QuadratureEvidence {
        &self.evidence
    }

    /// Nearest f64 with the enclosure and absolute error of the rounding.
    pub fn certify_f64(&self) -> Result<CertifiedF64, EnclosureProjectionError> {
        let enclosure = self
            .enclosure
            .as_dyadic_enclosure()
            .try_clone()
            .map_err(EnclosureProjectionError::Bigint)?;
        project_enclosure_f64(enclosure, self.requirement.precision())
    }
}

/// Encloses the integral over a closed interval with dyadic bounds by bisecting the widest piece
/// until the enclosure meets the requested absolute bits. A spent piece budget is a failure that
/// keeps the enclosure reached, never a success value.
pub fn evaluate_quadrature<F>(
    input: QuadratureInput<'_, F>,
) -> Result<QuadratureEvaluation, CalculusFailure>
where
    F: IntervalDifferentiable + ?Sized,
{
    let QuadratureInput {
        integrand,
        lower,
        upper,
        refinement,
        request,
    } = input;
    let TranscendentalEvaluationRequest {
        owner,
        requirement,
        source,
    } = request;
    let operation = EvaluationOperation::Quadrature;
    let requested = requirement.precision();
    if requested.get() == 0 || requested.get() > MAX_PRECISION_BITS {
        return Err(precision_limit_failure(
            operation,
            owner,
            requirement,
            source,
            input_snapshot(lower, upper, refinement),
            requested,
        ));
    }
    if lower >= upper {
        let witness = match (lower.try_clone(), upper.try_clone()) {
            (Ok(lower), Ok(upper)) => DomainWitness::NonIncreasingBounds { lower, upper },
            (Err(error), _) | (_, Err(error)) => {
                return Err(evaluation_lower_failure(
                    operation,
                    owner,
                    requirement,
                    source,
                    input_snapshot(lower, upper, refinement),
                    error.into(),
                ))
            }
        };
        return Err(EvaluationFailure::QuadratureDomainViolation {
            operation,
            owner,
            requirement,
            source,
            input: input_snapshot(lower, upper, refinement),
            witness,
        }
        .into());
    }
    match refine(integrand, &lower, &upper, refinement, requested) {
        Ok(Refinement::Converged(reached)) => Ok(QuadratureEvaluation::new(
            Enclosure::from_dyadic_enclosure(reached.enclosure),
            owner,
            requirement,
            source,
            reached.evidence,
        )),
        Ok(Refinement::Exhausted(reached)) => Err(EvaluationFailure::QuadratureNotConverged {
            operation,
            owner,
            requirement,
            source,
            input: input_snapshot(lower, upper, refinement),
            achieved: Enclosure::from_dyadic_enclosure(reached.enclosure),
            evidence: reached.evidence,
        }
        .into()),
        Err(fault) => Err(evaluation_lower_failure(
            operation,
            owner,
            requirement,
            source,
            input_snapshot(lower, upper, refinement),
            fault.into(),
        )),
    }
}

fn input_snapshot(
    lower: Dyadic,
    upper: Dyadic,
    refinement: QuadratureRefinement,
) -> EvaluationInputSnapshot {
    EvaluationInputSnapshot::Quadrature {
        lower,
        upper,
        order: refinement.order(),
        maximum_pieces: refinement.maximum_pieces(),
    }
}

enum QuadratureFault {
    Arithmetic(BigintError),
    Integrand(Box<CalculusFailure>),
}

impl From<BigintError> for QuadratureFault {
    fn from(error: BigintError) -> Self {
        Self::Arithmetic(error)
    }
}

impl From<QuadratureFault> for CalculusFailure {
    fn from(fault: QuadratureFault) -> Self {
        match fault {
            QuadratureFault::Arithmetic(error) => Self::Arithmetic(error),
            QuadratureFault::Integrand(failure) => *failure,
        }
    }
}

struct Reached {
    enclosure: DyadicEnclosure,
    evidence: QuadratureEvidence,
}

enum Refinement {
    Converged(Reached),
    Exhausted(Reached),
}

struct Piece {
    lower: Dyadic,
    upper: Dyadic,
    enclosure: DyadicEnclosure,
    width: Dyadic,
    remainder_width: Dyadic,
}

impl Ord for Piece {
    fn cmp(&self, other: &Self) -> Ordering {
        self.width
            .cmp(&other.width)
            .then_with(|| other.lower.cmp(&self.lower))
    }
}

impl PartialOrd for Piece {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Piece {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Piece {}

struct Totals {
    lower: Dyadic,
    upper: Dyadic,
    remainder_width: Dyadic,
}

impl Totals {
    fn new() -> Self {
        Self {
            lower: zero(),
            upper: zero(),
            remainder_width: zero(),
        }
    }

    fn include(&mut self, piece: &Piece) -> Result<(), BigintError> {
        self.lower = self.lower.add(piece.enclosure.lower())?;
        self.upper = self.upper.add(piece.enclosure.upper())?;
        self.remainder_width = self.remainder_width.add(&piece.remainder_width)?;
        Ok(())
    }

    fn exclude(&mut self, piece: &Piece) -> Result<(), BigintError> {
        self.lower = self.lower.sub(piece.enclosure.lower())?;
        self.upper = self.upper.sub(piece.enclosure.upper())?;
        self.remainder_width = self.remainder_width.sub(&piece.remainder_width)?;
        Ok(())
    }

    fn width(&self) -> Result<Dyadic, BigintError> {
        self.upper.sub(&self.lower)
    }

    fn into_reached(self, pieces: usize) -> Result<Reached, BigintError> {
        Ok(Reached {
            enclosure: DyadicEnclosure::new(self.lower, self.upper)?,
            evidence: QuadratureEvidence {
                pieces,
                remainder_width: self.remainder_width,
            },
        })
    }
}

fn zero() -> Dyadic {
    Dyadic::new(BigInt::zero(), 0)
}

struct TaylorRule {
    pairs: usize,
    working_bits: u32,
    reciprocals: Vec<DyadicEnclosure>,
}

impl TaylorRule {
    fn new(refinement: QuadratureRefinement, target: AbsoluteBits) -> Result<Self, BigintError> {
        let pieces_bits = usize::BITS - (refinement.maximum_pieces().get() - 1).leading_zeros();
        let working_bits = target
            .get()
            .saturating_add(WORKING_GUARD_BITS)
            .saturating_add(pieces_bits)
            .min(MAX_PRECISION_BITS);
        let pairs = refinement.order().pairs();
        Ok(Self {
            pairs: pairs as usize,
            working_bits,
            reciprocals: odd_factorial_reciprocals(pairs, working_bits)?,
        })
    }

    fn piece<F>(
        &self,
        integrand: &F,
        lower: Dyadic,
        upper: Dyadic,
    ) -> Result<Piece, QuadratureFault>
    where
        F: IntervalDifferentiable + ?Sized,
    {
        let half_width = scale_by_power_of_two(&upper.sub(&lower)?, -1)?;
        let center_domain =
            Enclosure::from_dyadic_enclosure(point_enclosure(&lower.midpoint(&upper)?)?);
        let piece_domain = Enclosure::from_dyadic_enclosure(DyadicEnclosure::new(
            lower.try_clone()?,
            upper.try_clone()?,
        )?);
        let square = half_width.mul(&half_width)?;
        let mut power = half_width;
        let mut sum = point_enclosure(&zero())?;
        let mut remainder_width = zero();
        for (index, reciprocal) in self.reciprocals.iter().enumerate() {
            let is_remainder = index == self.pairs;
            let domain = if is_remainder {
                &piece_domain
            } else {
                &center_domain
            };
            let value = self.evaluate_term(integrand, index, domain)?;
            let weight = enclosure_mul(
                &point_enclosure(&scale_by_power_of_two(&power, 1)?)?,
                reciprocal,
            )?;
            let term = enclosure_mul(value.as_dyadic_enclosure(), &weight)?;
            sum = enclosure_add(&sum, &term)?;
            if is_remainder {
                remainder_width = term.width()?;
            } else {
                power = power.mul(&square)?;
            }
        }
        let width = sum.width()?;
        Ok(Piece {
            lower,
            upper,
            enclosure: sum,
            width,
            remainder_width,
        })
    }

    fn evaluate_term<F>(
        &self,
        integrand: &F,
        index: usize,
        domain: &Enclosure,
    ) -> Result<Enclosure, QuadratureFault>
    where
        F: IntervalDifferentiable + ?Sized,
    {
        let result = if index == 0 {
            integrand.evaluate_interval(domain, self.working_bits)
        } else {
            integrand.evaluate_derivative_interval(
                DerivativeOrder::new(2 * index),
                domain,
                self.working_bits,
            )
        };
        result.map_err(|failure| QuadratureFault::Integrand(Box::new(failure)))
    }
}

fn odd_factorial_reciprocals(pairs: u32, bits: u32) -> Result<Vec<DyadicEnclosure>, BigintError> {
    let mut reciprocals = Vec::new();
    let mut factorial = BigUint::one()?;
    for index in 0..=pairs {
        if index > 0 {
            factorial = factorial
                .mul(&BigUint::try_from(2 * index)?)?
                .mul(&BigUint::try_from(2 * index + 1)?)?;
        }
        let reciprocal = RawRational::new(BigInt::one()?, factorial.try_clone()?)
            .reduce()?
            .into_reduced();
        reciprocals.push(DyadicEnclosure::new(
            reciprocal.dyadic_floor(bits)?,
            reciprocal.dyadic_ceil(bits)?,
        )?);
    }
    Ok(reciprocals)
}

fn refine<F>(
    integrand: &F,
    lower: &Dyadic,
    upper: &Dyadic,
    refinement: QuadratureRefinement,
    target: AbsoluteBits,
) -> Result<Refinement, QuadratureFault>
where
    F: IntervalDifferentiable + ?Sized,
{
    let rule = TaylorRule::new(refinement, target)?;
    let maximum_width = Dyadic::new(BigInt::one()?, target.get());
    let first = rule.piece(integrand, lower.try_clone()?, upper.try_clone()?)?;
    let mut totals = Totals::new();
    totals.include(&first)?;
    let mut pieces = BinaryHeap::new();
    pieces.push(first);
    loop {
        if totals.width()? <= maximum_width {
            return Ok(Refinement::Converged(totals.into_reached(pieces.len())?));
        }
        let refinable = if pieces.len() < refinement.maximum_pieces().get() {
            pieces.pop()
        } else {
            None
        };
        let Some(widest) = refinable else {
            return Ok(Refinement::Exhausted(totals.into_reached(pieces.len())?));
        };
        totals.exclude(&widest)?;
        let center = widest.lower.midpoint(&widest.upper)?;
        let left = rule.piece(integrand, widest.lower, center.try_clone()?)?;
        let right = rule.piece(integrand, center, widest.upper)?;
        totals.include(&left)?;
        totals.include(&right)?;
        pieces.push(left);
        pieces.push(right);
    }
}
