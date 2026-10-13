//! The `*_200` constants hold the floor of value * 2^200 in hex, from mpmath and again from an
//! exact rational series. The ceiling is the next integer because no value is a multiple of 2^-200.

use std::cell::RefCell;
use std::num::NonZeroUsize;

use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, Dyadic, Sign};
use neco_calculus::{
    evaluate_quadrature, evaluate_transcendental_function, AbsoluteBits, CalculusFailure,
    ConsumerId, DerivativeOrder, DomainWitness, Enclosure, EvaluationFailure,
    EvaluationInputSnapshot, EvaluationOperation, ExactDecisionRequest, ExactExpressionRequirement,
    ExprId, IntervalDifferentiable, IntervalEvaluable, JetFailure, LowerEvaluationFailure,
    NecoImplementationSource, OwnerId, Polynomial, PolynomialFailure, PolynomialResourceFailure,
    PrecisionShortageCause, PrecisionWitness, QuadratureEvaluation, QuadratureEvidence,
    QuadratureInput, QuadratureOrder, QuadratureRefinement, RationalFailure, RationalFunction,
    TranscendentalEvaluationKind, TranscendentalEvaluationRequest, TranscendentalFunction,
    TranscendentalFunctionEvaluationInput, MAX_PRECISION_BITS, MAX_QUADRATURE_PIECES,
};

const ONE_THIRD_200: &str = "55555555555555555555555555555555555555555555555555";
const ONE_MINUS_COS_ONE_200: &str = "75aebf8257cba36e3db99268978e42d65dc8c576b0693c480d";
const E_MINUS_ONE_200: &str = "1b7e151628aed2a6abf7158809cf4f3c762e7160f38b4da56a7";
const SIN_HALF_200: &str = "7abba1d12c17bfa1d92f0d93f60ded9992f45b4fcaf13cd58b";
const SIN_ONE_200: &str = "d76aa47848677020c6e9e909c50f3c3289e511132f518b4def";
const EXP_HALF_200: &str = "1a61298e1e069bc972dfefab6df33f9b1f651f16c130b4759c4";
const EXP_ONE_200: &str = "2b7e151628aed2a6abf7158809cf4f3c762e7160f38b4da56a7";

fn integer(value: i64) -> BigInt {
    BigInt::try_from(value).unwrap()
}

fn dyadic(value: i64, exponent: u32) -> Dyadic {
    Dyadic::new(integer(value), exponent)
}

fn hex_dyadic(digits: &str, exponent: u32) -> Dyadic {
    let sixteen = integer(16);
    let mut value = BigInt::zero();
    for digit in digits.chars() {
        let digit = i64::from(digit.to_digit(16).unwrap());
        value = value.mul(&sixteen).unwrap().add(&integer(digit)).unwrap();
    }
    Dyadic::new(value, exponent)
}

fn unit(exponent: u32) -> Dyadic {
    Dyadic::new(BigInt::one().unwrap(), exponent)
}

fn interval(lower: Dyadic, upper: Dyadic) -> Enclosure {
    Enclosure::new(lower, upper).unwrap()
}

fn point(value: Dyadic) -> Enclosure {
    Enclosure::new(value.try_clone().unwrap(), value).unwrap()
}

fn copy(enclosure: &Enclosure) -> Enclosure {
    interval(
        enclosure.lower().try_clone().unwrap(),
        enclosure.upper().try_clone().unwrap(),
    )
}

fn negate(enclosure: &Enclosure) -> Enclosure {
    let zero = dyadic(0, 0);
    interval(
        zero.sub(enclosure.upper()).unwrap(),
        zero.sub(enclosure.lower()).unwrap(),
    )
}

fn algebraic(value: i64) -> RealAlgebraic {
    RealAlgebraic::from_integer(integer(value)).unwrap()
}

fn requirement(consumer: u32, expression: u32, precision: u32) -> ExactExpressionRequirement {
    ExactExpressionRequirement::new(
        ConsumerId::new(consumer),
        &[ExprId::new(expression)],
        ExactDecisionRequest::Zero(ExprId::new(expression)),
        AbsoluteBits::new(precision),
    )
    .unwrap()
}

fn source() -> NecoImplementationSource {
    NecoImplementationSource::current()
}

fn owner() -> OwnerId {
    OwnerId::new(1)
}

fn request(precision: u32) -> TranscendentalEvaluationRequest {
    TranscendentalEvaluationRequest::new(owner(), requirement(11, 500, precision), source())
}

fn refinement(pairs: u32, maximum_pieces: usize) -> QuadratureRefinement {
    QuadratureRefinement::new(
        QuadratureOrder::new(pairs).unwrap(),
        NonZeroUsize::new(maximum_pieces).unwrap(),
    )
    .unwrap()
}

fn integrate<F>(
    integrand: &F,
    bounds: (Dyadic, Dyadic),
    pairs: u32,
    maximum_pieces: usize,
    precision: u32,
) -> Result<QuadratureEvaluation, CalculusFailure>
where
    F: IntervalDifferentiable + ?Sized,
{
    evaluate_quadrature(QuadratureInput::new(
        integrand,
        bounds.0,
        bounds.1,
        refinement(pairs, maximum_pieces),
        request(precision),
    ))
}

fn unit_interval() -> (Dyadic, Dyadic) {
    (dyadic(0, 0), dyadic(1, 0))
}

fn snapshot(
    bounds: (Dyadic, Dyadic),
    pairs: u32,
    maximum_pieces: usize,
) -> EvaluationInputSnapshot {
    EvaluationInputSnapshot::Quadrature {
        lower: bounds.0,
        upper: bounds.1,
        order: QuadratureOrder::new(pairs).unwrap(),
        maximum_pieces: NonZeroUsize::new(maximum_pieces).unwrap(),
    }
}

fn assert_contains_reference(enclosure: &Enclosure, digits: &str, exponent: u32) {
    let floor = hex_dyadic(digits, exponent);
    let ceiling = floor.add(&unit(exponent)).unwrap();
    assert!(enclosure.contains(&floor));
    assert!(enclosure.contains(&ceiling));
}

fn assert_lineage(
    owner: OwnerId,
    requirement: &ExactExpressionRequirement,
    source: NecoImplementationSource,
    precision: u32,
) {
    assert_eq!(owner, OwnerId::new(1));
    assert_eq!(requirement.consumer_id(), ConsumerId::new(11));
    assert_eq!(requirement.expressions(), &[ExprId::new(500)]);
    assert_eq!(requirement.precision(), AbsoluteBits::new(precision));
    assert_eq!(requirement, &self::requirement(11, 500, precision));
    assert_eq!(source, NecoImplementationSource::current());
}

fn power_of_two(exponent: u32) -> BigInt {
    let magnitude = BigUint::one().unwrap().shl_bits(exponent as usize).unwrap();
    BigInt::from_sign_magnitude(Sign::Positive, magnitude)
}

fn at_most_ratio(value: &Dyadic, numerator: i64, denominator: &BigInt) -> bool {
    let scaled = integer(numerator)
        .mul(&power_of_two(value.exponent()))
        .unwrap();
    value.integer().mul(denominator).unwrap() <= scaled
}

fn at_least_ratio(value: &Dyadic, numerator: i64, denominator: &BigInt) -> bool {
    let scaled = integer(numerator)
        .mul(&power_of_two(value.exponent()))
        .unwrap();
    value.integer().mul(denominator).unwrap() >= scaled
}

/// Exact integer comparison with numerator / denominator, for a positive denominator.
fn assert_contains_ratio(enclosure: &Enclosure, numerator: i64, denominator: &BigInt) {
    assert!(at_most_ratio(enclosure.lower(), numerator, denominator));
    assert!(at_least_ratio(enclosure.upper(), numerator, denominator));
}

/// The value lies in [numerator / denominator, numerator / denominator + 2^-slack_bits].
fn assert_within_ratio(value: &Dyadic, numerator: i64, denominator: &BigInt, slack_bits: u32) {
    assert!(at_least_ratio(value, numerator, denominator));
    let reduced = value.sub(&unit(slack_bits)).unwrap();
    assert!(at_most_ratio(&reduced, numerator, denominator));
}

fn evaluation_failure<T: core::fmt::Debug>(
    result: Result<T, CalculusFailure>,
) -> EvaluationFailure {
    match result {
        Err(CalculusFailure::Evaluation(failure)) => *failure,
        other => panic!("expected an evaluation failure: {other:?}"),
    }
}

struct Unmet {
    achieved: Enclosure,
    evidence: QuadratureEvidence,
    input: EvaluationInputSnapshot,
}

fn not_converged(result: Result<QuadratureEvaluation, CalculusFailure>) -> Unmet {
    match evaluation_failure(result) {
        EvaluationFailure::QuadratureNotConverged {
            achieved,
            evidence,
            input,
            ..
        } => Unmet {
            achieved,
            evidence,
            input,
        },
        other => panic!("unexpected result: {other:?}"),
    }
}

fn reached(result: Result<QuadratureEvaluation, CalculusFailure>) -> Enclosure {
    match result {
        Ok(evaluation) => copy(evaluation.enclosure()),
        other => not_converged(other).achieved,
    }
}

fn square() -> Polynomial {
    Polynomial::from_coefficients(vec![algebraic(0), algebraic(0), algebraic(1)]).unwrap()
}

#[derive(Debug, Eq, PartialEq)]
enum Entry {
    Value,
    Derivative(usize),
}

#[derive(Debug, Eq, PartialEq)]
struct Call {
    entry: Entry,
    lower: Dyadic,
    upper: Dyadic,
    bits: u32,
}

struct Recording<'a, F: ?Sized> {
    inner: &'a F,
    calls: RefCell<Vec<Call>>,
}

impl<'a, F: ?Sized> Recording<'a, F> {
    fn new(inner: &'a F) -> Self {
        Self {
            inner,
            calls: RefCell::new(Vec::new()),
        }
    }

    fn record(&self, entry: Entry, domain: &Enclosure, bits: u32) {
        self.calls.borrow_mut().push(Call {
            entry,
            lower: domain.lower().try_clone().unwrap(),
            upper: domain.upper().try_clone().unwrap(),
            bits,
        });
    }

    fn derivative_orders(&self) -> Vec<usize> {
        let mut orders: Vec<usize> = self
            .calls
            .borrow()
            .iter()
            .filter_map(|call| match call.entry {
                Entry::Derivative(order) => Some(order),
                Entry::Value => None,
            })
            .collect();
        orders.sort_unstable();
        orders
    }
}

impl<F: IntervalEvaluable + ?Sized> IntervalEvaluable for Recording<'_, F> {
    fn evaluate_interval(
        &self,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        self.record(Entry::Value, domain, precision_bits);
        self.inner.evaluate_interval(domain, precision_bits)
    }
}

impl<F: IntervalDifferentiable + ?Sized> IntervalDifferentiable for Recording<'_, F> {
    fn evaluate_derivative_interval(
        &self,
        order: DerivativeOrder,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        self.record(Entry::Derivative(order.get()), domain, precision_bits);
        self.inner
            .evaluate_derivative_interval(order, domain, precision_bits)
    }
}

struct ValueOnly<G>(G);

impl<G> IntervalEvaluable for ValueOnly<G>
where
    G: Fn(&Enclosure, u32) -> Result<Enclosure, CalculusFailure>,
{
    fn evaluate_interval(
        &self,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        (self.0)(domain, precision_bits)
    }
}

impl<G> IntervalDifferentiable for ValueOnly<G>
where
    G: Fn(&Enclosure, u32) -> Result<Enclosure, CalculusFailure>,
{
    fn evaluate_derivative_interval(
        &self,
        _order: DerivativeOrder,
        _domain: &Enclosure,
        _precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        Err(CalculusFailure::Jet(JetFailure::OrderOutOfRange))
    }
}

fn wide_constant() -> ValueOnly<impl Fn(&Enclosure, u32) -> Result<Enclosure, CalculusFailure>> {
    ValueOnly(|_: &Enclosure, _: u32| Ok(interval(dyadic(0, 0), dyadic(1, 0))))
}

fn step(
    threshold: Dyadic,
) -> ValueOnly<impl Fn(&Enclosure, u32) -> Result<Enclosure, CalculusFailure>> {
    ValueOnly(move |domain: &Enclosure, _| {
        if domain.upper() < &threshold {
            Ok(point(dyadic(0, 0)))
        } else if domain.lower() >= &threshold {
            Ok(point(dyadic(1, 0)))
        } else {
            Ok(interval(dyadic(0, 0), dyadic(1, 0)))
        }
    })
}

fn reciprocal() -> ValueOnly<impl Fn(&Enclosure, u32) -> Result<Enclosure, CalculusFailure>> {
    let function = RationalFunction::new(
        Polynomial::from_coefficients(vec![algebraic(1)]).unwrap(),
        Polynomial::from_coefficients(vec![algebraic(0), algebraic(1)]).unwrap(),
    )
    .unwrap();
    ValueOnly(move |domain: &Enclosure, precision_bits| {
        function.evaluate_interval(domain, precision_bits)
    })
}

fn windows(
    first: Dyadic,
    second: Dyadic,
) -> ValueOnly<impl Fn(&Enclosure, u32) -> Result<Enclosure, CalculusFailure>> {
    ValueOnly(move |domain: &Enclosure, _| {
        if domain.contains(&first) || domain.contains(&second) {
            Ok(interval(dyadic(0, 0), dyadic(1, 0)))
        } else {
            Ok(point(dyadic(0, 0)))
        }
    })
}

/// (x - shift)^degree with exact enclosures of every derivative over any interval, so only the
/// outward rounding of the factorial reciprocals widens a result.
struct ShiftedPower {
    shift: Dyadic,
    degree: u32,
}

fn exact_power(shift: Dyadic, degree: u32) -> ShiftedPower {
    ShiftedPower { shift, degree }
}

fn dyadic_power(base: &Dyadic, exponent: u32) -> Dyadic {
    let mut result = dyadic(1, 0);
    for _ in 0..exponent {
        result = result.mul(base).unwrap();
    }
    result
}

fn magnitude(value: &Dyadic) -> Dyadic {
    if value < &dyadic(0, 0) {
        dyadic(0, 0).sub(value).unwrap()
    } else {
        value.try_clone().unwrap()
    }
}

impl ShiftedPower {
    fn range(&self, order: u32, domain: &Enclosure) -> Enclosure {
        if order > self.degree {
            return point(dyadic(0, 0));
        }
        let exponent = self.degree - order;
        let mut scale = dyadic(1, 0);
        for factor in (exponent + 1)..=self.degree {
            scale = scale.mul(&dyadic(i64::from(factor), 0)).unwrap();
        }
        let low = domain.lower().sub(&self.shift).unwrap();
        let high = domain.upper().sub(&self.shift).unwrap();
        let (least, greatest) = if exponent % 2 == 1 || exponent == 0 {
            (dyadic_power(&low, exponent), dyadic_power(&high, exponent))
        } else {
            let (low_size, high_size) = (magnitude(&low), magnitude(&high));
            let (smaller, larger) = if low_size < high_size {
                (low_size, high_size)
            } else {
                (high_size, low_size)
            };
            let crosses_zero = low <= dyadic(0, 0) && high >= dyadic(0, 0);
            let least = if crosses_zero {
                dyadic(0, 0)
            } else {
                dyadic_power(&smaller, exponent)
            };
            (least, dyadic_power(&larger, exponent))
        };
        interval(least.mul(&scale).unwrap(), greatest.mul(&scale).unwrap())
    }
}

impl IntervalEvaluable for ShiftedPower {
    fn evaluate_interval(
        &self,
        domain: &Enclosure,
        _precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        Ok(self.range(0, domain))
    }
}

impl IntervalDifferentiable for ShiftedPower {
    fn evaluate_derivative_interval(
        &self,
        order: DerivativeOrder,
        domain: &Enclosure,
        _precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        Ok(self.range(u32::try_from(order.get()).unwrap(), domain))
    }
}

/// The constant 1 with looser enclosures than its value needs: the value is only known within
/// 2^-5 and every derivative only within [0, 6].
struct LooseConstant;

impl IntervalEvaluable for LooseConstant {
    fn evaluate_interval(
        &self,
        _domain: &Enclosure,
        _precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        Ok(interval(dyadic(31, 5), dyadic(33, 5)))
    }
}

impl IntervalDifferentiable for LooseConstant {
    fn evaluate_derivative_interval(
        &self,
        _order: DerivativeOrder,
        _domain: &Enclosure,
        _precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        Ok(interval(dyadic(0, 0), dyadic(6, 0)))
    }
}

/// The constant 1 again. The value enclosure is loose for pieces centered from 1/2 on, and the
/// second derivative enclosure is loose for pieces starting below 1/2.
struct LooseHalves;

impl IntervalEvaluable for LooseHalves {
    fn evaluate_interval(
        &self,
        domain: &Enclosure,
        _precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        Ok(if domain.lower() >= &dyadic(1, 1) {
            interval(dyadic(3, 2), dyadic(5, 2))
        } else {
            point(dyadic(1, 0))
        })
    }
}

impl IntervalDifferentiable for LooseHalves {
    fn evaluate_derivative_interval(
        &self,
        order: DerivativeOrder,
        domain: &Enclosure,
        _precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        if order.get() != 2 {
            return Err(CalculusFailure::Jet(JetFailure::OrderOutOfRange));
        }
        Ok(if domain.lower() < &dyadic(1, 1) {
            interval(dyadic(0, 0), dyadic(1, 0))
        } else {
            point(dyadic(0, 0))
        })
    }
}

/// x^2 with a value enclosure wider by 2^-20 on each side. Every derivative enclosure is exact.
struct LooseSquare;

impl IntervalEvaluable for LooseSquare {
    fn evaluate_interval(
        &self,
        domain: &Enclosure,
        _precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        let slack = unit(20);
        Ok(interval(
            domain
                .lower()
                .mul(domain.lower())
                .unwrap()
                .sub(&slack)
                .unwrap(),
            domain
                .upper()
                .mul(domain.upper())
                .unwrap()
                .add(&slack)
                .unwrap(),
        ))
    }
}

impl IntervalDifferentiable for LooseSquare {
    fn evaluate_derivative_interval(
        &self,
        order: DerivativeOrder,
        _domain: &Enclosure,
        _precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        Ok(point(dyadic(if order.get() == 2 { 2 } else { 0 }, 0)))
    }
}

fn transcendental_range(
    function: TranscendentalFunction,
    domain: &Enclosure,
    precision_bits: u32,
) -> Result<Enclosure, CalculusFailure> {
    let evaluation = evaluate_transcendental_function(TranscendentalFunctionEvaluationInput {
        function,
        argument: copy(domain),
        request: request(precision_bits),
    })?;
    Ok(match evaluation.kind() {
        TranscendentalEvaluationKind::Enclosure => copy(evaluation.as_enclosure().unwrap()),
        TranscendentalEvaluationKind::Undetermined => {
            copy(evaluation.undetermined_achieved().unwrap())
        }
        TranscendentalEvaluationKind::ExactAlgebraic => {
            let isolating = evaluation
                .as_exact_algebraic()
                .unwrap()
                .enclose(precision_bits)
                .unwrap();
            Enclosure::from_dyadic_enclosure(isolating.enclosure().try_clone().unwrap())
        }
    })
}

/// The quadrature asks for even orders only, so these integrands refuse odd orders to expose any
/// other request.
struct Sine;

impl IntervalEvaluable for Sine {
    fn evaluate_interval(
        &self,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        transcendental_range(TranscendentalFunction::Sine, domain, precision_bits)
    }
}

impl IntervalDifferentiable for Sine {
    fn evaluate_derivative_interval(
        &self,
        order: DerivativeOrder,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        match order.get() % 4 {
            0 => transcendental_range(TranscendentalFunction::Sine, domain, precision_bits),
            2 => transcendental_range(TranscendentalFunction::Sine, domain, precision_bits)
                .map(|sine| negate(&sine)),
            _ => Err(CalculusFailure::Jet(JetFailure::OrderOutOfRange)),
        }
    }
}

struct Exponential;

impl IntervalEvaluable for Exponential {
    fn evaluate_interval(
        &self,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        transcendental_range(TranscendentalFunction::Exponential, domain, precision_bits)
    }
}

impl IntervalDifferentiable for Exponential {
    fn evaluate_derivative_interval(
        &self,
        order: DerivativeOrder,
        domain: &Enclosure,
        precision_bits: u32,
    ) -> Result<Enclosure, CalculusFailure> {
        if order.get() % 2 == 0 {
            transcendental_range(TranscendentalFunction::Exponential, domain, precision_bits)
        } else {
            Err(CalculusFailure::Jet(JetFailure::OrderOutOfRange))
        }
    }
}

#[test]
fn square_integral_encloses_one_third_without_refinement() {
    let evaluation = integrate(&square(), unit_interval(), 2, 1, 64).unwrap();
    assert_contains_reference(evaluation.enclosure(), ONE_THIRD_200, 200);
    assert!(evaluation.enclosure().width().unwrap() <= unit(64));
    assert_eq!(evaluation.evidence().pieces(), 1);
    assert!(evaluation.evidence().remainder_width() <= &unit(64));
}

#[test]
fn square_integral_range_only_refines_and_encloses_one_third() {
    let polynomial = square();
    let recording = Recording::new(&polynomial);
    let evaluation = integrate(&recording, unit_interval(), 0, 4096, 8).unwrap();
    assert_contains_reference(evaluation.enclosure(), ONE_THIRD_200, 200);
    assert!(evaluation.enclosure().width().unwrap() <= unit(8));
    let pieces = evaluation.evidence().pieces();
    assert!(pieces > 1 && pieces <= 4096);
    assert!(recording.derivative_orders().is_empty());
}

#[test]
fn taylor_terms_use_only_even_orders_and_wide_remainder() {
    let polynomial = square();
    let recording = Recording::new(&polynomial);
    integrate(&recording, unit_interval(), 2, 1, 64).unwrap();
    let expected = [
        Call {
            entry: Entry::Value,
            lower: dyadic(1, 1),
            upper: dyadic(1, 1),
            bits: 72,
        },
        Call {
            entry: Entry::Derivative(2),
            lower: dyadic(1, 1),
            upper: dyadic(1, 1),
            bits: 72,
        },
        Call {
            entry: Entry::Derivative(4),
            lower: dyadic(0, 0),
            upper: dyadic(1, 0),
            bits: 72,
        },
    ];
    let calls = recording.calls.borrow();
    assert_eq!(calls.len(), expected.len());
    for call in &expected {
        assert_eq!(calls.iter().filter(|recorded| *recorded == call).count(), 1);
    }
}

#[test]
fn exact_terms_keep_the_integral_inside_after_outward_rounding() {
    let integrand = exact_power(dyadic(0, 0), 2);
    let evaluation = integrate(&integrand, unit_interval(), 2, 1, 64).unwrap();
    assert_contains_reference(evaluation.enclosure(), ONE_THIRD_200, 200);
    assert!(evaluation.enclosure().width().unwrap() <= unit(64));
}

#[test]
fn transcendental_integrands_enclose_their_range_over_the_domain() {
    let (lower, upper) = unit_interval();
    let domain = interval(lower, upper);
    let sine = Sine.evaluate_interval(&domain, 64).unwrap();
    assert!(sine.contains(&dyadic(0, 0)));
    assert_contains_reference(&sine, SIN_HALF_200, 200);
    assert_contains_reference(&sine, SIN_ONE_200, 200);
    let second = Sine
        .evaluate_derivative_interval(DerivativeOrder::new(2), &domain, 64)
        .unwrap();
    assert_eq!(second, negate(&sine));
    let fourth = Sine
        .evaluate_derivative_interval(DerivativeOrder::new(4), &domain, 64)
        .unwrap();
    assert_eq!(fourth, sine);

    let exponential = Exponential.evaluate_interval(&domain, 64).unwrap();
    assert!(exponential.contains(&dyadic(1, 0)));
    assert_contains_reference(&exponential, EXP_HALF_200, 200);
    assert_contains_reference(&exponential, EXP_ONE_200, 200);
    let second = Exponential
        .evaluate_derivative_interval(DerivativeOrder::new(2), &domain, 64)
        .unwrap();
    assert_eq!(second, exponential);
}

#[test]
fn sine_integral_encloses_one_minus_cosine_one() {
    let evaluation = integrate(&Sine, unit_interval(), 4, 256, 64).unwrap();
    assert_contains_reference(evaluation.enclosure(), ONE_MINUS_COS_ONE_200, 200);
    assert!(evaluation.enclosure().width().unwrap() <= unit(64));
}

#[test]
fn exponential_integral_encloses_e_minus_one() {
    let evaluation = integrate(&Exponential, unit_interval(), 4, 256, 64).unwrap();
    assert_contains_reference(evaluation.enclosure(), E_MINUS_ONE_200, 200);
    assert!(evaluation.enclosure().width().unwrap() <= unit(64));
}

#[test]
fn unrefined_remainder_stays_inside_reported_enclosure() {
    match evaluation_failure(integrate(&Exponential, unit_interval(), 1, 1, 64)) {
        EvaluationFailure::QuadratureNotConverged {
            operation,
            achieved,
            evidence,
            ..
        } => {
            assert_eq!(operation, EvaluationOperation::Quadrature);
            assert_contains_reference(&achieved, E_MINUS_ONE_200, 200);
            assert!(evidence.remainder_width() > &dyadic(0, 0));
            assert_eq!(evidence.pieces(), 1);
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn nonshrinking_integrand_is_not_converged() {
    match evaluation_failure(integrate(&wide_constant(), unit_interval(), 0, 64, 20)) {
        EvaluationFailure::QuadratureNotConverged {
            achieved, evidence, ..
        } => {
            assert_eq!(evidence.pieces(), 64);
            assert_eq!(achieved.lower(), &dyadic(0, 0));
            assert_eq!(achieved.upper(), &dyadic(1, 0));
            assert_eq!(evidence.remainder_width(), &dyadic(1, 0));
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn step_integrand_refines_locally() {
    let evaluation = integrate(&step(dyadic(3, 3)), unit_interval(), 0, 64, 10).unwrap();
    assert_eq!(evaluation.evidence().pieces(), 11);
    let enclosure = evaluation.enclosure();
    assert_eq!(enclosure.lower(), &dyadic(5, 3));
    assert_eq!(enclosure.upper(), &dyadic(5, 3).add(&unit(10)).unwrap());
    assert_eq!(evaluation.evidence().remainder_width(), &unit(10));
}

#[test]
fn equal_width_pieces_are_split_lowest_first() {
    let integrand = windows(dyadic(1, 2), dyadic(11, 4));
    match evaluation_failure(integrate(&integrand, unit_interval(), 0, 3, 20)) {
        EvaluationFailure::QuadratureNotConverged {
            achieved, evidence, ..
        } => {
            assert_eq!(evidence.pieces(), 3);
            assert_eq!(achieved.lower(), &dyadic(0, 0));
            assert_eq!(achieved.upper(), &dyadic(1, 0));
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

fn assert_non_increasing(bounds: (i64, u32, i64, u32)) {
    let (lower, lower_exponent, upper, upper_exponent) = bounds;
    let make = || (dyadic(lower, lower_exponent), dyadic(upper, upper_exponent));
    match evaluation_failure(integrate(&square(), make(), 2, 1, 64)) {
        EvaluationFailure::QuadratureDomainViolation {
            operation,
            owner,
            requirement,
            source,
            input,
            witness,
        } => {
            assert_eq!(operation, EvaluationOperation::Quadrature);
            assert_lineage(owner, &requirement, source, 64);
            assert_eq!(input, snapshot(make(), 2, 1));
            let (lower, upper) = make();
            assert_eq!(witness, DomainWitness::NonIncreasingBounds { lower, upper });
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn reversed_bounds_are_rejected() {
    assert_non_increasing((1, 0, 0, 0));
}

#[test]
fn empty_bounds_are_rejected() {
    assert_non_increasing((1, 1, 1, 1));
}

#[test]
fn precision_limit_is_structural_and_checked_first() {
    let cases = [
        (257, (dyadic(0, 0), dyadic(1, 0))),
        (257, (dyadic(1, 0), dyadic(0, 0))),
        (0, (dyadic(0, 0), dyadic(1, 0))),
    ];
    for (precision, bounds) in cases {
        let expected = snapshot(
            (bounds.0.try_clone().unwrap(), bounds.1.try_clone().unwrap()),
            2,
            1,
        );
        match evaluation_failure(integrate(&square(), bounds, 2, 1, precision)) {
            EvaluationFailure::PrecisionShortage {
                operation,
                owner,
                requirement,
                source,
                input,
                requested,
                witness,
                lower,
            } => {
                assert_eq!(operation, EvaluationOperation::Quadrature);
                assert_lineage(owner, &requirement, source, precision);
                assert_eq!(input, expected);
                assert_eq!(requested, AbsoluteBits::new(precision));
                assert_eq!(
                    witness,
                    PrecisionWitness::Limit {
                        maximum: MAX_PRECISION_BITS
                    }
                );
                assert_eq!(
                    lower,
                    PrecisionShortageCause::RequestedBeyondLimit {
                        requested: AbsoluteBits::new(precision),
                        maximum: MAX_PRECISION_BITS,
                    }
                );
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
}

#[test]
fn piece_and_order_limits_are_rejected() {
    let order = QuadratureOrder::new(2).unwrap();
    let pieces = |count: usize| NonZeroUsize::new(count).unwrap();
    assert_eq!(
        QuadratureRefinement::new(order, pieces(MAX_QUADRATURE_PIECES + 1)),
        Err(CalculusFailure::Evaluation(Box::new(
            EvaluationFailure::QuadraturePieceLimitExceeded {
                requested: 65_537,
                maximum: 65_536,
            }
        )))
    );
    assert!(QuadratureRefinement::new(order, pieces(MAX_QUADRATURE_PIECES)).is_ok());

    assert_eq!(
        QuadratureOrder::new(33),
        Err(CalculusFailure::Polynomial(PolynomialFailure::Resource(
            PolynomialResourceFailure::DerivativeOrderLimitExceeded {
                requested: 66,
                maximum: 64,
            }
        )))
    );
    let boundary = QuadratureOrder::new(32).unwrap();
    assert_eq!(boundary.pairs(), 32);
    assert_eq!(boundary.derivative_order(), DerivativeOrder::new(64));
    assert!(QuadratureOrder::new(u32::MAX).is_err());
}

#[test]
fn integrand_failure_is_preserved_as_lower_failure() {
    let bounds = || (dyadic(-1, 0), dyadic(1, 0));
    match evaluation_failure(integrate(&reciprocal(), bounds(), 0, 64, 20)) {
        EvaluationFailure::Lower {
            operation,
            owner,
            requirement,
            source,
            input,
            witness,
            lower,
        } => {
            assert_eq!(operation, EvaluationOperation::Quadrature);
            assert_lineage(owner, &requirement, source, 20);
            assert_eq!(witness.operation(), EvaluationOperation::Quadrature);
            assert_eq!(input, snapshot(bounds(), 0, 64));
            match lower {
                LowerEvaluationFailure::Calculus(failure) => assert_eq!(
                    *failure,
                    CalculusFailure::Rational(RationalFailure::DomainContainsZero)
                ),
                other => panic!("unexpected lower failure: {other:?}"),
            }
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn unsupported_derivative_order_is_lower_failure() {
    match evaluation_failure(integrate(&step(dyadic(3, 3)), unit_interval(), 1, 64, 10)) {
        EvaluationFailure::Lower {
            operation,
            owner,
            requirement,
            source,
            lower,
            ..
        } => {
            assert_eq!(operation, EvaluationOperation::Quadrature);
            assert_lineage(owner, &requirement, source, 10);
            match lower {
                LowerEvaluationFailure::Calculus(failure) => {
                    assert_eq!(*failure, CalculusFailure::Jet(JetFailure::OrderOutOfRange))
                }
                other => panic!("unexpected lower failure: {other:?}"),
            }
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn result_and_failure_keep_requirement_lineage() {
    let evaluation = integrate(&square(), unit_interval(), 2, 1, 64).unwrap();
    assert_lineage(
        evaluation.owner(),
        evaluation.requirement(),
        evaluation.source(),
        64,
    );

    match evaluation_failure(integrate(&Exponential, unit_interval(), 1, 1, 64)) {
        EvaluationFailure::QuadratureNotConverged {
            owner,
            requirement,
            source,
            ..
        } => assert_lineage(owner, &requirement, source, 64),
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn integral_certify_f64_reports_actual_absolute_error() {
    let evaluation = integrate(&square(), unit_interval(), 2, 1, 64).unwrap();
    let certified = evaluation.certify_f64().unwrap();
    assert_eq!(certified.value(), 1.0_f64 / 3.0);
    assert_eq!(
        certified.enclosure(),
        evaluation.enclosure().as_dyadic_enclosure()
    );

    let tripled = certified.absolute_error().mul(&dyadic(3, 0)).unwrap();
    assert!(tripled >= unit(54));
    assert!(tripled <= unit(54).add(&dyadic(3, 0).mul(&unit(64)).unwrap()).unwrap());
    assert_ne!(certified.absolute_error(), &unit(64));
}

/// The fourth derivative of x^2 is exactly 0, so the remainder term is [0, 0] while the
/// rounded 1/3! widens the second center term.
#[test]
fn remainder_width_leaves_out_the_center_terms() {
    let integrand = exact_power(dyadic(0, 0), 2);
    let evaluation = integrate(&integrand, unit_interval(), 2, 1, 64).unwrap();
    let evidence = evaluation.evidence();
    assert_eq!(evidence.pieces(), 1);
    assert!(evidence.remainder_width().is_zero());
    let width = evaluation.enclosure().width().unwrap();
    assert!(width > dyadic(0, 0));
    assert!(width <= unit(64));
}

/// The 64 pieces have half-width 2^-7, so each remainder term is [0, 6] * 2h^3 / 3!, which is
/// [0, 2^-20] up to rounding, and the 64 terms sum to 2^-14. The center terms are exact and
/// add the value width 2^-4 once over the whole interval.
#[test]
fn remainder_width_sums_every_piece_and_leaves_out_the_center_widths() {
    let unmet = not_converged(integrate(&LooseConstant, unit_interval(), 1, 64, 8));
    assert_eq!(unmet.evidence.pieces(), 64);
    let remainder = unmet.evidence.remainder_width();
    assert!(remainder >= &unit(14));
    assert!(remainder <= &unit(14).add(&unit(30)).unwrap());
    let width = unmet.achieved.width().unwrap();
    assert_eq!(width, unit(4).add(remainder).unwrap());
    assert_eq!(unmet.achieved.lower(), &dyadic(31, 5));
}

/// The value enclosures of the pieces add up to 2^-4 at every depth, while the widest piece
/// is already below 2^-8 once there are 32 pieces.
#[test]
fn success_needs_the_total_width_not_the_widest_piece() {
    let unmet = not_converged(integrate(&LooseConstant, unit_interval(), 1, 64, 8));
    assert_eq!(unmet.evidence.pieces(), 64);
    assert!(unmet.achieved.width().unwrap() >= unit(4));
}

/// The fourth derivative of x^2 is exactly 0, so every remainder term is [0, 0], but the value
/// enclosure keeps the total width at 2^-19 however the interval is split.
#[test]
fn success_needs_the_total_width_when_the_remainder_terms_vanish() {
    let unmet = not_converged(integrate(&LooseSquare, unit_interval(), 2, 4, 64));
    assert_eq!(unmet.evidence.pieces(), 4);
    assert!(unmet.evidence.remainder_width().is_zero());
    let width = unmet.achieved.width().unwrap();
    assert!(width >= unit(19));
    assert!(width <= unit(19).add(&unit(60)).unwrap());
    assert_contains_ratio(&unmet.achieved, 1, &integer(3));
}

/// The first split gives the left half the only remainder, one 192nd, and the right half the
/// widest enclosure, one quarter, from its value. Splitting the right half keeps the remainder
/// sum at one 192nd, whereas splitting the left half would leave one 768th.
#[test]
fn the_widest_enclosure_is_split_even_when_another_piece_has_the_wider_remainder() {
    let unmet = not_converged(integrate(&LooseHalves, unit_interval(), 1, 3, 64));
    assert_eq!(unmet.evidence.pieces(), 3);
    assert_within_ratio(unmet.evidence.remainder_width(), 1, &integer(192), 60);
    assert_eq!(unmet.achieved.lower(), &dyadic(7, 3));
    assert_within_ratio(&unmet.achieved.width().unwrap(), 49, &integer(192), 60);
    assert_contains_ratio(&unmet.achieved, 1, &integer(1));
}

#[test]
fn not_converged_failure_keeps_the_input_snapshot() {
    let cases = [
        ((0, 0, 1, 0), 1, 64),
        ((1, 2, 7, 2), 2, 5),
        ((-1, 0, 1, 1), 0, 3),
    ];
    for ((lower, lower_exponent, upper, upper_exponent), pairs, maximum_pieces) in cases {
        let bounds = || (dyadic(lower, lower_exponent), dyadic(upper, upper_exponent));
        let unmet = not_converged(integrate(
            &LooseConstant,
            bounds(),
            pairs,
            maximum_pieces,
            8,
        ));
        assert_eq!(unmet.input, snapshot(bounds(), pairs, maximum_pieces));
    }
}

/// Working bits are the requested bits + 8 + ceil(log2(pieces)), capped at 256. The rounding
/// of 1/3! widens the exact square by 2 * 2h^3 * 2^-working, which is 2^-(working + 1).
#[test]
fn working_precision_follows_request_piece_budget_and_limit() {
    let cases = [
        (64, 1, 72),
        (64, 3, 74),
        (64, 16, 76),
        (64, 17, 77),
        (64, 4096, 84),
        (64, 65_536, 88),
        (250, 1, 256),
        (250, 16, 256),
        (256, 1, 256),
    ];
    for (precision, maximum_pieces, working) in cases {
        let integrand = exact_power(dyadic(0, 0), 2);
        let recording = Recording::new(&integrand);
        let result = integrate(&recording, unit_interval(), 2, maximum_pieces, precision);
        let evaluation = result.unwrap();
        let calls = recording.calls.borrow();
        assert_eq!(calls.len(), 3);
        assert!(calls.iter().all(|call| call.bits == working));
        assert_eq!(evaluation.evidence().pieces(), 1);
        assert_eq!(evaluation.enclosure().width().unwrap(), unit(working + 1));
        assert_contains_ratio(evaluation.enclosure(), 1, &integer(3));
    }
}

/// 250 + 8 exceeds 256, so the working bits are capped where the polynomial still accepts them.
#[test]
fn a_request_at_the_edge_of_the_precision_limit_reaches_a_polynomial() {
    let constant = Polynomial::constant(algebraic(1)).unwrap();
    let evaluation = integrate(&constant, unit_interval(), 0, 1, 250).unwrap();
    assert_contains_ratio(evaluation.enclosure(), 1, &integer(1));
    assert!(evaluation.enclosure().width().unwrap() <= unit(250));
}

/// The integral of x^4 over [0, 1] is the sum of three rounded terms and equals one fifth.
/// Centered at one half, the remainder term alone survives: three halves of the rounded 1/120.
#[test]
fn outward_rounding_keeps_quartic_integrals_inside() {
    let integrand = exact_power(dyadic(0, 0), 4);
    let evaluation = integrate(&integrand, unit_interval(), 2, 1, 64).unwrap();
    assert_contains_ratio(evaluation.enclosure(), 1, &integer(5));
    let remainder = evaluation.evidence().remainder_width();
    assert!(remainder > &dyadic(0, 0));
    assert!(remainder < &evaluation.enclosure().width().unwrap());

    let integrand = exact_power(dyadic(1, 1), 4);
    let evaluation = integrate(&integrand, unit_interval(), 2, 1, 64).unwrap();
    assert_contains_ratio(evaluation.enclosure(), 1, &integer(80));
    let width = evaluation.enclosure().width().unwrap();
    assert_eq!(evaluation.evidence().remainder_width(), &width);
}

/// (x - 1/2)^(2n) has one nonzero center term, (2n)! * 2^-2n * [floor, ceiling] of 1/(2n+1)!,
/// and the integral over [0, 1] is 1 / ((2n + 1) * 4^n).
#[test]
fn outward_rounding_holds_at_every_even_order_up_to_the_limit() {
    for pairs in 2..=31_u32 {
        let degree = 2 * pairs;
        let integrand = exact_power(dyadic(1, 1), degree);
        let enclosure = reached(integrate(&integrand, unit_interval(), pairs + 1, 1, 64));
        let denominator = integer(i64::from(degree) + 1)
            .mul(&power_of_two(degree))
            .unwrap();
        assert_contains_ratio(&enclosure, 1, &denominator);
    }
}

/// (x - 1/2)^64 with 32 pairs keeps only the remainder term, the 64th derivative over [0, 1].
#[test]
fn maximum_order_requests_every_even_derivative_once() {
    let integrand = exact_power(dyadic(1, 1), 64);
    let recording = Recording::new(&integrand);
    let enclosure = reached(integrate(&recording, unit_interval(), 32, 1, 64));
    let denominator = integer(65).mul(&power_of_two(64)).unwrap();
    assert_contains_ratio(&enclosure, 1, &denominator);
    let orders: Vec<usize> = (1..=32).map(|pair| 2 * pair).collect();
    assert_eq!(recording.derivative_orders(), orders);
    assert_eq!(recording.calls.borrow().len(), 33);
}
