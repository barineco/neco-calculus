import NecoCalculusVerify.Observation

set_option autoImplicit false

namespace NecoCalculusVerify

open NecoLean.Json
open NecoAlgebraVerify

structure EvenTouchInput where
  coefficients : Polynomial
  candidate : Rat
  maximumOrder : Nat

structure LocalTransitionInput where
  coefficients : Polynomial
  lower : Rat
  candidate : Rat
  upper : Rat
  maximumOrder : Nat
  precisionBits : Nat

private def decodeEvenTouchInput (value : Value) : Option EvenTouchInput := do
  let fields ← jsonObjectWithExactFields? value ["candidate", "coefficients", "maxOrder"]
  let coefficients ← decodePolynomialSchema (← jsonField? fields "coefficients")
  let candidate ← decodeExactIntegerRat (← jsonField? fields "candidate")
  let maximumOrder ← decodeMaximumOrder fields "maxOrder"
  pure { coefficients, candidate, maximumOrder }

private def decodeLocalTransitionInput (value : Value) : Option LocalTransitionInput := do
  let fields ← jsonObjectWithExactFields? value
    ["candidate", "coefficients", "lower", "maxOrder", "precisionBits", "upper"]
  let coefficients ← decodePolynomialSchema (← jsonField? fields "coefficients")
  let candidate ← decodeExactIntegerRat (← jsonField? fields "candidate")
  let lower ← decodeExactIntegerRat (← jsonField? fields "lower")
  let upper ← decodeExactIntegerRat (← jsonField? fields "upper")
  let maximumOrder ← decodeMaximumOrder fields "maxOrder"
  let precisionBits ← decodeSuccessPrecision fields "precisionBits"
  pure { coefficients, lower, candidate, upper, maximumOrder, precisionBits }

def EvenTouchObservation.Valid (observation : EvenTouchObservation) : Prop :=
  normalizedPolynomial observation.coefficients ∧
    firstNonzeroOrder observation.coefficients observation.candidate =
      some observation.multiplicity ∧
    observation.multiplicity ≤ observation.maximumOrder ∧
    2 ≤ observation.multiplicity ∧ observation.multiplicity % 2 = 0

instance evenTouchObservationDecidableValid
    (observation : EvenTouchObservation) : Decidable observation.Valid := by
  unfold EvenTouchObservation.Valid
  infer_instance

def LocalTransitionObservation.Valid
    (observation : LocalTransitionObservation) : Prop :=
  normalizedPolynomial observation.coefficients ∧
    observation.lociLength = 1 ∧
    observation.lower < observation.candidate ∧
    observation.candidate < observation.upper ∧
    observation.multiplicity ≤ observation.maximumOrder ∧
    observation.multiplicity % 2 = 1 ∧
    firstNonzeroOrder observation.coefficients observation.candidate =
      some observation.multiplicity ∧
    signChange (eval observation.coefficients observation.lower)
      (eval observation.coefficients observation.upper) ∧
    observation.locusCount = 1 ∧ observation.kind = "transition" ∧
    observation.pointIndex = 0 ∧ observation.sign = "varies"

instance localTransitionObservationDecidableValid
    (observation : LocalTransitionObservation) : Decidable observation.Valid := by
  unfold LocalTransitionObservation.Valid
  infer_instance

def EvenTouchFailureObservation.Valid
    (observation : EvenTouchFailureObservation) : Prop := observation.Semantics

instance evenTouchFailureObservationDecidableValid
    (observation : EvenTouchFailureObservation) : Decidable observation.Valid := by
  unfold EvenTouchFailureObservation.Valid EvenTouchFailureObservation.Semantics
  generalize firstNonzeroOrder observation.coefficients observation.candidate = result
  cases result <;> infer_instance

def LocalTransitionFailureObservation.Valid
    (observation : LocalTransitionFailureObservation) : Prop := observation.Semantics

instance localTransitionFailureObservationDecidableValid
    (observation : LocalTransitionFailureObservation) : Decidable observation.Valid := by
  unfold LocalTransitionFailureObservation.Valid LocalTransitionFailureObservation.Semantics
  infer_instance

def acceptEvenTouch (observation : EvenTouchObservation) : Bool :=
  decide observation.Valid

def acceptLocalTransition (observation : LocalTransitionObservation) : Bool :=
  decide observation.Valid

def acceptEvenTouchFailure (observation : EvenTouchFailureObservation) : Bool :=
  decide observation.Valid

def acceptLocalTransitionFailure
    (observation : LocalTransitionFailureObservation) : Bool :=
  decide observation.Valid

private structure LocalTransitionOutput where
  locusCount : Nat
  lociLength : Nat
  kind : String
  multiplicity : Nat
  pointIndex : Nat
  sign : String

private def decodeLocalTransitionOutput (value : Value) : Option LocalTransitionOutput := do
  let fields ← jsonObjectWithExactFields? value ["loci", "locusCount", "sign"]
  let locusCount ← decodeNatInRange fields "locusCount" 0 1
  let sign ← decodeStringIn fields "sign"
    ["constant-positive", "constant-negative", "constant-zero", "varies", "contains-zero",
      "undetermined"]
  let loci ← decodeValueArray (← jsonField? fields "loci")
  if loci.length > 1 then none
  let (kind, multiplicity, pointIndex) ←
    match loci.head? with
    | none => pure ("", 0, 1)
    | some locus => do
        let locusFields ← jsonObjectWithExactFields? locus ["kind", "multiplicity", "pointIndex"]
        let kind ← decodeStringIn locusFields "kind" ["transition", "singular-contact"]
        let multiplicity ← decodeNatInRange locusFields "multiplicity" 1 64
        let pointIndex ← jsonNatField? locusFields "pointIndex"
        if pointIndex == 0 then pure (kind, multiplicity, pointIndex) else none
  pure { locusCount, lociLength := loci.length, kind, multiplicity, pointIndex, sign }

def decodeEvenTouch
    (input : Value) (payload : ObservationPayload) : Option SupportedObservation := do
  let decodedInput ← decodeEvenTouchInput input
  match payload with
  | .success value => do
      let fields ← jsonObjectWithExactFields? value ["multiplicity"]
      let multiplicity ← decodeNatInRange fields "multiplicity" 2 64
      pure (.evenTouch {
        coefficients := decodedInput.coefficients
        candidate := decodedInput.candidate
        maximumOrder := decodedInput.maximumOrder
        multiplicity
      })
  | .failure name => do
      if name != "no-even-multiplicity-touch" then none
      pure (.evenTouchFailure {
        coefficients := decodedInput.coefficients
        candidate := decodedInput.candidate
        maximumOrder := decodedInput.maximumOrder
        failure := name
      })

def decodeLocalTransition
    (input : Value) (payload : ObservationPayload) : Option SupportedObservation := do
  let decodedInput ← decodeLocalTransitionInput input
  match payload with
  | .success value => do
      let output ← decodeLocalTransitionOutput value
      pure (.localTransition {
        coefficients := decodedInput.coefficients
        lower := decodedInput.lower
        candidate := decodedInput.candidate
        upper := decodedInput.upper
        maximumOrder := decodedInput.maximumOrder
        precisionBits := decodedInput.precisionBits
        locusCount := output.locusCount
        lociLength := output.lociLength
        kind := output.kind
        multiplicity := output.multiplicity
        pointIndex := output.pointIndex
        sign := output.sign
      })
  | .failure name => do
      if name != "point-outside-enclosure" then none
      pure (.localTransitionFailure {
        coefficients := decodedInput.coefficients
        lower := decodedInput.lower
        candidate := decodedInput.candidate
        upper := decodedInput.upper
        maximumOrder := decodedInput.maximumOrder
        precisionBits := decodedInput.precisionBits
        failure := name
      })

end NecoCalculusVerify
