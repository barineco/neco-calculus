import NecoCalculusVerify.Observation
import NecoCalculusVerify.SurfacePolynomial

set_option autoImplicit false

namespace NecoCalculusVerify

open NecoLean.Json
open NecoAlgebraVerify

private structure DecodedSurfaceTerms where
  terms : List SurfaceTerm

private def decodeSurfaceTerms (value : Value) : Option DecodedSurfaceTerms := do
  let values ← decodeValueArray value
  if values.length > 4096 then none
  let decoded ← values.mapM fun term => do
    let fields ← jsonObjectWithExactFields? term ["coefficient", "exponentU", "exponentV"]
    let coefficient ← decodeRationalSchema (← jsonField? fields "coefficient")
    let exponentU ← decodeSurfaceExponent fields "exponentU"
    let exponentV ← decodeSurfaceExponent fields "exponentV"
    pure ({ coefficient, exponentU, exponentV : SurfaceTerm })
  pure { terms := SurfacePolynomial.normalize decoded }

private def decodeNatArray (value : Value) : Option (List Nat) :=
  match value with
  | .array values => values.mapM jsonNat?
  | _ => none

private def decodeDecision (value : Value) : Option (String × List Nat) := do
  let kindFields ← jsonFields? value
  let kind ← jsonStringField? kindFields "kind"
  if kind == "equality" then
    let fields ← jsonObjectWithExactFields? value ["expressions", "kind"]
    let expressions ← decodeNatArray (← jsonField? fields "expressions")
    pure (kind, expressions)
  else
    let fields ← jsonObjectWithExactFields? value ["expression", "kind"]
    let expression ← jsonNat? (← jsonField? fields "expression")
    pure (kind, [expression])

private def decodeProvenance (value : Value) : Option Provenance := do
  let fields ← jsonObjectWithExactFields? value
    ["consumerId", "decision", "expressions", "owner", "precision", "source"]
  let consumerId ← decodeProvenanceIndex fields "consumerId"
  let expressions ← decodeNatArray (← jsonField? fields "expressions")
  if expressions.isEmpty then none else pure ()
  let (decisionKind, decisionExpressions) ← decodeDecision (← jsonField? fields "decision")
  let owner ← decodeProvenanceIndex fields "owner"
  let precision ← decodeNatAtMost fields "precision" maxSchemaU32
  let sourceFields ← jsonObjectWithExactFields? (← jsonField? fields "source")
    ["publicApiIdentity", "revisionIdentity"]
  let sourcePublicApiIdentity ← jsonStringField? sourceFields "publicApiIdentity"
  let sourceRevisionIdentity ← jsonStringField? sourceFields "revisionIdentity"
  pure {
    consumerId
    expressions
    decisionKind
    decisionExpressions
    owner
    precision
    sourcePublicApiIdentity
    sourceRevisionIdentity
  }

private structure SurfaceExactInput where
  x : List SurfaceTerm
  y : List SurfaceTerm
  z : List SurfaceTerm
  denominator : List SurfaceTerm
  u : Rat
  v : Rat

private structure SurfaceCheckedInput where
  x : List SurfaceTerm
  y : List SurfaceTerm
  z : List SurfaceTerm
  denominator : List SurfaceTerm
  u : Interval
  v : Interval
  precisionBits : Nat
  inputProvenance : Provenance

private def decodeExactInput (value : Value) : Option SurfaceExactInput := do
  let fields ← jsonObjectWithExactFields? value ["denominator", "u", "v", "x", "y", "z"]
  let denominator ← decodeSurfaceTerms (← jsonField? fields "denominator")
  let x ← decodeSurfaceTerms (← jsonField? fields "x")
  let y ← decodeSurfaceTerms (← jsonField? fields "y")
  let z ← decodeSurfaceTerms (← jsonField? fields "z")
  let u ← decodeExactIntegerRat (← jsonField? fields "u")
  let v ← decodeExactIntegerRat (← jsonField? fields "v")
  pure {
    x := x.terms
    y := y.terms
    z := z.terms
    denominator := denominator.terms
    u
    v
  }

private def decodeCheckedInput (value : Value) : Option SurfaceCheckedInput := do
  let fields ← jsonObjectWithExactFields? value
    ["denominator", "provenance", "u", "v", "x", "y", "z"]
  let denominator ← decodeSurfaceTerms (← jsonField? fields "denominator")
  let x ← decodeSurfaceTerms (← jsonField? fields "x")
  let y ← decodeSurfaceTerms (← jsonField? fields "y")
  let z ← decodeSurfaceTerms (← jsonField? fields "z")
  let u ← decodeDyadicInterval (← jsonField? fields "u")
  let v ← decodeDyadicInterval (← jsonField? fields "v")
  let inputProvenance ← decodeProvenance (← jsonField? fields "provenance")
  pure {
    x := x.terms
    y := y.terms
    z := z.terms
    denominator := denominator.terms
    u
    v
    precisionBits := inputProvenance.precision
    inputProvenance
  }

def SurfaceExactObservation.Valid (observation : SurfaceExactObservation) : Prop :=
  let divisor := surfaceValue observation.denominator observation.u observation.v
  divisor ≠ 0 ∧
    observation.observedX = surfaceValue observation.x observation.u observation.v / divisor ∧
    observation.observedY = surfaceValue observation.y observation.u observation.v / divisor ∧
    observation.observedZ = surfaceValue observation.z observation.u observation.v / divisor

instance surfaceExactObservationDecidableValid
    (observation : SurfaceExactObservation) : Decidable observation.Valid := by
  unfold SurfaceExactObservation.Valid
  infer_instance

def SurfaceCheckedObservation.Valid (observation : SurfaceCheckedObservation) : Prop :=
  let denominatorRange := surfaceRange observation.denominator observation.u observation.v
  let xRange := Interval.div (surfaceRange observation.x observation.u observation.v) denominatorRange
  let yRange := Interval.div (surfaceRange observation.y observation.u observation.v) denominatorRange
  let zRange := Interval.div (surfaceRange observation.z observation.u observation.v) denominatorRange
  0 < observation.precisionBits ∧
    observation.u.Valid ∧ observation.v.Valid ∧ denominatorRange.ExcludesZero ∧
    observation.output.x.Valid ∧ observation.output.y.Valid ∧ observation.output.z.Valid ∧
    observation.outputProvenance = observation.inputProvenance ∧
    observation.output.x.lower ≤ xRange.lower ∧ xRange.upper ≤ observation.output.x.upper ∧
    observation.output.y.lower ≤ yRange.lower ∧ yRange.upper ≤ observation.output.y.upper ∧
    observation.output.z.lower ≤ zRange.lower ∧ zRange.upper ≤ observation.output.z.upper

instance surfaceCheckedObservationDecidableValid
    (observation : SurfaceCheckedObservation) : Decidable observation.Valid := by
  unfold SurfaceCheckedObservation.Valid
  infer_instance

def SurfaceExactFailureObservation.Valid
    (observation : SurfaceExactFailureObservation) : Prop := observation.Semantics

instance surfaceExactFailureObservationDecidableValid
    (observation : SurfaceExactFailureObservation) : Decidable observation.Valid := by
  unfold SurfaceExactFailureObservation.Valid SurfaceExactFailureObservation.Semantics
  infer_instance

def SurfaceCheckedFailureObservation.Valid
    (observation : SurfaceCheckedFailureObservation) : Prop := observation.Semantics

instance surfaceCheckedFailureObservationDecidableValid
    (observation : SurfaceCheckedFailureObservation) : Decidable observation.Valid := by
  unfold SurfaceCheckedFailureObservation.Valid SurfaceCheckedFailureObservation.Semantics
  infer_instance

def acceptSurfaceExact (observation : SurfaceExactObservation) : Bool :=
  decide observation.Valid

def acceptSurfaceChecked (observation : SurfaceCheckedObservation) : Bool :=
  decide observation.Valid

def acceptSurfaceExactFailure (observation : SurfaceExactFailureObservation) : Bool :=
  decide observation.Valid

def acceptSurfaceCheckedFailure (observation : SurfaceCheckedFailureObservation) : Bool :=
  decide observation.Valid

private structure SurfaceExactOutput where
  x : Rat
  y : Rat
  z : Rat

private def decodeExactOutput (value : Value) : Option SurfaceExactOutput := do
  let fields ← jsonObjectWithExactFields? value ["x", "y", "z"]
  let x ← decodeExactIntegerRat (← jsonField? fields "x")
  let y ← decodeExactIntegerRat (← jsonField? fields "y")
  let z ← decodeExactIntegerRat (← jsonField? fields "z")
  pure { x, y, z }

private structure SurfaceCheckedOutput where
  coordinates : CoordinateIntervals
  provenance : Provenance

private def decodeCheckedOutput (value : Value) : Option SurfaceCheckedOutput := do
  let fields ← jsonObjectWithExactFields? value ["provenance", "x", "y", "z"]
  let provenance ← decodeProvenance (← jsonField? fields "provenance")
  let x ← decodeDyadicInterval (← jsonField? fields "x")
  let y ← decodeDyadicInterval (← jsonField? fields "y")
  let z ← decodeDyadicInterval (← jsonField? fields "z")
  pure { coordinates := { x, y, z }, provenance }

def decodeSurfaceExact
    (input : Value) (payload : ObservationPayload) : Option SupportedObservation := do
  let decodedInput ← decodeExactInput input
  match payload with
  | .success value => do
      let output ← decodeExactOutput value
      pure (.surfaceExact {
        x := decodedInput.x
        y := decodedInput.y
        z := decodedInput.z
        denominator := decodedInput.denominator
        u := decodedInput.u
        v := decodedInput.v
        observedX := output.x
        observedY := output.y
        observedZ := output.z
      })
  | .failure name => do
      if name != "undefined-domain" then none
      pure (.surfaceExactFailure {
        x := decodedInput.x
        y := decodedInput.y
        z := decodedInput.z
        denominator := decodedInput.denominator
        u := decodedInput.u
        v := decodedInput.v
        failure := name
      })

def decodeSurfaceChecked
    (input : Value) (payload : ObservationPayload) : Option SupportedObservation := do
  let decodedInput ← decodeCheckedInput input
  match payload with
  | .success value => do
      let output ← decodeCheckedOutput value
      pure (.surfaceChecked {
        x := decodedInput.x
        y := decodedInput.y
        z := decodedInput.z
        denominator := decodedInput.denominator
        u := decodedInput.u
        v := decodedInput.v
        precisionBits := decodedInput.precisionBits
        output := output.coordinates
        inputProvenance := decodedInput.inputProvenance
        outputProvenance := output.provenance
      })
  | .failure name => do
      if name != "zero-denominator-polynomial" then none
      pure (.surfaceCheckedFailure {
        x := decodedInput.x
        y := decodedInput.y
        z := decodedInput.z
        denominator := decodedInput.denominator
        u := decodedInput.u
        v := decodedInput.v
        precisionBits := decodedInput.precisionBits
        inputProvenance := decodedInput.inputProvenance
        failure := name
      })

end NecoCalculusVerify
