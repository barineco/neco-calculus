import NecoCalculusVerify.Observation

set_option autoImplicit false

namespace NecoCalculusVerify

open NecoLean.Json
open NecoAlgebraVerify

structure RationalExactInput where
  numerator : Polynomial
  denominator : Polynomial
  point : Rat

structure RationalEnclosureInput where
  numerator : Polynomial
  denominator : Polynomial
  domain : Interval
  precisionBits : Nat

private def decodeExactInput (value : Value) : Option RationalExactInput := do
  let fields ← jsonObjectWithExactFields? value ["denominator", "numerator", "point"]
  let numerator ← decodePolynomialSchema (← jsonField? fields "numerator")
  let denominator ← decodePolynomialSchema (← jsonField? fields "denominator")
  let point ← decodeExactIntegerRat (← jsonField? fields "point")
  pure { numerator, denominator, point }

private def decodeEnclosureInput (value : Value) : Option RationalEnclosureInput := do
  let fields ← jsonObjectWithExactFields? value
    ["denominator", "lower", "numerator", "precisionBits", "upper"]
  let numerator ← decodePolynomialSchema (← jsonField? fields "numerator")
  let denominator ← decodePolynomialSchema (← jsonField? fields "denominator")
  let lower ← decodeExactIntegerRat (← jsonField? fields "lower")
  let upper ← decodeExactIntegerRat (← jsonField? fields "upper")
  let precisionBits ← decodeSuccessPrecision fields "precisionBits"
  pure { numerator, denominator, domain := { lower, upper }, precisionBits }

def RationalExactObservation.Valid (observation : RationalExactObservation) : Prop :=
  let divisor := eval observation.denominator observation.point
  normalizedPolynomial observation.numerator ∧
    normalizedPolynomial observation.denominator ∧
    divisor ≠ 0 ∧
    observation.observed = eval observation.numerator observation.point / divisor

instance rationalExactObservationDecidableValid
    (observation : RationalExactObservation) : Decidable observation.Valid := by
  unfold RationalExactObservation.Valid
  infer_instance

def RationalEnclosureObservation.Valid
    (observation : RationalEnclosureObservation) : Prop :=
  let denominatorRange := intervalEval observation.denominator observation.domain
  let quotient := Interval.div
    (intervalEval observation.numerator observation.domain) denominatorRange
  normalizedPolynomial observation.numerator ∧
    normalizedPolynomial observation.denominator ∧
    observation.domain.Valid ∧ denominatorRange.ExcludesZero ∧
    observation.output.Valid ∧
    observation.output.lower ≤ quotient.lower ∧ quotient.upper ≤ observation.output.upper

instance rationalEnclosureObservationDecidableValid
    (observation : RationalEnclosureObservation) : Decidable observation.Valid := by
  unfold RationalEnclosureObservation.Valid
  infer_instance

def RationalExactFailureObservation.Valid
    (observation : RationalExactFailureObservation) : Prop := observation.Semantics

instance rationalExactFailureObservationDecidableValid
    (observation : RationalExactFailureObservation) : Decidable observation.Valid := by
  unfold RationalExactFailureObservation.Valid RationalExactFailureObservation.Semantics
  infer_instance

def RationalEnclosureFailureObservation.Valid
    (observation : RationalEnclosureFailureObservation) : Prop := observation.Semantics

instance rationalEnclosureFailureObservationDecidableValid
    (observation : RationalEnclosureFailureObservation) : Decidable observation.Valid := by
  unfold RationalEnclosureFailureObservation.Valid RationalEnclosureFailureObservation.Semantics
  infer_instance

def acceptRationalExact (observation : RationalExactObservation) : Bool :=
  decide observation.Valid

def acceptRationalEnclosure (observation : RationalEnclosureObservation) : Bool :=
  decide observation.Valid

def acceptRationalExactFailure (observation : RationalExactFailureObservation) : Bool :=
  decide observation.Valid

def acceptRationalEnclosureFailure
    (observation : RationalEnclosureFailureObservation) : Bool :=
  decide observation.Valid

private def decodeEnclosureOutput (value : Value) : Option Interval := do
  let fields ← jsonObjectWithExactFields? value ["lower", "upper"]
  let lower ← decodeDyadicRatSchema (← jsonField? fields "lower")
  let upper ← decodeDyadicRatSchema (← jsonField? fields "upper")
  pure { lower, upper }

def decodeRationalExact
    (input : Value) (payload : ObservationPayload) : Option SupportedObservation := do
  let decodedInput ← decodeExactInput input
  match payload with
  | .success value => do
      let fields ← jsonObjectWithExactFields? value ["value"]
      let observed ← decodeExactIntegerRat (← jsonField? fields "value")
      pure (.rationalExact {
        numerator := decodedInput.numerator
        denominator := decodedInput.denominator
        point := decodedInput.point
        observed
      })
  | .failure name => do
      if name != "undefined-domain" then none
      pure (.rationalExactFailure {
        numerator := decodedInput.numerator
        denominator := decodedInput.denominator
        point := decodedInput.point
        failure := name
      })

def decodeRationalEnclosure
    (input : Value) (payload : ObservationPayload) : Option SupportedObservation := do
  let decodedInput ← decodeEnclosureInput input
  match payload with
  | .success value => do
      let output ← decodeEnclosureOutput value
      pure (.rationalEnclosure {
        numerator := decodedInput.numerator
        denominator := decodedInput.denominator
        domain := decodedInput.domain
        precisionBits := decodedInput.precisionBits
        output
      })
  | .failure name => do
      if name != "domain-contains-zero" then none
      pure (.rationalEnclosureFailure {
        numerator := decodedInput.numerator
        denominator := decodedInput.denominator
        domain := decodedInput.domain
        precisionBits := decodedInput.precisionBits
        failure := name
      })

end NecoCalculusVerify
