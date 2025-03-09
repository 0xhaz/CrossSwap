use ark_bn254::Fr;
use ark_ff::{Field, Zero};
use ark_std::{vec::Vec};
use std::str::FromStr;

/// Poseidon hash function parameters
pub struct PoseidonParameters {
    mds_matrix: Vec<Vec<Fr>>,
    round_keys: Vec<Fr>,
    rounds: usize,
    capacity: usize,
    rate: usize,
}

/// Poseidon hash function implementation aligned with PoseidonT3.sol
pub struct Poseidon {
    params: PoseidonParameters,
}

impl Poseidon {
    /// Creates a new Poseidon instance with parameters matching PoseidonT3.sol
    pub fn new(rounds: usize, capacity: usize, rate: usize) -> Self {
        assert_eq!(capacity, 1, "Capacity must be 1 for PoseidonT3 compatibility");
        assert_eq!(rate, 2, "Rate must be 2 for PoseidonT3 compatibility");
        assert_eq!(rounds, 8, "Rounds must be 8 for PoseidonT3 compatibility");

        // MDS matrix from PoseidonT3.sol (BN254 field)
        let mds_matrix = vec![
            vec![
                Fr::from_str("0x109b7f411ba0e4c9b2b70caf5c36a7b194be7c11ad24378bfedb68592ba8118b").unwrap(),
                Fr::from_str("0x2969f27eed31a480b9c36c764379dbca2cc8fdd1415c3dded62940bcde0bd771").unwrap(),
                Fr::from_str("0x143021ec686a3f330d5f9e654638065ce6cd79e28c5b3753326244ee65a1b1a7").unwrap(),
            ],
            vec![
                Fr::from_str("0x16ed41e13bb9c0c66ae119424fddbcbc9314dc9fdbdeea55d6c64543dc4903e0").unwrap(),
                Fr::from_str("0x2e2419f9ec02ec394c9871c832963dc1b89d743c8c7b964029b2311687b1fe23").unwrap(),
                Fr::from_str("0x176cc029695ad02582a70eff08a6fd99d057e12e58e7d7b6b16cdfabc8ee2911").unwrap(),
            ],
            vec![
                Fr::from_str("0x2b90bba00fca0589f617e7dcbfe82e0df706ab640ceb247b791a93b74e36736d").unwrap(),
                Fr::from_str("0x101071f0032379b697315876690f053d148d4e109f5fb065c8aacc55a0f89bfa").unwrap(),
                Fr::from_str("0x19a3fc0a56702bf417ba7fee3802593fa644470307043f7773279cd71d25d5e0").unwrap(),
            ],
        ];

        // Round constants from PoseidonT3.sol (BN254 field)
        let round_keys = vec![
            // Initial constants for state[1] and state[2]
            Fr::from_str("0x00f1445235f2148c5986587169fc1bcd887b08d4d00868df5696fff40956e864").unwrap(),
            Fr::from_str("0x08dff3487e8ac99e1f29a058d0fa80b930c728730b7ab36ce879f3890ecf73f5").unwrap(),
            // Round 0
            Fr::from_str("0x2f27be690fdaee46c3ce28f7532b13c856c35342c84bda6e20966310fadc01d0").unwrap(),
            Fr::from_str("0x2b2ae1acf68b7b8d2416bebf3d4f6234b763fe04b8043ee48b8327bebca16cf2").unwrap(),
            Fr::from_str("0x0319d062072bef7ecca5eac06f97d4d55952c175ab6b03eae64b44c7dbf11cfa").unwrap(),
            // Round 1
            Fr::from_str("0x28813dcaebaeaa828a376df87af4a63bc8b7bf27ad49c6298ef7b387bf28526d").unwrap(),
            Fr::from_str("0x2727673b2ccbc903f181bf38e1c1d40d2033865200c352bc150928adddf9cb78").unwrap(),
            Fr::from_str("0x234ec45ca27727c2e74abd2b2a1494cd6efbd43e340587d6b8fb9e31e65cc632").unwrap(),
            // Round 2
            Fr::from_str("0x15b52534031ae18f7f862cb2cf7cf760ab10a8150a337b1ccd99ff6e8797d428").unwrap(),
            Fr::from_str("0x0dc8fad6d9e4b35f5ed9a3d186b79ce38e0e8a8d1b58b132d701d4eecf68d1f6").unwrap(),
            Fr::from_str("0x1bcd95ffc211fbca600f705fad3fb567ea4eb378f62e1fec97805518a47e4d9c").unwrap(),
            // Round 3
            Fr::from_str("0x10520b0ab721cadfe9eff81b016fc34dc76da36c2578937817cb978d069de559").unwrap(),
            Fr::from_str("0x1f6d48149b8e7f7d9b257d8ed5fbbaf42932498075fed0ace88a9eb81f5627f6").unwrap(),
            Fr::from_str("0x1d9655f652309014d29e00ef35a2089bfff8dc1c816f0dc9ca34bdb5460c8705").unwrap(),
            // Round 4
            Fr::from_str("0x04df5a56ff95bcafb051f7b1cd43a99ba731ff67e47032058fe3d4185697cc7d").unwrap(),
            Fr::from_str("0x0672d995f8fff640151b3d290cedaf148690a10a8c8424a7f6ec282b6e4be828").unwrap(),
            Fr::from_str("0x099952b414884454b21200d7ffafdd5f0c9a9dcc06f2708e9fc1d8209b5c75b9").unwrap(),
            // Round 5
            Fr::from_str("0x052cba2255dfd00c7c483143ba8d469448e43586a9b4cd9183fd0e843a6b9fa6").unwrap(),
            Fr::from_str("0x0b8badee690adb8eb0bd74712b7999af82de55707251ad7716077cb93c464ddc").unwrap(),
            Fr::from_str("0x119b1590f13307af5a1ee651020c07c749c15d60683a8050b963d0a8e4b2bdd1").unwrap(),
            // Round 6
            Fr::from_str("0x03150b7cd6d5d17b2529d36be0f67b832c4acfc884ef4ee5ce15be0bfb4a8d09").unwrap(),
            Fr::from_str("0x2cc6182c5e14546e3cf1951f173912355374efb83d80898abe69cb317c9ea565").unwrap(),
            Fr::from_str("0x005032551e6378c450cfe129a404b3764218cadedac14e2b92d2cd73111bf0f9").unwrap(),
            // Round 7
            Fr::from_str("0x233237e3289baa34bb147e972ebcb9516469c399fcc069fb88f9da2cc28276b5").unwrap(),
            Fr::from_str("0x05c8f4f4ebd4a6e3c980d31674bfbe6323037f21b34ae5a4e80c2d4c24d60280").unwrap(),
            Fr::from_str("0x0a7b1db13042d396ba05d818a319f25252bcf35ef3aeed91ee1f09b2590fc65b").unwrap(),
        ];

        Poseidon {
            params: PoseidonParameters {
                mds_matrix,
                round_keys,
                rounds,
                capacity,
                rate,
            },
        }
    }

    /// Computes the Poseidon hash of 2 inputs, matching PoseidonT3.sol
    pub fn hash(&self, inputs: &[Fr]) -> Result<Fr, &'static str> {
        if inputs.len() != 2 {
            return Err("PoseidonT3 expects exactly 2 inputs");
        }

        // Initialize state matching PoseidonT3.sol
        let mut state = vec![Fr::zero(); 3];
        state[1] = inputs[0] + self.params.round_keys[0];
        state[2] = inputs[1] + self.params.round_keys[1];

        // 8 full rounds
        for round in 0..self.params.rounds {
            // Apply S-box: x^5
            for s in state.iter_mut() {
                *s = s.pow([5]);
            }

            // MDS mixing
            let mut new_state = vec![Fr::zero(); 3];
            for i in 0..3 {
                for j in 0..3 {
                    new_state[i] += state[j] * self.params.mds_matrix[i][j];
                }
            }

            // Add round constants
            for i in 0..3 {
                new_state[i] += self.params.round_keys[2 + round * 3 + i];
            }
            state = new_state;
        }

        // Final S-box and MDS mix
        for s in state.iter_mut() {
            *s = s.pow([5]);
        }
        let mut final_state = vec![Fr::zero(); 3];
        for i in 0..3 {
            for j in 0..3 {
                final_state[i] += state[j] * self.params.mds_matrix[i][j];
            }
        }

        Ok(final_state[0])
    }
}