use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, Dyadic};
use neco_polynomial::{
    DerivativeOrder, Enclosure, Polynomial, PolynomialFailure, ResourceFailure, MAX_COEFFICIENTS,
    MAX_DERIVATIVE_ORDER, MAX_PRECISION_BITS,
};

fn algebraic(value: usize) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn dyadic(value: i64) -> Dyadic {
    Dyadic::new(BigInt::try_from(value).unwrap(), 0)
}

fn assert_algebraic_integer(value: &RealAlgebraic, expected: usize) {
    assert_eq!(value, &algebraic(expected));
}

fn repeated_coefficients(count: usize) -> Vec<RealAlgebraic> {
    let one = algebraic(1);
    (0..count).map(|_| one.try_clone().unwrap()).collect()
}

#[test]
fn rejects_empty_coefficients_and_normalizes_zero() {
    assert_eq!(
        Polynomial::from_coefficients(Vec::new()).unwrap_err(),
        PolynomialFailure::EmptyCoefficients
    );
    let zero = Polynomial::from_coefficients(vec![algebraic(0), algebraic(0)]).unwrap();
    assert_eq!(zero.degree(), 0);
    assert_eq!(zero.coefficients().len(), 1);
}

#[test]
fn evaluates_constants_x_and_x_squared() {
    let point = algebraic(2);
    let constant = Polynomial::constant(algebraic(1)).unwrap();
    assert_algebraic_integer(&constant.evaluate(&point).unwrap(), 1);

    let x = Polynomial::from_coefficients(vec![algebraic(0), algebraic(1)]).unwrap();
    assert_algebraic_integer(&x.evaluate(&point).unwrap(), 2);

    let x_squared =
        Polynomial::from_coefficients(vec![algebraic(0), algebraic(0), algebraic(1)]).unwrap();
    assert_algebraic_integer(&x_squared.evaluate(&point).unwrap(), 4);
}

#[test]
fn differentiates_polynomials() {
    let x_squared =
        Polynomial::from_coefficients(vec![algebraic(0), algebraic(0), algebraic(1)]).unwrap();
    let unchanged = x_squared.derivative(DerivativeOrder::new(0)).unwrap();
    assert_eq!(unchanged.degree(), 2);

    let first = x_squared.derivative(DerivativeOrder::new(1)).unwrap();
    assert_eq!(first.degree(), 1);
    assert_algebraic_integer(&first.evaluate(&algebraic(2)).unwrap(), 4);

    let second = x_squared.derivative(DerivativeOrder::new(2)).unwrap();
    assert_eq!(second.degree(), 0);
    assert_algebraic_integer(&second.evaluate(&algebraic(2)).unwrap(), 2);
}

#[test]
fn constructs_and_uses_enclosures() {
    let enclosure = Enclosure::new(dyadic(1), dyadic(2)).unwrap();
    assert!(enclosure.contains(&dyadic(1)));
    assert!(enclosure.contains(&dyadic(2)));
    assert!(!enclosure.contains_zero());
    assert_eq!(enclosure.width().unwrap(), dyadic(1));
    assert_eq!(
        enclosure.midpoint().unwrap(),
        Dyadic::new(BigInt::try_from(3_i64).unwrap(), 1)
    );
}

#[test]
fn evaluates_enclosure_with_containment() {
    let x_squared =
        Polynomial::from_coefficients(vec![algebraic(0), algebraic(0), algebraic(1)]).unwrap();
    let domain = Enclosure::new(dyadic(1), dyadic(2)).unwrap();
    let image = x_squared.evaluate_enclosure(&domain, 0).unwrap();
    assert!(image.contains(&dyadic(1)));
    assert!(image.contains(&dyadic(4)));
}

#[test]
fn rejects_resource_inputs_before_evaluation() {
    assert!(Polynomial::from_coefficients(repeated_coefficients(1)).is_ok());
    assert!(Polynomial::from_coefficients(repeated_coefficients(16)).is_ok());
    let near_limit = repeated_coefficients(MAX_COEFFICIENTS - 1);
    assert!(Polynomial::from_coefficients(near_limit).is_ok());
    let at_limit = repeated_coefficients(MAX_COEFFICIENTS);
    assert!(Polynomial::from_coefficients(at_limit).is_ok());
    let coefficients = repeated_coefficients(MAX_COEFFICIENTS + 1);
    assert_eq!(
        Polynomial::from_coefficients(coefficients),
        Err(PolynomialFailure::Resource(
            ResourceFailure::CoefficientLimitExceeded {
                requested: MAX_COEFFICIENTS + 1,
                maximum: MAX_COEFFICIENTS,
            }
        ))
    );

    let polynomial = Polynomial::constant(algebraic(1)).unwrap();
    assert!(polynomial.derivative(DerivativeOrder::new(1)).is_ok());
    assert!(polynomial.derivative(DerivativeOrder::new(8)).is_ok());
    assert!(polynomial
        .derivative(DerivativeOrder::new(MAX_DERIVATIVE_ORDER - 1))
        .is_ok());
    assert!(polynomial
        .derivative(DerivativeOrder::new(MAX_DERIVATIVE_ORDER))
        .is_ok());
    assert_eq!(
        polynomial.derivative(DerivativeOrder::new(MAX_DERIVATIVE_ORDER + 1)),
        Err(PolynomialFailure::Resource(
            ResourceFailure::DerivativeOrderLimitExceeded {
                requested: MAX_DERIVATIVE_ORDER + 1,
                maximum: MAX_DERIVATIVE_ORDER,
            }
        ))
    );
    let domain = Enclosure::new(dyadic(0), dyadic(1)).unwrap();
    assert!(polynomial.evaluate_enclosure(&domain, 1).is_ok());
    assert!(polynomial.evaluate_enclosure(&domain, 64).is_ok());
    assert!(polynomial
        .evaluate_enclosure(&domain, MAX_PRECISION_BITS - 1)
        .is_ok());
    assert!(polynomial
        .evaluate_enclosure(&domain, MAX_PRECISION_BITS)
        .is_ok());
    assert_eq!(
        polynomial.evaluate_enclosure(&domain, MAX_PRECISION_BITS + 1),
        Err(PolynomialFailure::Resource(
            ResourceFailure::PrecisionLimitExceeded {
                requested: MAX_PRECISION_BITS + 1,
                maximum: MAX_PRECISION_BITS,
            }
        ))
    );
}
