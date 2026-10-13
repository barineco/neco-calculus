import NecoCalculusVerify.Observation

set_option autoImplicit false

namespace NecoCalculusVerify

open NecoLean.Json
open NecoAlgebraVerify

structure PolynomialExactInput where
  coefficients : Polynomial
  point : Rat

structure PolynomialEnclosureInput where
  coefficients : Polynomial
  domain : Interval
  precisionBits : Nat

private def decodeExactInput (value : Value) : Option PolynomialExactInput := do
  let fields ← jsonObjectWithExactFields? value ["coefficients", "point"]
  let coefficients ← decodeEmptyPolynomial (← jsonField? fields "coefficients")
  let point ← decodeExactIntegerRat (← jsonField? fields "point")
  pure { coefficients, point }

private def decodeExactSchemaInput (value : Value) : Option PolynomialExactInput := do
  let fields ← jsonObjectWithExactFields? value ["coefficients", "point"]
  let coefficients ← decodePolynomialSchema (← jsonField? fields "coefficients")
  let point ← decodeExactIntegerRat (← jsonField? fields "point")
  pure { coefficients, point }

private def decodeEnclosureInput (value : Value) : Option PolynomialEnclosureInput := do
  let fields ← jsonObjectWithExactFields? value
    ["coefficients", "lower", "precisionBits", "upper"]
  let coefficients ← decodePolynomialSchema (← jsonField? fields "coefficients")
  let lower ← decodeExactIntegerRat (← jsonField? fields "lower")
  let upper ← decodeExactIntegerRat (← jsonField? fields "upper")
  let precisionBits ← decodeNatAtMost fields "precisionBits" maxSchemaU32
  pure { coefficients, domain := { lower, upper }, precisionBits }

def PolynomialExactObservation.Valid (observation : PolynomialExactObservation) : Prop :=
  normalizedPolynomial observation.coefficients ∧
    observation.observed = eval observation.coefficients observation.point

instance polynomialExactObservationDecidableValid
    (observation : PolynomialExactObservation) : Decidable observation.Valid := by
  unfold PolynomialExactObservation.Valid
  infer_instance

def PolynomialEnclosureObservation.Valid
    (observation : PolynomialEnclosureObservation) : Prop :=
  let expected := intervalEval observation.coefficients observation.domain
  normalizedPolynomial observation.coefficients ∧
    observation.domain.Valid ∧ observation.output.Valid ∧
    observation.output.lower ≤ expected.lower ∧ expected.upper ≤ observation.output.upper

instance polynomialEnclosureObservationDecidableValid
    (observation : PolynomialEnclosureObservation) : Decidable observation.Valid := by
  unfold PolynomialEnclosureObservation.Valid
  infer_instance

def PolynomialExactFailureObservation.Valid
    (observation : PolynomialExactFailureObservation) : Prop := observation.Semantics

instance polynomialExactFailureObservationDecidableValid
    (observation : PolynomialExactFailureObservation) : Decidable observation.Valid := by
  unfold PolynomialExactFailureObservation.Valid PolynomialExactFailureObservation.Semantics
  infer_instance

def PolynomialEnclosureFailureObservation.Valid
    (observation : PolynomialEnclosureFailureObservation) : Prop := observation.Semantics

instance polynomialEnclosureFailureObservationDecidableValid
    (observation : PolynomialEnclosureFailureObservation) : Decidable observation.Valid := by
  unfold PolynomialEnclosureFailureObservation.Valid PolynomialEnclosureFailureObservation.Semantics
  infer_instance

def acceptPolynomialExact (observation : PolynomialExactObservation) : Bool :=
  decide observation.Valid

def acceptPolynomialEnclosure (observation : PolynomialEnclosureObservation) : Bool :=
  decide observation.Valid

def acceptPolynomialExactFailure (observation : PolynomialExactFailureObservation) : Bool :=
  decide observation.Valid

def acceptPolynomialEnclosureFailure
    (observation : PolynomialEnclosureFailureObservation) : Bool :=
  decide observation.Valid

private def decodeEnclosureOutput (value : Value) : Option Interval := do
  let fields ← jsonObjectWithExactFields? value ["lower", "upper"]
  let lower ← decodeDyadicRatSchema (← jsonField? fields "lower")
  let upper ← decodeDyadicRatSchema (← jsonField? fields "upper")
  pure { lower, upper }

def decodePolynomialExact
    (input : Value) (payload : ObservationPayload) : Option SupportedObservation :=
  match payload with
  | .success value => do
      let decodedInput ← decodeExactSchemaInput input
      let fields ← jsonObjectWithExactFields? value ["value"]
      let observed ← decodeExactIntegerRat (← jsonField? fields "value")
      pure (.polynomialExact {
        coefficients := decodedInput.coefficients
        point := decodedInput.point
        observed
      })
  | .failure name => do
      let decodedInput ← decodeExactInput input
      if name != "empty-coefficients" then none
      pure (.polynomialExactFailure {
        coefficients := decodedInput.coefficients
        point := decodedInput.point
        failure := name
      })

def decodePolynomialEnclosure
    (input : Value) (payload : ObservationPayload) : Option SupportedObservation := do
  let decodedInput ← decodeEnclosureInput input
  match payload with
  | .success value => do
      if decodedInput.precisionBits > 256 then none
      let output ← decodeEnclosureOutput value
      pure (.polynomialEnclosure {
        coefficients := decodedInput.coefficients
        domain := decodedInput.domain
        precisionBits := decodedInput.precisionBits
        output
      })
  | .failure name => do
      if decodedInput.precisionBits < 257 || name != "precision-limit-exceeded" then none
      pure (.polynomialEnclosureFailure {
        coefficients := decodedInput.coefficients
        domain := decodedInput.domain
        precisionBits := decodedInput.precisionBits
        failure := name
      })

end NecoCalculusVerify
