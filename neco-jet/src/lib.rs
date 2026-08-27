#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

extern crate alloc;

use alloc::vec::Vec;

use neco_algnum::{AlgnumError, RealAlgebraic};

pub use neco_polynomial::DerivativeOrder;

pub const MAX_JET_ORDER: usize = 64;

#[derive(Debug, Eq, PartialEq)]
pub struct Jet {
    value: RealAlgebraic,
    derivatives: Vec<RealAlgebraic>,
}

impl Jet {
    pub fn from_value_and_derivatives(
        value: RealAlgebraic,
        derivatives: Vec<RealAlgebraic>,
    ) -> Result<Self, JetFailure> {
        if derivatives.is_empty() {
            return Err(JetFailure::EmptyDerivatives);
        }
        if derivatives.len() > MAX_JET_ORDER + 1 {
            return Err(JetFailure::OrderLimitExceeded {
                requested: derivatives.len() - 1,
                maximum: MAX_JET_ORDER,
            });
        }
        if derivatives[0] != value {
            return Err(JetFailure::ValueDerivativeMismatch);
        }
        Ok(Self { value, derivatives })
    }

    pub fn value(&self) -> &RealAlgebraic {
        &self.value
    }

    pub fn order(&self) -> DerivativeOrder {
        DerivativeOrder::new(self.derivatives.len() - 1)
    }

    pub fn derivative(&self, order: DerivativeOrder) -> Result<&RealAlgebraic, JetFailure> {
        self.derivatives
            .get(order.get())
            .ok_or(JetFailure::OrderOutOfRange)
    }

    pub fn derivatives(&self) -> &[RealAlgebraic] {
        &self.derivatives
    }

    pub fn shift(&self) -> Result<Self, JetFailure> {
        if self.derivatives.len() <= 1 {
            return Err(JetFailure::OrderOutOfRange);
        }
        let mut shifted = Vec::new();
        for derivative in self.derivatives.iter().skip(1) {
            shifted.push(derivative.try_clone()?);
        }
        let value = shifted[0].try_clone()?;
        Ok(Self {
            value,
            derivatives: shifted,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum JetFailure {
    EmptyDerivatives,
    ValueDerivativeMismatch,
    OrderOutOfRange,
    OrderLimitExceeded { requested: usize, maximum: usize },
    Algebra(AlgnumError),
}

impl From<AlgnumError> for JetFailure {
    fn from(error: AlgnumError) -> Self {
        Self::Algebra(error)
    }
}
