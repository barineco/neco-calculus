import NecoCalculusVerify.RationalPolynomial

set_option autoImplicit false

namespace NecoCalculusVerify.RationalPolynomial

theorem eval_intCoefficients (p : List Int) (t : Rat) :
    eval (p.map Rat.ofInt) t = NecoAlgebraVerify.eval p t := by
  induction p with
  | nil => rfl
  | cons coefficient rest ih =>
      simp only [List.map_cons, eval, NecoAlgebraVerify.eval, ih]

theorem range_intCoefficients (p : List Int) (domain : NecoAlgebraVerify.Interval) :
    range (p.map Rat.ofInt) domain = NecoAlgebraVerify.intervalEval p domain := by
  induction p with
  | nil => rfl
  | cons coefficient rest ih =>
      simp only [List.map_cons, range, NecoAlgebraVerify.intervalEval, ih]

end NecoCalculusVerify.RationalPolynomial
