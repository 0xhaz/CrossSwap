use ark_bn254::Fr;
use ark_ff::{Field};
use ark_std::{vec::Vec, Zero};

pub struct PoseidonParameters {
    rounds: usize,
    capacity: usize,
    rate: usize,
    mds_matrix: Vec<Vec<Fr>>,
    round_keys: Vec<Fr>,
}

pub struct Poseidon {
    params: PoseidonParameters,
}

impl Poseidon {
    pub fn new(rounds: usize, capacity: usize, rate: usize) -> Self {
        // Parameters: rate=2, capacity=1 -> state size = 3, so MDS matrix must be 3x3
        let mds_matrix = vec![
            vec![Fr::from(1u32), Fr::from(2u32), Fr::from(3u32)],
            vec![Fr::from(2u32), Fr::from(1u32), Fr::from(3u32)],
            vec![Fr::from(3u32), Fr::from(2u32), Fr::from(1u32)],
        ];
        let round_keys = vec![Fr::from(1u32); rounds * (capacity + rate)];
        Poseidon {
            params: PoseidonParameters {
                rounds,
                capacity,
                rate,
                mds_matrix,
                round_keys,
            },
        }
    }

    pub fn hash(&self, inputs: &[Fr]) -> Result<Fr, &'static str> {
        if inputs.len() > self.params.rate {
            return Err("Input length exceeds rate");
        }

        let mut state = vec![Fr::zero(); self.params.capacity + self.params.rate];
        for (i, &input) in inputs.iter().enumerate() {
            state[i] = input;
        }

        for round in 0..self.params.rounds {
            let state_len = state.len();
            for i in 0..state_len {
                state[i] += self.params.round_keys[round * state_len + i];
            }

            for s in state.iter_mut() {
                *s = s.pow([5]);
            }

            let mut new_state = vec![Fr::zero(); state_len];
            for i in 0..state_len {
                for j in 0..state_len {
                    new_state[i] += state[j] * self.params.mds_matrix[i][j];
                }
            }
            state = new_state;
        }

        Ok(state[0])
    }
}