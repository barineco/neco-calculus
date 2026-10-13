use neco_calculus::{CalculusFailure, EvaluationFailure};

#[test]
fn calculus_failure_debug_text_keeps_the_unboxed_form() {
    let zero = CalculusFailure::from(EvaluationFailure::ZeroImplicitTerm);
    let duplicate = CalculusFailure::from(EvaluationFailure::DuplicateImplicitExponent);
    assert_eq!(format!("{zero:?}"), "Evaluation(ZeroImplicitTerm)");
    assert_eq!(
        format!("{duplicate:?}"),
        "Evaluation(DuplicateImplicitExponent)"
    );
}
