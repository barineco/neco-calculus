use neco_algnum::{RationalPolynomial, RealAlgebraic};
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};
use neco_calculus::{
    construct_transcendental_evaluation, evaluate_implicit, evaluate_rational_polynomial,
    evaluate_surface, jet_at, CalculusFailure, DerivativeOrder, Enclosure, EvaluationFailure,
    ImplicitEvaluationInput, ImplicitEvaluationProducer, ImplicitPolynomial3, ImplicitTerm, Jet,
    JetFailure, JetSource, MethodDescriptor, Polynomial, PolynomialResourceFailure,
    RationalPolynomialInput, SurfaceEvaluationInput, SurfaceEvaluationProducer, SurfacePolynomial,
    SurfaceTerm, TranscendentalConnectorFailure, TranscendentalEvaluationCandidate,
    TranscendentalEvaluationConstructionInput, TranscendentalEvaluationKind,
    TranscendentalEvaluationRequest, MAX_COEFFICIENTS, MAX_IMPLICIT_TERMS, MAX_JET_ORDER,
    MAX_PRECISION_BITS, MAX_SURFACE_TERMS,
};
use neco_expr::ExprId;

fn rational(value: i32) -> neco_bigint::ReducedRational {
    RawRational::new(BigInt::try_from(value).unwrap(), BigUint::one().unwrap())
        .reduce()
        .unwrap()
        .into_reduced()
}

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn dyadic(value: i32, exponent: u32) -> Dyadic {
    Dyadic::new(BigInt::try_from(value).unwrap(), exponent)
}

fn surface_term(coefficient: i32, exponent_u: u32, exponent_v: u32) -> SurfaceTerm {
    SurfaceTerm {
        coefficient: rational(coefficient),
        exponent_u,
        exponent_v,
    }
}

fn implicit_term(coefficient: i32, exponent_x: u32) -> ImplicitTerm {
    ImplicitTerm {
        coefficient: algebraic(coefficient),
        exponent_x,
        exponent_y: 0,
        exponent_z: 0,
    }
}

#[test]
fn term_order_is_canonical_and_evaluation_is_deterministic() {
    let first =
        SurfacePolynomial::try_from_terms(vec![surface_term(2, 1, 0), surface_term(3, 0, 1)])
            .unwrap();
    let second =
        SurfacePolynomial::try_from_terms(vec![surface_term(3, 0, 1), surface_term(2, 1, 0)])
            .unwrap();
    assert_eq!(first.terms(), second.terms());

    let constant = || SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap();
    let evaluate = |x| {
        evaluate_surface(SurfaceEvaluationInput {
            x,
            y: constant(),
            z: constant(),
            denominator: constant(),
            u: algebraic(2),
            v: algebraic(3),
        })
        .unwrap()
    };
    assert_eq!(evaluate(first), evaluate(second));
}

#[test]
fn term_sets_reject_zero_duplicate_and_excess_inputs() {
    assert!(SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).is_ok());
    assert!(ImplicitPolynomial3::try_from_terms(vec![implicit_term(1, 0)]).is_ok());
    assert_eq!(
        SurfacePolynomial::try_from_terms(vec![surface_term(0, 0, 0)]),
        Err(CalculusFailure::Evaluation(
            EvaluationFailure::ZeroSurfaceTerm
        ))
    );
    assert_eq!(
        SurfacePolynomial::try_from_terms(vec![surface_term(1, 2, 3), surface_term(2, 2, 3),]),
        Err(CalculusFailure::Evaluation(
            EvaluationFailure::DuplicateSurfaceExponent
        ))
    );
    assert_eq!(
        ImplicitPolynomial3::try_from_terms(vec![implicit_term(0, 0)]),
        Err(CalculusFailure::Evaluation(
            EvaluationFailure::ZeroImplicitTerm
        ))
    );
    assert_eq!(
        ImplicitPolynomial3::try_from_terms(vec![implicit_term(1, 2), implicit_term(2, 2)]),
        Err(CalculusFailure::Evaluation(
            EvaluationFailure::DuplicateImplicitExponent
        ))
    );
    let surface_middle = (0..16).map(|index| surface_term(1, index, 0)).collect();
    assert!(SurfacePolynomial::try_from_terms(surface_middle).is_ok());
    let implicit_middle = (0..16).map(|index| implicit_term(1, index)).collect();
    assert!(ImplicitPolynomial3::try_from_terms(implicit_middle).is_ok());

    let surface_near_limit = (0..MAX_SURFACE_TERMS - 1)
        .map(|index| surface_term(1, index as u32, 0))
        .collect();
    assert!(SurfacePolynomial::try_from_terms(surface_near_limit).is_ok());
    let surface_at_limit = (0..MAX_SURFACE_TERMS)
        .map(|index| surface_term(1, index as u32, 0))
        .collect();
    assert!(SurfacePolynomial::try_from_terms(surface_at_limit).is_ok());
    let surface_terms = (0..=MAX_SURFACE_TERMS)
        .map(|index| surface_term(1, index as u32, 0))
        .collect();
    assert_eq!(
        SurfacePolynomial::try_from_terms(surface_terms),
        Err(CalculusFailure::Evaluation(
            EvaluationFailure::SurfaceTermLimitExceeded {
                requested: MAX_SURFACE_TERMS + 1,
                maximum: MAX_SURFACE_TERMS,
            }
        ))
    );

    let implicit_near_limit = (0..MAX_IMPLICIT_TERMS - 1)
        .map(|index| implicit_term(1, index as u32))
        .collect();
    assert!(ImplicitPolynomial3::try_from_terms(implicit_near_limit).is_ok());
    let implicit_at_limit = (0..MAX_IMPLICIT_TERMS)
        .map(|index| implicit_term(1, index as u32))
        .collect();
    assert!(ImplicitPolynomial3::try_from_terms(implicit_at_limit).is_ok());
    let implicit_terms = (0..=MAX_IMPLICIT_TERMS)
        .map(|index| implicit_term(1, index as u32))
        .collect();
    assert_eq!(
        ImplicitPolynomial3::try_from_terms(implicit_terms),
        Err(CalculusFailure::Evaluation(
            EvaluationFailure::ImplicitTermLimitExceeded {
                requested: MAX_IMPLICIT_TERMS + 1,
                maximum: MAX_IMPLICIT_TERMS,
            }
        ))
    );
}

#[test]
fn exact_surface_and_implicit_evaluation_preserve_values() {
    let surface = evaluate_surface(SurfaceEvaluationInput {
        x: SurfacePolynomial::try_from_terms(vec![surface_term(2, 1, 0)]).unwrap(),
        y: SurfacePolynomial::try_from_terms(vec![surface_term(3, 0, 1)]).unwrap(),
        z: SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
        denominator: SurfacePolynomial::try_from_terms(vec![surface_term(1, 0, 0)]).unwrap(),
        u: algebraic(2),
        v: algebraic(3),
    })
    .unwrap();
    match surface {
        SurfaceEvaluationProducer::Exact { x, y, z } => {
            assert_eq!(x, algebraic(4));
            assert_eq!(y, algebraic(9));
            assert_eq!(z, algebraic(1));
        }
    }

    let implicit = evaluate_implicit(ImplicitEvaluationInput {
        equation: ImplicitPolynomial3::try_from_terms(vec![implicit_term(2, 1)]).unwrap(),
        x: algebraic(3),
        y: algebraic(1),
        z: algebraic(1),
    })
    .unwrap();
    assert_eq!(
        implicit,
        ImplicitEvaluationProducer::Exact {
            scalar: algebraic(6)
        }
    );
}

#[test]
fn rational_polynomial_evaluation_preserves_exact_value() {
    let polynomial = RationalPolynomial::from_coefficients(vec![rational(1), rational(2)]);
    let result = evaluate_rational_polynomial(RationalPolynomialInput {
        polynomial,
        point: algebraic(3),
    })
    .unwrap();
    assert_eq!(result.value(), &algebraic(7));
}

#[test]
fn facade_resource_limits_reject_before_conversion_or_derivation() {
    for coefficient_count in [1, 16, MAX_COEFFICIENTS - 1, MAX_COEFFICIENTS] {
        let polynomial = RationalPolynomial::from_coefficients(
            (0..coefficient_count).map(|_| rational(1)).collect(),
        );
        assert!(evaluate_rational_polynomial(RationalPolynomialInput {
            polynomial,
            point: algebraic(1),
        })
        .is_ok());
    }
    let coefficients = (0..=neco_calculus::MAX_COEFFICIENTS)
        .map(|_| rational(1))
        .collect();
    let polynomial = RationalPolynomial::from_coefficients(coefficients);
    assert_eq!(
        evaluate_rational_polynomial(RationalPolynomialInput {
            polynomial,
            point: algebraic(1),
        }),
        Err(CalculusFailure::Polynomial(
            neco_calculus::PolynomialFailure::Resource(
                PolynomialResourceFailure::CoefficientLimitExceeded {
                    requested: neco_calculus::MAX_COEFFICIENTS + 1,
                    maximum: neco_calculus::MAX_COEFFICIENTS,
                }
            )
        ))
    );

    let polynomial = Polynomial::constant(algebraic(1)).unwrap();
    for order in [1, 8, MAX_JET_ORDER - 1, MAX_JET_ORDER] {
        assert!(jet_at(&polynomial, &algebraic(0), DerivativeOrder::new(order),).is_ok());
    }
    assert_eq!(
        jet_at(
            &polynomial,
            &algebraic(0),
            DerivativeOrder::new(neco_calculus::MAX_JET_ORDER + 1),
        ),
        Err(CalculusFailure::Jet(JetFailure::OrderLimitExceeded {
            requested: neco_calculus::MAX_JET_ORDER + 1,
            maximum: neco_calculus::MAX_JET_ORDER,
        }))
    );

    struct MustNotEvaluate;
    impl JetSource for MustNotEvaluate {
        fn jet_at(
            &self,
            _point: &RealAlgebraic,
            _order: DerivativeOrder,
        ) -> Result<Jet, CalculusFailure> {
            panic!("resource validation must precede source evaluation")
        }
    }
    assert_eq!(
        jet_at(
            &MustNotEvaluate,
            &algebraic(0),
            DerivativeOrder::new(neco_calculus::MAX_JET_ORDER + 1),
        ),
        Err(CalculusFailure::Jet(JetFailure::OrderLimitExceeded {
            requested: neco_calculus::MAX_JET_ORDER + 1,
            maximum: neco_calculus::MAX_JET_ORDER,
        }))
    );
}

#[test]
fn transcendental_connector_checks_expression_precision_and_width() {
    for precision_bits in [1, 64, MAX_PRECISION_BITS - 1, MAX_PRECISION_BITS] {
        assert!(construct_transcendental_evaluation(
            TranscendentalEvaluationConstructionInput::new(
                TranscendentalEvaluationCandidate::ExactAlgebraic {
                    value: algebraic(1),
                    source_expression: ExprId::new(7),
                },
                TranscendentalEvaluationRequest::new(ExprId::new(7), precision_bits),
            )
        )
        .is_ok());
    }
    let request = TranscendentalEvaluationRequest::new(ExprId::new(7), 8);
    let narrow = Enclosure::new(dyadic(0, 8), dyadic(1, 8)).unwrap();
    let candidate = TranscendentalEvaluationCandidate::Enclosure {
        enclosure: narrow,
        method_descriptor: MethodDescriptor::new(3),
        source_expression: ExprId::new(7),
    };
    let result = construct_transcendental_evaluation(
        TranscendentalEvaluationConstructionInput::new(candidate, request),
    )
    .unwrap();
    assert_eq!(result.kind(), TranscendentalEvaluationKind::Enclosure);
    let (enclosure, provenance) = result.as_enclosure().unwrap();
    assert_eq!(enclosure.width().unwrap(), dyadic(1, 8));
    assert_eq!(provenance.method_descriptor(), MethodDescriptor::new(3));
    assert_eq!(provenance.source_expression(), ExprId::new(7));

    let wide = Enclosure::new(dyadic(0, 0), dyadic(1, 0)).unwrap();
    let candidate = TranscendentalEvaluationCandidate::Enclosure {
        enclosure: wide,
        method_descriptor: MethodDescriptor::new(3),
        source_expression: ExprId::new(7),
    };
    assert_eq!(
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            candidate, request
        )),
        Err(CalculusFailure::Transcendental(
            TranscendentalConnectorFailure::EnclosureTooWide {
                requested_precision_bits: 8
            }
        ))
    );

    let excessive = TranscendentalEvaluationRequest::new(ExprId::new(7), MAX_PRECISION_BITS + 1);
    assert_eq!(
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            TranscendentalEvaluationCandidate::Undetermined {
                source_expression: ExprId::new(7)
            },
            excessive
        )),
        Err(CalculusFailure::Transcendental(
            TranscendentalConnectorFailure::PrecisionLimitExceeded {
                requested: MAX_PRECISION_BITS + 1,
                maximum: MAX_PRECISION_BITS,
            }
        ))
    );

    let zero_precision = TranscendentalEvaluationRequest::new(ExprId::new(7), 0);
    assert_eq!(
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            TranscendentalEvaluationCandidate::Undetermined {
                source_expression: ExprId::new(7)
            },
            zero_precision
        )),
        Err(CalculusFailure::Transcendental(
            TranscendentalConnectorFailure::PrecisionMustBePositive
        ))
    );

    let maximum = TranscendentalEvaluationRequest::new(ExprId::new(7), MAX_PRECISION_BITS);
    assert!(
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            TranscendentalEvaluationCandidate::ExactAlgebraic {
                value: algebraic(1),
                source_expression: ExprId::new(7),
            },
            maximum,
        ))
        .is_ok()
    );

    let mismatch = TranscendentalEvaluationCandidate::Undetermined {
        source_expression: ExprId::new(8),
    };
    assert_eq!(
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            mismatch, request
        )),
        Err(CalculusFailure::Transcendental(
            TranscendentalConnectorFailure::CandidateExpressionMismatch {
                candidate_expression: ExprId::new(8),
                requested: ExprId::new(7),
            }
        ))
    );

    let exact_mismatch = TranscendentalEvaluationCandidate::ExactAlgebraic {
        value: algebraic(1),
        source_expression: ExprId::new(8),
    };
    assert_eq!(
        construct_transcendental_evaluation(TranscendentalEvaluationConstructionInput::new(
            exact_mismatch,
            request
        )),
        Err(CalculusFailure::Transcendental(
            TranscendentalConnectorFailure::CandidateExpressionMismatch {
                candidate_expression: ExprId::new(8),
                requested: ExprId::new(7),
            }
        ))
    );
}
