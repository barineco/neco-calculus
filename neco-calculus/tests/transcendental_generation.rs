use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, Dyadic, Sign};
use neco_calculus::{
    evaluate_transcendental_function, evaluate_transcendental_power, AbsoluteBits, CalculusFailure,
    ConsumerId, DomainWitness, Enclosure, EvaluationFailure, EvaluationInputSnapshot,
    EvaluationOperation, ExactDecisionRequest, ExactExpressionRequirement, ExprId,
    LowerEvaluationFailure, NecoImplementationSource, OwnerId, PolynomialFailure,
    PolynomialResourceFailure, PrecisionShortageCause, PrecisionWitness,
    TranscendentalCertificationFailure, TranscendentalEvaluation, TranscendentalEvaluationKind,
    TranscendentalEvaluationRequest, TranscendentalFunction, TranscendentalFunctionEvaluationInput,
    TranscendentalPowerEvaluationInput, MAX_PRECISION_BITS,
};

const PI_OVER_SIX_224: &str = "860a91c16b9b2c232dd99707ab3d688b70ac3405b19a884d56b27f19";
const PI_OVER_THREE_224: &str = "10c152382d73658465bb32e0f567ad116e158680b6335109aad64fe32";
const LN_TWO_224: &str = "b17217f7d1cf79abc9e3b39803f2f6af40f343267298b62d8a0d175b";
const SEMITONE_SEVEN_224: &str = "17f910d768cfafe6d8be5b92eaf1c8d80c8fedd99ef807abc6149634a";
const SQRT_TWO_400: &str = "16a09e667f3bcc908b2fb1366ea957d3e3adec17512775099da2f590b0667322a95f90608757145875163fcdfb907b6721ee9";
const PI_OVER_SIX_400: &str = "860a91c16b9b2c232dd99707ab3d688b70ac3405b19a884d56b27f197cb7bcc18b86b0510978033e9fb8bbcd337c2cbccac7";

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

fn rational_floor_interval(numerator: i64, denominator: i64, bits: u32) -> Enclosure {
    let scaled = BigInt::from_sign_magnitude(
        Sign::Positive,
        BigUint::one().unwrap().shl_bits(bits as usize).unwrap(),
    )
    .mul(&integer(numerator))
    .unwrap();
    let (lower, remainder) = scaled.div_rem_euclid(&integer(denominator)).unwrap();
    let upper = if remainder.is_zero() {
        lower.try_clone().unwrap()
    } else {
        lower.add(&integer(1)).unwrap()
    };
    interval(Dyadic::new(lower, bits), Dyadic::new(upper, bits))
}

fn around(center: &Dyadic, half_width_exponent: u32) -> Enclosure {
    let half_width = unit(half_width_exponent);
    Enclosure::new(
        center.sub(&half_width).unwrap(),
        center.add(&half_width).unwrap(),
    )
    .unwrap()
}

fn interval(lower: Dyadic, upper: Dyadic) -> Enclosure {
    Enclosure::new(lower, upper).unwrap()
}

fn point(value: Dyadic) -> Enclosure {
    Enclosure::new(value.try_clone().unwrap(), value).unwrap()
}

fn requirement(precision: u32) -> ExactExpressionRequirement {
    ExactExpressionRequirement::new(
        ConsumerId::new(31),
        &[ExprId::new(7)],
        ExactDecisionRequest::Zero(ExprId::new(7)),
        AbsoluteBits::new(precision),
    )
    .unwrap()
}

fn request(precision: u32) -> TranscendentalEvaluationRequest {
    TranscendentalEvaluationRequest::new(
        OwnerId::new(2),
        requirement(precision),
        NecoImplementationSource::current(),
    )
}

fn function(
    function: TranscendentalFunction,
    argument: Enclosure,
    precision: u32,
) -> Result<TranscendentalEvaluation, CalculusFailure> {
    evaluate_transcendental_function(TranscendentalFunctionEvaluationInput {
        function,
        argument,
        request: request(precision),
    })
}

fn power(
    base: Enclosure,
    exponent: Enclosure,
    precision: u32,
) -> Result<TranscendentalEvaluation, CalculusFailure> {
    evaluate_transcendental_power(TranscendentalPowerEvaluationInput {
        base,
        exponent,
        request: request(precision),
    })
}

fn evaluation_failure<T: core::fmt::Debug>(
    result: Result<T, CalculusFailure>,
) -> EvaluationFailure {
    match result {
        Err(CalculusFailure::Evaluation(failure)) => *failure,
        other => panic!("expected an evaluation failure: {other:?}"),
    }
}

fn algebraic(value: i64) -> RealAlgebraic {
    RealAlgebraic::from_integer(integer(value)).unwrap()
}

fn assert_enclosure_within(evaluation: &TranscendentalEvaluation, precision: u32) -> &Enclosure {
    assert_eq!(evaluation.kind(), TranscendentalEvaluationKind::Enclosure);
    let enclosure = evaluation.as_enclosure().unwrap();
    assert!(enclosure.width().unwrap() <= unit(precision));
    enclosure
}

fn assert_lineage(evaluation: &TranscendentalEvaluation, precision: u32) {
    assert_eq!(evaluation.owner(), OwnerId::new(2));
    assert_eq!(evaluation.requirement(), &requirement(precision));
    assert_eq!(evaluation.source(), NecoImplementationSource::current());
}

fn sine_of_pi_over_six(precision: u32) -> TranscendentalEvaluation {
    let center = hex_dyadic(PI_OVER_SIX_224, 224);
    function(
        TranscendentalFunction::Sine,
        around(&center, 220),
        precision,
    )
    .unwrap()
}

#[test]
fn sine_of_zero_is_exact() {
    let evaluation = function(TranscendentalFunction::Sine, point(dyadic(0, 0)), 64).unwrap();
    assert_eq!(
        evaluation.kind(),
        TranscendentalEvaluationKind::ExactAlgebraic
    );
    assert_eq!(evaluation.as_exact_algebraic(), Some(&algebraic(0)));
    assert_lineage(&evaluation, 64);
}

#[test]
fn cosine_of_zero_is_exact() {
    let evaluation = function(TranscendentalFunction::Cosine, point(dyadic(0, 0)), 64).unwrap();
    assert_eq!(
        evaluation.kind(),
        TranscendentalEvaluationKind::ExactAlgebraic
    );
    assert_eq!(evaluation.as_exact_algebraic(), Some(&algebraic(1)));
}

#[test]
fn exponential_of_zero_is_exact() {
    let evaluation =
        function(TranscendentalFunction::Exponential, point(dyadic(0, 0)), 64).unwrap();
    assert_eq!(
        evaluation.kind(),
        TranscendentalEvaluationKind::ExactAlgebraic
    );
    assert_eq!(evaluation.as_exact_algebraic(), Some(&algebraic(1)));
}

#[test]
fn sine_of_pi_over_six_encloses_reference() {
    let evaluation = sine_of_pi_over_six(64);
    let enclosure = assert_enclosure_within(&evaluation, 64);
    assert!(enclosure.contains(&dyadic(1, 1)));
    assert_lineage(&evaluation, 64);
}

#[test]
fn cosine_of_pi_over_three_encloses_reference() {
    let center = hex_dyadic(PI_OVER_THREE_224, 224);
    let evaluation = function(TranscendentalFunction::Cosine, around(&center, 220), 64).unwrap();
    let enclosure = assert_enclosure_within(&evaluation, 64);
    assert!(enclosure.contains(&dyadic(1, 1)));
}

#[test]
fn exponential_of_ln2_encloses_reference() {
    let center = hex_dyadic(LN_TWO_224, 224);
    let evaluation = function(
        TranscendentalFunction::Exponential,
        around(&center, 220),
        64,
    )
    .unwrap();
    let enclosure = assert_enclosure_within(&evaluation, 64);
    assert!(enclosure.contains(&dyadic(2, 0)));
}

fn seven_twelfths_exponent() -> Enclosure {
    let mut numerator = integer(7);
    for _ in 0..224 {
        numerator = numerator.mul(&integer(2)).unwrap();
    }
    let (quotient, _) = numerator.div_rem_euclid(&integer(12)).unwrap();
    around(&Dyadic::new(quotient, 224), 220)
}

#[test]
fn power_of_semitone_ratio_encloses_reference() {
    let evaluation = power(point(dyadic(2, 0)), seven_twelfths_exponent(), 64).unwrap();
    let enclosure = assert_enclosure_within(&evaluation, 64);
    assert!(enclosure.contains(&hex_dyadic(SEMITONE_SEVEN_224, 224)));
    assert_lineage(&evaluation, 64);
    assert_eq!(
        evaluation.certify_f64().unwrap().value(),
        1.4983070768766815_f64
    );
}

#[test]
fn power_rejects_nonpositive_base() {
    let result = power(
        interval(dyadic(-2, 0), dyadic(-1, 0)),
        point(dyadic(1, 1)),
        64,
    );
    match evaluation_failure(result) {
        EvaluationFailure::TranscendentalDomainViolation {
            operation,
            owner,
            requirement: result_requirement,
            source,
            input,
            witness,
        } => {
            assert_eq!(operation, EvaluationOperation::TranscendentalPower);
            assert_eq!(owner, OwnerId::new(2));
            assert_eq!(result_requirement, requirement(64));
            assert_eq!(source, NecoImplementationSource::current());
            assert_eq!(
                input,
                EvaluationInputSnapshot::TranscendentalPower {
                    base: interval(dyadic(-2, 0), dyadic(-1, 0)),
                    exponent: point(dyadic(1, 1)),
                }
            );
            assert_eq!(
                witness,
                DomainWitness::NonPositiveBase {
                    base: interval(dyadic(-2, 0), dyadic(-1, 0)),
                    exponent: point(dyadic(1, 1)),
                }
            );
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn power_rejects_base_interval_reaching_zero() {
    for lower in [dyadic(0, 0), dyadic(-1, 4)] {
        let result = power(interval(lower, dyadic(1, 0)), point(dyadic(1, 1)), 64);
        assert!(matches!(
            evaluation_failure(result),
            EvaluationFailure::TranscendentalDomainViolation {
                witness: DomainWitness::NonPositiveBase { .. },
                ..
            }
        ));
    }
}

#[test]
fn power_of_base_one_is_exact() {
    let evaluation = power(
        point(dyadic(1, 0)),
        interval(dyadic(77, 8), dyadic(230, 8)),
        64,
    )
    .unwrap();
    assert_eq!(
        evaluation.kind(),
        TranscendentalEvaluationKind::ExactAlgebraic
    );
    assert_eq!(evaluation.as_exact_algebraic(), Some(&algebraic(1)));
}

#[test]
fn power_with_zero_exponent_is_exact() {
    let evaluation = power(
        interval(dyadic(1, 0), dyadic(3, 0)),
        point(dyadic(0, 0)),
        64,
    )
    .unwrap();
    assert_eq!(evaluation.as_exact_algebraic(), Some(&algebraic(1)));
}

#[test]
fn precision_shortage_is_structural() {
    for bits in [MAX_PRECISION_BITS + 1, 0] {
        let result = function(TranscendentalFunction::Sine, point(dyadic(0, 0)), bits);
        match evaluation_failure(result) {
            EvaluationFailure::PrecisionShortage {
                operation,
                requested,
                witness,
                lower,
                input,
                ..
            } => {
                assert_eq!(
                    operation,
                    EvaluationOperation::TranscendentalFunction(TranscendentalFunction::Sine)
                );
                assert_eq!(requested, AbsoluteBits::new(bits));
                assert_eq!(witness, PrecisionWitness::Limit { maximum: 256 });
                assert_eq!(
                    lower,
                    PrecisionShortageCause::RequestedBeyondLimit {
                        requested: AbsoluteBits::new(bits),
                        maximum: 256,
                    }
                );
                assert_eq!(
                    input,
                    EvaluationInputSnapshot::TranscendentalFunction {
                        function: TranscendentalFunction::Sine,
                        argument: point(dyadic(0, 0)),
                    }
                );
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
}

#[test]
fn wide_argument_is_undetermined_not_failure() {
    let center = hex_dyadic(PI_OVER_SIX_224, 224);
    let evaluation = function(TranscendentalFunction::Sine, around(&center, 9), 200).unwrap();
    assert!(evaluation.is_undetermined());
    assert_eq!(
        evaluation.kind(),
        TranscendentalEvaluationKind::Undetermined
    );
    assert!(evaluation.as_enclosure().is_none());
    let achieved = evaluation.undetermined_achieved().unwrap();
    assert!(achieved.width().unwrap() <= unit(8));
    assert!(achieved.contains(&dyadic(1, 1)));
    assert_lineage(&evaluation, 200);
}

#[test]
fn narrow_argument_same_target_succeeds() {
    let center = hex_dyadic(PI_OVER_SIX_224, 224);
    let evaluation = function(TranscendentalFunction::Sine, around(&center, 221), 200).unwrap();
    let enclosure = assert_enclosure_within(&evaluation, 200);
    assert!(enclosure.contains(&dyadic(1, 1)));
    assert!(evaluation.undetermined_achieved().is_none());
}

#[test]
fn certify_f64_reports_actual_absolute_error() {
    let evaluation = sine_of_pi_over_six(64);
    let certified = evaluation.certify_f64().unwrap();
    assert_eq!(certified.value(), 0.5_f64);

    let enclosure = evaluation.as_enclosure().unwrap();
    assert_eq!(certified.enclosure(), enclosure.as_dyadic_enclosure());
    let half = dyadic(1, 1);
    let below = half.sub(enclosure.lower()).unwrap();
    let above = enclosure.upper().sub(&half).unwrap();
    let farther = if below > above { below } else { above };
    assert_eq!(certified.absolute_error(), &farther);

    let doubled_error = certified.absolute_error().mul(&dyadic(2, 0)).unwrap();
    assert!(doubled_error >= enclosure.width().unwrap());
    assert_ne!(certified.absolute_error(), &unit(64));
    assert!(certified.absolute_error() < &unit(64));
}

#[test]
fn certify_f64_error_follows_f64_spacing_when_bits_exceed_mantissa() {
    let evaluation = function(
        TranscendentalFunction::Exponential,
        around(&dyadic(1, 3), 220),
        64,
    )
    .unwrap();
    let certified = evaluation.certify_f64().unwrap();
    assert_eq!(certified.value(), 1.1331484530668263_f64);
    assert!(certified.absolute_error() > &unit(64));
    assert!(certified.absolute_error() <= &unit(53));
}

#[test]
fn undetermined_certification_is_rejected() {
    let center = hex_dyadic(PI_OVER_SIX_224, 224);
    let evaluation = function(TranscendentalFunction::Sine, around(&center, 9), 200).unwrap();
    assert_eq!(
        evaluation.certify_f64().unwrap_err(),
        TranscendentalCertificationFailure::Undetermined
    );
}

#[test]
fn exact_result_certifies_without_error() {
    let evaluation = function(TranscendentalFunction::Cosine, point(dyadic(0, 0)), 64).unwrap();
    let certified = evaluation.certify_f64().unwrap();
    assert_eq!(certified.value(), 1.0_f64);
    assert!(certified.absolute_error() <= &unit(64));
}

#[test]
fn large_and_negative_arguments_reduce_to_the_correct_quadrant() {
    let sine = function(
        TranscendentalFunction::Sine,
        around(&dyadic(1000, 0), 100),
        64,
    )
    .unwrap();
    assert_eq!(sine.certify_f64().unwrap().value(), 0.8268795405320025_f64);

    let cosine = function(
        TranscendentalFunction::Cosine,
        around(&dyadic(-3, 0), 100),
        64,
    )
    .unwrap();
    assert_eq!(
        cosine.certify_f64().unwrap().value(),
        -0.9899924966004454_f64
    );
}

#[test]
fn exponential_handles_negative_and_large_arguments() {
    let negative = function(
        TranscendentalFunction::Exponential,
        around(&dyadic(-1, 0), 100),
        64,
    )
    .unwrap();
    assert_eq!(
        negative.certify_f64().unwrap().value(),
        0.36787944117144233_f64
    );

    let large = function(
        TranscendentalFunction::Exponential,
        around(&dyadic(10, 0), 100),
        64,
    )
    .unwrap();
    assert_enclosure_within(&large, 64);
    assert_eq!(large.certify_f64().unwrap().value(), 22026.465794806718_f64);
}

#[test]
fn sine_over_an_interval_containing_the_maximum_reaches_one() {
    let evaluation = function(
        TranscendentalFunction::Sine,
        interval(dyadic(1, 0), dyadic(2, 0)),
        32,
    )
    .unwrap();
    let achieved = evaluation.undetermined_achieved().unwrap();
    assert_eq!(achieved.upper(), &dyadic(1, 0));
    assert!(achieved.lower() <= &dyadic(216, 8));
    assert!(achieved.lower() > &dyadic(214, 8));
}

#[test]
fn maximum_precision_is_reachable() {
    let center = hex_dyadic(PI_OVER_SIX_400, 400);
    let evaluation = function(
        TranscendentalFunction::Sine,
        around(&center, 300),
        MAX_PRECISION_BITS,
    )
    .unwrap();
    let enclosure = assert_enclosure_within(&evaluation, MAX_PRECISION_BITS);
    assert!(enclosure.contains(&dyadic(1, 1)));

    let exponent = around(&dyadic(1, 1), 300);
    let evaluation = power(point(dyadic(2, 0)), exponent, MAX_PRECISION_BITS).unwrap();
    let enclosure = assert_enclosure_within(&evaluation, MAX_PRECISION_BITS);
    assert!(enclosure.contains(&hex_dyadic(SQRT_TWO_400, 400)));
}

const EXP_MINUS_25_LOWER_300: &str =
    "f451bd1f2cd279fd387710eeeccd960308d34b490b091425820dbd5398d49d2a60";
const EXP_MINUS_25_UPPER_300: &str =
    "f451bd22fe196e818e4ee1e7635035a9e14476ae4759b3bfc40bfcd83bcc82ac74";
const EXP_MINUS_40_LOWER_300: &str =
    "4e5e492532ef6c0e13d52771ca3c5bf8638810aaa10c93d513291afa2b224";
const EXP_MINUS_40_UPPER_300: &str =
    "4e5e49266c6890a5528520d6f4fa11158e5f9e4010a95cd5cb36fe9b6550f";
const EXP_MINUS_100_LOWER_300: &str = "d460f8a7157ae579eec89bae3a7e5ae6c05cbc4";
const EXP_MINUS_100_UPPER_300: &str = "d46105ed256f87502702ed4f1a464e3947663ac";
const TWO_TO_SEVEN_EIGHTHS_100: &str = "1d5818dcfba48725da05aeb66e";

fn exponential(argument: Enclosure, precision: u32) -> TranscendentalEvaluation {
    function(TranscendentalFunction::Exponential, argument, precision).unwrap()
}

fn assert_narrow_negative_exponential(
    upper: Dyadic,
    width_exponent: u32,
    lower_reference: &str,
    upper_reference: &str,
) {
    let lower = upper.sub(&unit(width_exponent)).unwrap();
    let evaluation = exponential(interval(lower, upper), 64);
    let enclosure = assert_enclosure_within(&evaluation, 64);
    assert!(enclosure.contains(&hex_dyadic(lower_reference, 300)));
    assert!(enclosure.contains(&hex_dyadic(upper_reference, 300)));
}

#[test]
fn exponential_of_narrow_negative_interval_is_determined() {
    assert_narrow_negative_exponential(
        dyadic(-25, 0),
        30,
        EXP_MINUS_25_LOWER_300,
        EXP_MINUS_25_UPPER_300,
    );
    assert_narrow_negative_exponential(
        dyadic(-40, 0),
        30,
        EXP_MINUS_40_LOWER_300,
        EXP_MINUS_40_UPPER_300,
    );
    let upper = dyadic(-100, 0).add(&unit(20)).unwrap();
    let evaluation = exponential(interval(dyadic(-100, 0), upper), 64);
    let enclosure = assert_enclosure_within(&evaluation, 64);
    assert!(enclosure.contains(&hex_dyadic(EXP_MINUS_100_LOWER_300, 300)));
    assert!(enclosure.contains(&hex_dyadic(EXP_MINUS_100_UPPER_300, 300)));
}

#[test]
fn exponential_of_large_argument_scales_working_precision() {
    let evaluation = exponential(point(dyadic(400, 0)), 64);
    assert_enclosure_within(&evaluation, 64);
    assert_eq!(
        evaluation.certify_f64().unwrap().value(),
        5.221469689764144e173_f64
    );
}

fn power_of_two_integer(exponent: u32) -> BigInt {
    let mut value = BigInt::one().unwrap();
    for _ in 0..exponent {
        value = value.mul(&integer(2)).unwrap();
    }
    value
}

#[test]
fn power_of_two_to_a_large_integer_exponent_encloses_the_integer() {
    let evaluation = power(point(dyadic(2, 0)), point(dyadic(400, 0)), 64).unwrap();
    let enclosure = assert_enclosure_within(&evaluation, 64);
    assert!(enclosure.contains(&Dyadic::new(power_of_two_integer(400), 0)));
}

fn assert_series_capacity_failure(
    result: Result<TranscendentalEvaluation, CalculusFailure>,
    expected_operation: EvaluationOperation,
    expected_input: EvaluationInputSnapshot,
) -> (u32, u32) {
    match evaluation_failure(result) {
        EvaluationFailure::Lower {
            operation,
            owner,
            requirement: result_requirement,
            source,
            input,
            witness,
            lower: LowerEvaluationFailure::Calculus(inner),
        } => {
            assert_eq!(operation, expected_operation);
            assert_eq!(witness.operation(), expected_operation);
            assert_eq!(owner, OwnerId::new(2));
            assert_eq!(result_requirement, requirement(64));
            assert_eq!(source, NecoImplementationSource::current());
            assert_eq!(input, expected_input);
            match *inner {
                CalculusFailure::Polynomial(PolynomialFailure::Resource(
                    PolynomialResourceFailure::PrecisionLimitExceeded { requested, maximum },
                )) => {
                    assert!(requested > maximum);
                    (requested, maximum)
                }
                other => panic!("unexpected lower failure: {other:?}"),
            }
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn exponential_beyond_series_capacity_is_a_wrapped_lower_failure() {
    let (requested, maximum) = assert_series_capacity_failure(
        function(
            TranscendentalFunction::Exponential,
            point(dyadic(2000, 0)),
            64,
        ),
        EvaluationOperation::TranscendentalFunction(TranscendentalFunction::Exponential),
        EvaluationInputSnapshot::TranscendentalFunction {
            function: TranscendentalFunction::Exponential,
            argument: point(dyadic(2000, 0)),
        },
    );
    assert!(requested >= 2885 + 64);
    assert_eq!(maximum, 1684 - 7);
}

#[test]
fn power_beyond_series_capacity_is_a_wrapped_lower_failure() {
    assert_series_capacity_failure(
        power(point(dyadic(2, 0)), point(dyadic(2000, 0)), 64),
        EvaluationOperation::TranscendentalPower,
        EvaluationInputSnapshot::TranscendentalPower {
            base: point(dyadic(2, 0)),
            exponent: point(dyadic(2000, 0)),
        },
    );
}

#[test]
fn every_quarter_turn_phase_matches_the_reference() {
    let cases: [(TranscendentalFunction, i64, u32, f64); 8] = [
        (TranscendentalFunction::Sine, 1, 1, 0.479425538604203),
        (TranscendentalFunction::Sine, 1, 0, 0.8414709848078965),
        (TranscendentalFunction::Sine, 3, 0, 0.1411200080598672),
        (TranscendentalFunction::Sine, -1, 0, -0.8414709848078965),
        (TranscendentalFunction::Cosine, 1, 1, 0.8775825618903728),
        (TranscendentalFunction::Cosine, 1, 0, 0.5403023058681398),
        (TranscendentalFunction::Cosine, 3, 0, -0.9899924966004454),
        (TranscendentalFunction::Cosine, -1, 0, 0.5403023058681398),
    ];
    for (kind, numerator, exponent, expected) in cases {
        let evaluation = function(kind, point(dyadic(numerator, exponent)), 64).unwrap();
        assert_enclosure_within(&evaluation, 64);
        assert_eq!(evaluation.certify_f64().unwrap().value(), expected);
    }
}

fn assert_bounds_near(value: &Dyadic, floor_units: i64) {
    assert!(value >= &dyadic(floor_units, 40));
    assert!(value <= &dyadic(floor_units + 2, 40));
}

fn achieved_of(kind: TranscendentalFunction, lower: i64, upper: i64) -> Enclosure {
    let argument = interval(dyadic(lower, 0), dyadic(upper, 0));
    let evaluation = function(kind, argument, 8).unwrap();
    assert!(evaluation.is_undetermined());
    let achieved = evaluation.undetermined_achieved().unwrap();
    interval(
        achieved.lower().try_clone().unwrap(),
        achieved.upper().try_clone().unwrap(),
    )
}

#[test]
fn intervals_across_an_extremum_reach_the_extreme_value() {
    let sine_minimum = achieved_of(TranscendentalFunction::Sine, 4, 5);
    assert_eq!(sine_minimum.lower(), &dyadic(-1, 0));
    assert_bounds_near(sine_minimum.upper(), -832113143521);

    let cosine_maximum = achieved_of(TranscendentalFunction::Cosine, -1, 1);
    assert_eq!(cosine_maximum.upper(), &dyadic(1, 0));
    assert_bounds_near(cosine_maximum.lower(), 594068667816);

    let negative_sine_minimum = achieved_of(TranscendentalFunction::Sine, -2, -1);
    assert_eq!(negative_sine_minimum.lower(), &dyadic(-1, 0));
    assert_bounds_near(negative_sine_minimum.upper(), -925207132233);

    let cosine_minimum = achieved_of(TranscendentalFunction::Cosine, 3, 4);
    assert_eq!(cosine_minimum.lower(), &dyadic(-1, 0));
    assert_bounds_near(cosine_minimum.upper(), -718688761562);

    let full_period = achieved_of(TranscendentalFunction::Sine, 0, 7);
    assert_eq!(full_period.lower(), &dyadic(-1, 0));
    assert_eq!(full_period.upper(), &dyadic(1, 0));
}

const REFERENCE_BITS: u32 = 640;

struct Reference {
    lower: BigInt,
    upper: BigInt,
}

impl Reference {
    fn scale() -> BigInt {
        BigInt::from_sign_magnitude(
            Sign::Positive,
            BigUint::one()
                .unwrap()
                .shl_bits(REFERENCE_BITS as usize)
                .unwrap(),
        )
    }

    fn quotient(n: &BigInt, d: &BigInt, upward: bool) -> BigInt {
        let (q, r) = n.div_rem_euclid(d).unwrap();
        if upward && !r.is_zero() {
            q.add(&integer(1)).unwrap()
        } else {
            q
        }
    }

    fn exact(value: &Dyadic) -> Self {
        assert!(value.exponent() <= REFERENCE_BITS);
        let factor = BigInt::from_sign_magnitude(
            Sign::Positive,
            BigUint::one()
                .unwrap()
                .shl_bits((REFERENCE_BITS - value.exponent()) as usize)
                .unwrap(),
        );
        let lower = value.integer().mul(&factor).unwrap();
        Self {
            upper: lower.try_clone().unwrap(),
            lower,
        }
    }

    fn number(value: i64) -> Self {
        Self::exact(&dyadic(value, 0))
    }

    fn add(&self, rhs: &Self) -> Self {
        Self {
            lower: self.lower.add(&rhs.lower).unwrap(),
            upper: self.upper.add(&rhs.upper).unwrap(),
        }
    }

    fn negate(&self) -> Self {
        Self {
            lower: self.upper.negated().unwrap(),
            upper: self.lower.negated().unwrap(),
        }
    }

    fn mul(&self, rhs: &Self) -> Self {
        let products = [
            self.lower.mul(&rhs.lower).unwrap(),
            self.lower.mul(&rhs.upper).unwrap(),
            self.upper.mul(&rhs.lower).unwrap(),
            self.upper.mul(&rhs.upper).unwrap(),
        ];
        Self {
            lower: Self::quotient(products.iter().min().unwrap(), &Self::scale(), false),
            upper: Self::quotient(products.iter().max().unwrap(), &Self::scale(), true),
        }
    }

    fn divide(&self, denominator: i64) -> Self {
        assert!(denominator > 0);
        Self {
            lower: Self::quotient(&self.lower, &integer(denominator), false),
            upper: Self::quotient(&self.upper, &integer(denominator), true),
        }
    }

    fn magnitude(&self) -> BigInt {
        let a = BigInt::from_sign_magnitude(Sign::Positive, self.lower.abs().unwrap());
        let b = BigInt::from_sign_magnitude(Sign::Positive, self.upper.abs().unwrap());
        a.max(b)
    }

    fn enlarge(&self, radius: &BigInt) -> Self {
        Self {
            lower: self.lower.sub(radius).unwrap(),
            upper: self.upper.add(radius).unwrap(),
        }
    }

    fn enclosure(&self) -> Enclosure {
        interval(
            Dyadic::new(self.lower.try_clone().unwrap(), REFERENCE_BITS),
            Dyadic::new(self.upper.try_clone().unwrap(), REFERENCE_BITS),
        )
    }
}

fn independent_atan_inverse(denominator: i64) -> Reference {
    let x = Reference::number(1).divide(denominator);
    let square = x.mul(&x);
    let mut power = x;
    let mut sum = Reference::number(0);
    for n in 0..200 {
        let term = power.divide(2 * n + 1);
        sum = sum.add(&if n % 2 == 0 { term } else { term.negate() });
        power = power.mul(&square);
    }
    sum.enlarge(&power.divide(401).magnitude())
}

fn independent_pi() -> Reference {
    independent_atan_inverse(5)
        .mul(&Reference::number(16))
        .add(&independent_atan_inverse(239).mul(&Reference::number(-4)))
}

fn independent_point(kind: TranscendentalFunction, x: &Dyadic, pi: &Reference) -> Reference {
    let x = Reference::exact(x);
    let period = pi.mul(&Reference::number(2));
    let midpoint = period.lower.add(&period.upper).unwrap();
    let turns = Reference::quotient(
        &x.lower
            .mul(&integer(2))
            .unwrap()
            .add(&period.lower)
            .unwrap(),
        &midpoint,
        false,
    );
    let turns = Reference {
        lower: turns.try_clone().unwrap(),
        upper: turns,
    };
    let turn_value = Reference {
        lower: turns.lower.mul(&Reference::scale()).unwrap(),
        upper: turns.upper.mul(&Reference::scale()).unwrap(),
    };
    let reduced = x.add(&period.mul(&turn_value).negate());
    assert!(reduced.magnitude() <= Reference::number(4).upper);
    let square = reduced.mul(&reduced).negate();
    let mut term = if kind == TranscendentalFunction::Sine {
        reduced
    } else {
        Reference::number(1)
    };
    let mut sum = Reference::number(0);
    for n in 0..160 {
        sum = sum.add(&term);
        let degree = 2 * n
            + if kind == TranscendentalFunction::Sine {
                1
            } else {
                0
            };
        term = term.mul(&square).divide((degree + 1) * (degree + 2));
    }
    let remainder = term.magnitude().mul(&integer(2)).unwrap();
    assert!(remainder < Reference::exact(&unit(600)).upper);
    sum.enlarge(&remainder)
}

fn independent_range(kind: TranscendentalFunction, input: &Enclosure, pi: &Reference) -> Enclosure {
    assert!(input.width().unwrap() < dyadic(1, 0));
    assert!(pi.lower > Reference::number(3).upper);
    assert!(pi.upper < Reference::number(4).lower);
    assert!(Reference::exact(input.lower()).magnitude() < Reference::number(1_i64 << 41).upper);
    let a = independent_point(kind, input.lower(), pi);
    let b = independent_point(kind, input.upper(), pi);
    let mut result = Reference {
        lower: a.lower.min(b.lower),
        upper: a.upper.max(b.upper),
    };
    let lower = Reference::exact(input.lower());
    let upper = Reference::exact(input.upper());
    let index = Reference::quotient(&lower.lower, &pi.lower, false);
    for offset in -2..=2 {
        let k = index.add(&integer(offset)).unwrap();
        let k_scaled = k.mul(&Reference::scale()).unwrap();
        let factor = Reference {
            upper: k_scaled.try_clone().unwrap(),
            lower: k_scaled,
        };
        let factor = if kind == TranscendentalFunction::Sine {
            factor.add(&Reference::number(1).divide(2))
        } else {
            factor
        };
        let extremum = pi.mul(&factor);
        if extremum.upper >= lower.lower && extremum.lower <= upper.upper {
            let odd = !k
                .magnitude()
                .div_rem(&BigUint::try_from(2_u32).unwrap())
                .unwrap()
                .1
                .is_zero();
            let extreme = Reference::number(if odd { -1 } else { 1 });
            result.lower = result.lower.min(extreme.lower);
            result.upper = result.upper.max(extreme.upper);
        }
    }
    result.enclosure()
}

fn contains_reference(actual: &Enclosure, reference: &Enclosure) -> bool {
    actual.lower() <= reference.lower() && actual.upper() >= reference.upper()
}

fn negate_interval(input: &Enclosure) -> Enclosure {
    interval(
        Dyadic::new(
            input.upper().integer().negated().unwrap(),
            input.upper().exponent(),
        ),
        Dyadic::new(
            input.lower().integer().negated().unwrap(),
            input.lower().exponent(),
        ),
    )
}

#[test]
fn independent_machin_taylor_ranges_certify_all_trigonometric_vectors() {
    let pi = independent_pi();
    let pi_bounds = pi.enclosure();
    assert!(pi_bounds.width().unwrap() < unit(620));
    let center = hex_dyadic(PI_OVER_SIX_224, 224);
    let half = center.mul(&dyadic(3, 0)).unwrap();
    let full = center.mul(&dyadic(6, 0)).unwrap();
    let half_bracket = around(&half, 211);
    let full_bracket = around(&full, 211);
    assert!(contains_reference(&half_bracket, &pi.divide(2).enclosure()));
    assert!(contains_reference(&full_bracket, &pi_bounds));
    let phase = rational_floor_interval(1, 96, 200);
    let large = interval(
        phase.lower().add(&dyadic(1_i64 << 40, 0)).unwrap(),
        phase.upper().add(&dyadic(1_i64 << 40, 0)).unwrap(),
    );
    let product = Dyadic::from_f64_exact(2.0 * std::f64::consts::PI)
        .unwrap()
        .mul(&Dyadic::from_f64_exact(27.4870818057832).unwrap())
        .unwrap();
    let scaled = Reference::exact(&product).divide(48000).enclosure();
    let divisor = BigInt::from_sign_magnitude(
        Sign::Positive,
        BigUint::one()
            .unwrap()
            .shl_bits((REFERENCE_BITS - 200) as usize)
            .unwrap(),
    );
    let actual_lower =
        Reference::quotient(&Reference::exact(scaled.lower()).lower, &divisor, false);
    let actual_phase = interval(
        Dyadic::new(actual_lower.try_clone().unwrap(), 200),
        Dyadic::new(actual_lower.add(&integer(1)).unwrap(), 200),
    );
    assert!(contains_reference(&actual_phase, &scaled));
    let mut vectors = vec![phase, large, actual_phase, half_bracket, full_bracket];
    for center in [&half, &full] {
        let before = interval(
            center.sub(&unit(209)).unwrap(),
            center.sub(&unit(210)).unwrap(),
        );
        let after = interval(
            center.add(&unit(210)).unwrap(),
            center.add(&unit(209)).unwrap(),
        );
        let certified_center = if center == &half {
            pi.divide(2).enclosure()
        } else {
            interval(
                pi_bounds.lower().try_clone().unwrap(),
                pi_bounds.upper().try_clone().unwrap(),
            )
        };
        assert!(before.upper() < certified_center.lower());
        assert!(after.lower() > certified_center.upper());
        vectors.push(before);
        vectors.push(after);
    }
    let negatives: Vec<_> = vectors.iter().map(negate_interval).collect();
    vectors.extend(negatives);
    for input in &vectors {
        for kind in [TranscendentalFunction::Sine, TranscendentalFunction::Cosine] {
            let reference = independent_range(kind, input, &pi);
            assert!(reference.width().unwrap() < unit(190));
            let evaluation = function(
                kind,
                interval(
                    input.lower().try_clone().unwrap(),
                    input.upper().try_clone().unwrap(),
                ),
                72,
            )
            .unwrap();
            let actual = assert_enclosure_within(&evaluation, 72);
            assert!(
                contains_reference(actual, &reference),
                "{kind:?}: {input:?}"
            );
            if (input == &vectors[0] || input == &vectors[9])
                && kind == TranscendentalFunction::Cosine
            {
                assert!(actual.upper() < &dyadic(1, 0));
            }
            assert_lineage(&evaluation, 72);
        }
    }
    let small = rational_floor_interval(1, 96, 200);
    assert!(!contains_reference(
        &point(unit(6)),
        &independent_range(TranscendentalFunction::Sine, &small, &pi)
    ));
    assert!(!contains_reference(
        &point(dyadic(1, 0).sub(&unit(15)).unwrap()),
        &independent_range(TranscendentalFunction::Cosine, &small, &pi)
    ));
    for input in [&vectors[1], &vectors[10]] {
        for kind in [TranscendentalFunction::Sine, TranscendentalFunction::Cosine] {
            assert!(!contains_reference(
                &point(dyadic(0, 0)),
                &independent_range(kind, input, &pi)
            ));
        }
    }
    for kind in [TranscendentalFunction::Sine, TranscendentalFunction::Cosine] {
        let zero_reference = independent_point(kind, &dyadic(0, 0), &pi).enclosure();
        assert!(zero_reference.contains(&dyadic(
            if kind == TranscendentalFunction::Sine {
                0
            } else {
                1
            },
            0
        )));
        let evaluation = function(kind, interval(dyadic(-8, 0), dyadic(8, 0)), 72).unwrap();
        let achieved = evaluation.undetermined_achieved().unwrap();
        assert!(achieved.contains(&dyadic(-1, 0)) && achieved.contains(&dyadic(1, 0)));
        assert!(evaluation.certify_f64().is_err());
    }
}
#[test]
fn trigonometric_precision_uses_the_certified_output_range() {
    let phase = rational_floor_interval(1, 96, 70);
    let pi = independent_pi();
    let cosine_reference = independent_range(TranscendentalFunction::Cosine, &phase, &pi);
    let cosine = function(TranscendentalFunction::Cosine, phase, 72).unwrap();
    assert!(!cosine.is_undetermined());
    assert!(contains_reference(
        assert_enclosure_within(&cosine, 72),
        &cosine_reference
    ));

    let phase = rational_floor_interval(1, 96, 70);
    let sine_reference = independent_range(TranscendentalFunction::Sine, &phase, &pi);
    let sine = function(TranscendentalFunction::Sine, phase, 72).unwrap();
    assert!(sine.is_undetermined());
    assert!(sine.undetermined_achieved().unwrap().width().unwrap() > unit(72));
    assert!(contains_reference(
        sine.undetermined_achieved().unwrap(),
        &sine_reference
    ));
}

#[test]
fn powers_of_dyadic_bases_enclose_and_certify_the_exact_value() {
    let cases: [(Dyadic, i64, Dyadic, f64); 5] = [
        (dyadic(3, 2), 2, dyadic(9, 4), 0.5625),
        (dyadic(3, 0), 1, dyadic(3, 0), 3.0),
        (dyadic(7, 2), 2, dyadic(49, 4), 3.0625),
        (dyadic(5, 3), 2, dyadic(25, 6), 0.390625),
        (dyadic(1, 1), 3, dyadic(1, 3), 0.125),
    ];
    for (base, exponent, expected, expected_f64) in cases {
        let evaluation = power(point(base), point(dyadic(exponent, 0)), 64).unwrap();
        let enclosure = assert_enclosure_within(&evaluation, 64);
        assert!(enclosure.contains(&expected));
        assert_eq!(evaluation.certify_f64().unwrap().value(), expected_f64);
    }
}

#[test]
fn power_with_wide_inputs_is_undetermined() {
    let wide_exponent = around(&dyadic(7, 3), 9);
    let evaluation = power(point(dyadic(2, 0)), wide_exponent, 200).unwrap();
    assert!(evaluation.is_undetermined());
    let achieved = evaluation.undetermined_achieved().unwrap();
    assert!(achieved.width().unwrap() <= unit(6));
    assert!(achieved.contains(&hex_dyadic(TWO_TO_SEVEN_EIGHTHS_100, 100)));

    let upper = dyadic(2, 0).add(&unit(9)).unwrap();
    let evaluation = power(interval(dyadic(2, 0), upper), point(dyadic(2, 0)), 200).unwrap();
    assert!(evaluation.is_undetermined());
    let achieved = evaluation.undetermined_achieved().unwrap();
    assert!(achieved.width().unwrap() <= unit(6));
    assert!(achieved.contains(&dyadic(4, 0)));
}
