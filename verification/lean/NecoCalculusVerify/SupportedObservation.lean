import NecoCalculusVerify.Polynomial
import NecoCalculusVerify.Rational
import NecoCalculusVerify.Roots
import NecoCalculusVerify.LocalRegion
import NecoCalculusVerify.Surface

set_option autoImplicit false

namespace NecoCalculusVerify

def acceptSupportedObservation : SupportedObservation → Bool
  | .polynomialExact observation => acceptPolynomialExact observation
  | .polynomialEnclosure observation => acceptPolynomialEnclosure observation
  | .polynomialExactFailure observation => acceptPolynomialExactFailure observation
  | .polynomialEnclosureFailure observation => acceptPolynomialEnclosureFailure observation
  | .rationalExact observation => acceptRationalExact observation
  | .rationalEnclosure observation => acceptRationalEnclosure observation
  | .rationalExactFailure observation => acceptRationalExactFailure observation
  | .rationalEnclosureFailure observation => acceptRationalEnclosureFailure observation
  | .rootBracket observation => acceptRootBracket observation
  | .bisection observation => acceptBisection observation
  | .rootBracketFailure observation => acceptRootBracketFailure observation
  | .bisectionFailure observation => acceptBisectionFailure observation
  | .evenTouch observation => acceptEvenTouch observation
  | .localTransition observation => acceptLocalTransition observation
  | .evenTouchFailure observation => acceptEvenTouchFailure observation
  | .localTransitionFailure observation => acceptLocalTransitionFailure observation
  | .surfaceExact observation => acceptSurfaceExact observation
  | .surfaceChecked observation => acceptSurfaceChecked observation
  | .surfaceExactFailure observation => acceptSurfaceExactFailure observation
  | .surfaceCheckedFailure observation => acceptSurfaceCheckedFailure observation

end NecoCalculusVerify
