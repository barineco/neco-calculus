import NecoCalculusVerify.SurfacePolynomial

set_option autoImplicit false

namespace NecoCalculusVerify.RationalPolynomial

open NecoAlgebraVerify

def normalize (coefficients : List Rat) : List Rat :=
  (coefficients.reverse.dropWhile (fun c => c == 0)).reverse

def eval : List Rat → Rat → Rat
  | [], _ => 0
  | coefficient :: rest, value => coefficient + value * eval rest value

def constant (value : Rat) : List Rat := if value == 0 then [] else [value]

def add : List Rat → List Rat → List Rat
  | [], q => q
  | p, [] => p
  | a :: p, b :: q => (a + b) :: add p q

def neg (p : List Rat) : List Rat := p.map Neg.neg

def sub (p q : List Rat) : List Rat := normalize (add p (neg q))

def scale (scalar : Rat) (p : List Rat) : List Rat := p.map (scalar * ·)

def shift (p : List Rat) : List Rat := 0 :: p

def mul : List Rat → List Rat → List Rat
  | [], _ => []
  | a :: p, q => add (scale a q) (shift (mul p q))

def pow : List Rat → Nat → List Rat
  | _, 0 => [1]
  | p, exponent + 1 => mul (pow p exponent) p

def range (coefficients : List Rat) (domain : NecoAlgebraVerify.Interval) :
    NecoAlgebraVerify.Interval :=
  match coefficients with
  | [] => NecoAlgebraVerify.Interval.zero
  | coefficient :: rest =>
      NecoAlgebraVerify.Interval.add (NecoAlgebraVerify.Interval.constant coefficient)
        (NecoAlgebraVerify.Interval.mul domain (range rest domain))

def nonpositiveOn (num den : List Rat) (lo hi : Rat) : Bool :=
  decide (range den { lower := lo, upper := hi }).ExcludesZero &&
    decide ((range num { lower := lo, upper := hi }).upper ≤ 0)

def nonpositiveSub (num den : List Rat) (lo hi : Rat) : Nat → Bool
  | 0 => nonpositiveOn num den lo hi
  | depth + 1 =>
      let mid := (lo + hi) / 2
      nonpositiveOn num den lo hi ||
        (nonpositiveSub num den lo mid depth && nonpositiveSub num den mid hi depth)

def rationalRange (num den : List Rat) (lo hi : Rat) :
    Option NecoAlgebraVerify.Interval :=
  let denominatorRange := range den { lower := lo, upper := hi }
  if denominatorRange.ExcludesZero then
    some (NecoAlgebraVerify.Interval.div (range num { lower := lo, upper := hi })
      denominatorRange)
  else none

def squaredSumWithinOn (nx dx ny dy nz dz : List Rat) (bound lo hi : Rat) : Bool :=
  match rationalRange nx dx lo hi, rationalRange ny dy lo hi, rationalRange nz dz lo hi with
  | some rx, some ry, some rz =>
      decide (NecoAlgebraVerify.Interval.maxAbs rx * NecoAlgebraVerify.Interval.maxAbs rx +
        NecoAlgebraVerify.Interval.maxAbs ry * NecoAlgebraVerify.Interval.maxAbs ry +
        NecoAlgebraVerify.Interval.maxAbs rz * NecoAlgebraVerify.Interval.maxAbs rz ≤ bound)
  | _, _, _ => false

def squaredSumWithinSub (nx dx ny dy nz dz : List Rat) (bound lo hi : Rat) :
    Nat → Bool
  | 0 => squaredSumWithinOn nx dx ny dy nz dz bound lo hi
  | depth + 1 =>
      let mid := (lo + hi) / 2
      squaredSumWithinOn nx dx ny dy nz dz bound lo hi ||
        (squaredSumWithinSub nx dx ny dy nz dz bound lo mid depth &&
          squaredSumWithinSub nx dx ny dy nz dz bound mid hi depth)

def composeSurface (terms : List SurfaceTerm) (uN uD vN vD : List Rat) :
    List Rat × List Rat :=
  let degreeU := SurfacePolynomial.maxDegree terms (·.exponentU)
  let degreeV := SurfacePolynomial.maxDegree terms (·.exponentV)
  let numerator := normalize (terms.foldl (fun acc term =>
    add acc (scale term.coefficient
      (mul (pow uN term.exponentU)
        (mul (pow vN term.exponentV)
          (mul (pow uD (degreeU - term.exponentU))
            (pow vD (degreeV - term.exponentV))))))) [])
  (numerator, normalize (mul (pow uD degreeU) (pow vD degreeV)))

def rationalValueAt (num den : List Rat) (t : Rat) : Rat :=
  eval num t / eval den t

end NecoCalculusVerify.RationalPolynomial
