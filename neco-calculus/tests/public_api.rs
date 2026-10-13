use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational, ReducedRational};
use neco_calculus::{
    evaluate_surface, evaluate_surface_checked, CalculusFailure, CoordinateEnclosure3,
    DomainWitness, Enclosure, EvaluationFailure, EvaluationOperation, OwnerId, RationalFailure,
    RationalSurfacePolynomials, SurfaceEvaluationInput, SurfaceEvaluationInput2,
    SurfaceEvaluationProducer, SurfacePolynomial, SurfaceTerm,
};
use neco_expr::{
    AbsoluteBits, ConsumerId, ExactDecisionRequest, ExactExpressionRequirement, ExprId,
    NecoImplementationSource,
};

fn rational(value: i32) -> ReducedRational {
    RawRational::new(BigInt::try_from(value).unwrap(), BigUint::one().unwrap())
        .reduce()
        .unwrap()
        .into_reduced()
}

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn polynomial(coefficient: i32, exponent_u: u32, exponent_v: u32) -> SurfacePolynomial {
    SurfacePolynomial::try_from_terms(vec![SurfaceTerm {
        coefficient: rational(coefficient),
        exponent_u,
        exponent_v,
    }])
    .unwrap()
}

fn interval(lower: i32, upper: i32) -> Enclosure {
    Enclosure::new(
        Dyadic::new(BigInt::try_from(lower).unwrap(), 0),
        Dyadic::new(BigInt::try_from(upper).unwrap(), 0),
    )
    .unwrap()
}

fn requirement(expression: u32) -> ExactExpressionRequirement {
    ExactExpressionRequirement::new(
        ConsumerId::new(11),
        &[ExprId::new(expression)],
        ExactDecisionRequest::Zero(ExprId::new(expression)),
        AbsoluteBits::new(8),
    )
    .unwrap()
}

fn evaluation_failure<T: core::fmt::Debug>(
    result: Result<T, CalculusFailure>,
) -> EvaluationFailure {
    match result {
        Err(CalculusFailure::Evaluation(failure)) => *failure,
        other => panic!("expected an evaluation failure: {other:?}"),
    }
}

#[test]
fn bivariate_surface_api_is_crate_root_reachable() {
    let exact = evaluate_surface(SurfaceEvaluationInput {
        x: polynomial(1, 1, 0),
        y: polynomial(1, 0, 1),
        z: polynomial(1, 0, 0),
        denominator: polynomial(1, 0, 0),
        u: algebraic(2),
        v: algebraic(3),
    })
    .unwrap();
    assert_eq!(
        exact,
        SurfaceEvaluationProducer::Exact {
            x: algebraic(2),
            y: algebraic(3),
            z: algebraic(1),
        }
    );

    let checked_input = SurfaceEvaluationInput2::new(
        RationalSurfacePolynomials {
            x: polynomial(1, 1, 0),
            y: polynomial(1, 0, 1),
            z: polynomial(1, 0, 0),
            denominator: polynomial(1, 0, 0),
        },
        interval(1, 2),
        interval(2, 3),
        OwnerId::new(1),
        requirement(500),
        NecoImplementationSource::current(),
    )
    .unwrap();
    let coordinates: CoordinateEnclosure3 = match evaluate_surface_checked(checked_input).unwrap() {
        SurfaceEvaluationProducer::Certified { coordinates } => *coordinates,
        SurfaceEvaluationProducer::Exact { .. } => {
            panic!("checked evaluation unexpectedly produced exact values")
        }
    };
    assert!(coordinates
        .x()
        .contains(&Dyadic::new(BigInt::try_from(1).unwrap(), 0)));
    assert!(coordinates
        .y()
        .contains(&Dyadic::new(BigInt::try_from(2).unwrap(), 0)));
    assert_eq!(coordinates.owner(), OwnerId::new(1));
    assert_eq!(coordinates.requirement(), &requirement(500));
    assert_eq!(coordinates.source(), NecoImplementationSource::current());

    let zero_denominator = SurfacePolynomial::try_from_terms(Vec::new()).unwrap();
    let failure = evaluate_surface_checked(
        SurfaceEvaluationInput2::new(
            RationalSurfacePolynomials {
                x: polynomial(1, 0, 0),
                y: polynomial(1, 0, 0),
                z: polynomial(1, 0, 0),
                denominator: zero_denominator,
            },
            interval(1, 2),
            interval(2, 3),
            OwnerId::new(1),
            requirement(501),
            NecoImplementationSource::current(),
        )
        .unwrap(),
    );
    assert!(matches!(
        evaluation_failure(failure),
        EvaluationFailure::DomainViolation {
            operation: EvaluationOperation::SurfaceChecked,
            witness: DomainWitness::ZeroDenominatorPolynomial,
            lower: RationalFailure::ZeroDenominatorPolynomial,
            ..
        }
    ));
}
