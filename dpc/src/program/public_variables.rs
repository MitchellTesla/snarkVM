// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use crate::Parameters;
use snarkvm_fields::{ConstraintFieldError, ToConstraintField};

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: Parameters"),
    Debug(bound = "C: Parameters"),
    Default(bound = "C: Parameters")
)]
pub struct PublicVariables<C: Parameters> {
    pub record_position: u8,
    pub local_data_root: C::LocalDataRoot,
}

impl<C: Parameters> PublicVariables<C> {
    pub fn new(record_position: u8, local_data_root: &C::LocalDataRoot) -> Self {
        Self {
            record_position,
            local_data_root: local_data_root.clone(),
        }
    }
}

/// Converts the public variables into bytes and packs them into field elements.
impl<C: Parameters> ToConstraintField<C::InnerScalarField> for PublicVariables<C> {
    fn to_field_elements(&self) -> Result<Vec<C::InnerScalarField>, ConstraintFieldError> {
        let mut v = ToConstraintField::<C::InnerScalarField>::to_field_elements(&[self.record_position][..])?;
        v.extend_from_slice(&self.local_data_root.to_field_elements()?);
        Ok(v)
    }
}