import NecoAlgebraVerify.Canonical
import NecoAlgebraVerify.Semantics

set_option autoImplicit false

namespace NecoCalculusVerify

open NecoLean.Json
open NecoAlgebraVerify

inductive Verdict where
  | verified | refuted | undetermined | invalid
  deriving Repr, BEq, DecidableEq

structure CaseResult where
  caseId : String
  verdict : Verdict
  deriving Repr, BEq

inductive CheckResult where
  | batchInvalid (digest : String)
  | cases (digest : String) (items : List CaseResult)
  deriving Repr, BEq

def decodeIntegerList (value : Value) : Option (List Int) :=
  match value with
  | .array values => values.mapM fun item => jsonString? item >>= decodeDecimalInteger
  | _ => none

def decodePolynomialAtMost (value : Value) : Option (List Int) := do
  let coefficients ← decodeIntegerList value
  if coefficients.length ≤ 256 then some coefficients else none

def decodePolynomialSchema (value : Value) : Option (List Int) := do
  let coefficients ← decodePolynomialAtMost value
  if coefficients.isEmpty then none
  else if coefficients.length == 1 || coefficients.getLast? != some 0 then some coefficients
  else none

def decodeEmptyPolynomial (value : Value) : Option (List Int) := do
  let coefficients ← decodePolynomialAtMost value
  if coefficients.isEmpty then some coefficients else none

def decodeRationalSchema (value : Value) : Option Rat := do
  let fields ← jsonObjectWithExactFields? value ["denominator", "numerator"]
  let numerator ← decodeDecimalInteger (← jsonStringField? fields "numerator")
  let denominator ← decodePositiveNat (← jsonStringField? fields "denominator")
  if Int.gcd numerator denominator == 1 then
    pure (Rat.ofInt numerator / Rat.ofInt (Int.ofNat denominator))
  else
    none

def decodeExactIntegerRat (value : Value) : Option Rat :=
  jsonString? value >>= decodeDecimalInteger |>.map Rat.ofInt

def maxSchemaU32 : Nat := 4294967295

def maxSchemaU64 : Nat := 18446744073709551615

def maxDyadicExponent : Nat := 256

def decodeNatAtMost (fields : JsonFields) (name : String) (maximum : Nat) : Option Nat := do
  let value ← jsonNatField? fields name
  if value ≤ maximum then some value else none

def decodeNatInRange (fields : JsonFields) (name : String) (minimum maximum : Nat) : Option Nat := do
  let value ← jsonNatField? fields name
  if minimum ≤ value && value ≤ maximum then some value else none

def decodeStringIn (fields : JsonFields) (name : String) (allowed : List String) : Option String := do
  let value ← jsonStringField? fields name
  if allowed.contains value then some value else none

def decodeDyadicRatSchema (value : Value) : Option Rat := do
  let fields ← jsonObjectWithExactFields? value ["exponent", "integer"]
  let integer ← decodeDecimalInteger (← jsonStringField? fields "integer")
  let exponent ← decodeNatAtMost fields "exponent" maxDyadicExponent
  pure (Rat.ofInt integer / (2 ^ exponent : Rat))

def decodeProvenanceIndex (fields : JsonFields) (name : String) : Option Nat :=
  decodeNatAtMost fields name maxSchemaU64

def decodeSuccessPrecision (fields : JsonFields) (name : String) : Option Nat :=
  decodeNatAtMost fields name 256

def decodeFailurePrecision (fields : JsonFields) (name : String) : Option Nat :=
  decodeNatInRange fields name 257 maxSchemaU32

def decodeMaximumOrder (fields : JsonFields) (name : String) : Option Nat :=
  decodeNatAtMost fields name 64

def decodeSurfaceExponent (fields : JsonFields) (name : String) : Option Nat :=
  decodeNatAtMost fields name 256

def decodeBisectionSuccessIterations (fields : JsonFields) (name : String) : Option Nat :=
  decodeNatInRange fields name 1 19

def decodeBisectionFailureIterations (fields : JsonFields) (name : String) : Option Nat :=
  do
    let value ← jsonNatField? fields name
    if 20 ≤ value then some value else none

def decodeDyadicInterval (value : Value) : Option Interval := do
  let fields ← jsonObjectWithExactFields? value ["lower", "upper"]
  let lower ← decodeDyadicRatSchema (← jsonField? fields "lower")
  let upper ← decodeDyadicRatSchema (← jsonField? fields "upper")
  pure { lower, upper }

def decodeValueArray (value : Value) : Option (List Value) :=
  match value with
  | .array values => some values
  | _ => none

end NecoCalculusVerify
