// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use super::*;

impl<A: Aleo> Response<A> {
    /// Returns `true` if the outputs match their output IDs, and `false` otherwise.
    pub fn verify(&self, tvk: &Field<A>) -> Boolean<A> {
        // Check the outputs against their output IDs.
        self.output_ids
            .iter()
            .zip_eq(&self.outputs)
            .map(|(output_id, output)| {
                match output_id {
                    // For a constant output, compute the hash of the output, and compare it to the computed hash.
                    OutputID::Constant(expected_hash) => {
                        // Hash the output to a field element.
                        let output_hash = A::hash_bhp1024(&output.to_bits_le());
                        // Ensure the computed hash matches the expected hash.
                        output_hash.is_equal(expected_hash)
                    }
                    // For a public output, compute the hash of the output, and compare it to the computed hash.
                    OutputID::Public(expected_hash) => {
                        // Hash the output to a field element.
                        let output_hash = A::hash_bhp1024(&output.to_bits_le());
                        // Ensure the computed hash matches the expected hash.
                        output_hash.is_equal(expected_hash)
                    }
                    // For a private output, compute the commitment (using `tvk`) for the output, and compare it to the commitment.
                    OutputID::Private(output_index, expected_cm) => {
                        // Compute the commitment randomizer as `HashToScalar(tvk || index)`.
                        let randomizer = A::hash_to_scalar_psd2(&[tvk.clone(), output_index.clone()]);
                        // Commit the output to a field element.
                        let commitment = A::commit_bhp1024(&output.to_bits_le(), &randomizer);
                        // Ensure the computed commitment matches the expected commitment.
                        commitment.is_equal(expected_cm)
                    }
                    // For an output record, compute the record commitment, and encrypt the record (using `tvk`).
                    // An expected record commitment is injected as `Mode::Public`, and compared to the computed record commitment.
                    OutputID::Record(output_index, expected_cm, expected_nonce, expected_checksum) => {
                        // Retrieve the record.
                        let record = match &output {
                            CircuitValue::Record(record) => record,
                            // Ensure the output is a record.
                            CircuitValue::Plaintext(..) => {
                                A::halt("Expected a record output, found a plaintext output")
                            }
                        };

                        // Compute the encryption randomizer as `HashToScalar(tvk || index)`.
                        let randomizer = A::hash_to_scalar_psd2(&[tvk.clone(), output_index.clone()]);
                        // Compute the record commitment.
                        let commitment = record.to_commitment(&randomizer);

                        // Compute the record nonce.
                        let nonce = A::g_scalar_multiply(&randomizer).to_x_coordinate();

                        // Encrypt the record, using the randomizer.
                        let encrypted_record = record.encrypt(&randomizer);
                        // Compute the record checksum, as the hash of the encrypted record.
                        let checksum = A::hash_bhp1024(&encrypted_record.to_bits_le());

                        // Ensure the computed record commitment matches the expected record commitment.
                        commitment.is_equal(expected_cm)
                        // Ensure the computed nonce matches the expected nonce.
                        & nonce.is_equal(expected_nonce)
                        // Ensure the computed record checksum matches the expected record checksum.
                        & checksum.is_equal(expected_checksum)
                    }
                }
            })
            .fold(Boolean::constant(true), |acc, x| acc & x)
    }
}

#[cfg(all(test, console))]
mod tests {
    use super::*;
    use crate::Circuit;
    use snarkvm_utilities::{test_crypto_rng, Uniform};

    use anyhow::Result;

    pub(crate) const ITERATIONS: usize = 50;

    fn check_verify(
        mode: Mode,
        num_constants: u64,
        num_public: u64,
        num_private: u64,
        num_constraints: u64,
    ) -> Result<()> {
        let rng = &mut test_crypto_rng();

        for i in 0..ITERATIONS {
            // Construct four outputs.
            let output_constant = console::StackValue::<<Circuit as Environment>::Network>::Plaintext(
                console::Plaintext::from_str("{ token_amount: 9876543210u128 }").unwrap(),
            );
            let output_public = console::StackValue::<<Circuit as Environment>::Network>::Plaintext(
                console::Plaintext::from_str("{ token_amount: 9876543210u128 }").unwrap(),
            );
            let output_private = console::StackValue::<<Circuit as Environment>::Network>::Plaintext(
                console::Plaintext::from_str("{ token_amount: 9876543210u128 }").unwrap(),
            );
            let output_record = console::StackValue::<<Circuit as Environment>::Network>::Record(console::Record::from_str("{ owner: aleo1d5hg2z3ma00382pngntdp68e74zv54jdxy249qhaujhks9c72yrs33ddah.private, balance: 5u64.private, token_amount: 100u64.private }").unwrap());
            let outputs = vec![output_constant, output_public, output_private, output_record];

            // Construct the output types.
            let output_types = vec![
                console::ValueType::from_str("amount.constant").unwrap(),
                console::ValueType::from_str("amount.public").unwrap(),
                console::ValueType::from_str("amount.private").unwrap(),
                console::ValueType::from_str("token.record").unwrap(),
            ];

            // Sample a `tvk`.
            let tvk = Uniform::rand(rng);

            // Construct the response.
            let response = console::Response::new(4, &tvk, outputs, &output_types)?;
            // assert!(response.verify());

            // Inject the response into a circuit.
            let tvk = Field::<Circuit>::new(mode, tvk);
            let response = Response::<Circuit>::new(mode, response);

            Circuit::scope(format!("Response {i}"), || {
                let candidate = response.verify(&tvk);
                assert!(candidate.eject_value());
                match mode.is_constant() {
                    true => assert_scope!(<=num_constants, <=num_public, <=num_private, <=num_constraints),
                    false => assert_scope!(<=num_constants, num_public, num_private, num_constraints),
                }
            });
            Circuit::reset();
        }
        Ok(())
    }

    #[test]
    fn test_verify_constant() -> Result<()> {
        // Note: This is correct. At this (high) level of a program, we override the default mode in the `Record` case,
        // based on the user-defined visibility in the record type. Thus, we have nonzero private and constraint values.
        check_verify(Mode::Constant, 21534, 0, 7670, 7680)
    }

    #[test]
    fn test_verify_public() -> Result<()> {
        check_verify(Mode::Public, 21000, 0, 12613, 12625)
    }

    #[test]
    fn test_verify_private() -> Result<()> {
        check_verify(Mode::Private, 21000, 0, 12613, 12625)
    }
}
