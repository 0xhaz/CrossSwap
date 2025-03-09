use ark_bn254::Fr;
use ark_ff::{Field, Zero};
use ark_std::vec::Vec;
use ark_serialize::CanonicalSerialize;
use ark_ff::PrimeField;
use ark_ff::BigInteger;

pub struct PoseidonParameters {
    mds_matrix: Vec<Vec<Fr>>,
    round_keys: Vec<Fr>,
    full_rounds: usize,
    partial_rounds: usize,
    capacity: usize,
    rate: usize,
}

pub struct Poseidon {
    params: PoseidonParameters,
}

impl Poseidon {
    pub fn new(_rounds: usize, capacity: usize, rate: usize) -> Self {
        assert_eq!(capacity, 1, "Capacity must be 1 for PoseidonT3 compatibility");
        assert_eq!(rate, 2, "Rate must be 2 for PoseidonT3 compatibility");

        // MDS matrix from PoseidonT3.sol (little-endian)
        let mds_matrix = vec![
            vec![
                Fr::from_le_bytes_mod_order(&hex::decode("109b7f411ba0e4c9b2b70caf5c36a7b194be7c11ad24378bfedb68592ba8118b").unwrap()),
                Fr::from_le_bytes_mod_order(&hex::decode("2969f27eed31a480b9c36c764379dbca2cc8fdd1415c3dded62940bcde0bd771").unwrap()),
                Fr::from_le_bytes_mod_order(&hex::decode("143021ec686a3f330d5f9e654638065ce6cd79e28c5b3753326244ee65a1b1a7").unwrap()),
            ],
            vec![
                Fr::from_le_bytes_mod_order(&hex::decode("16ed41e13bb9c0c66ae119424fddbcbc9314dc9fdbdeea55d6c64543dc4903e0").unwrap()),
                Fr::from_le_bytes_mod_order(&hex::decode("2e2419f9ec02ec394c9871c832963dc1b89d743c8c7b964029b2311687b1fe23").unwrap()),
                Fr::from_le_bytes_mod_order(&hex::decode("176cc029695ad02582a70eff08a6fd99d057e12e58e7d7b6b16cdfabc8ee2911").unwrap()),
            ],
            vec![
                Fr::from_le_bytes_mod_order(&hex::decode("2b90bba00fca0589f617e7dcbfe82e0df706ab640ceb247b791a93b74e36736d").unwrap()),
                Fr::from_le_bytes_mod_order(&hex::decode("101071f0032379b697315876690f053d148d4e109f5fb065c8aacc55a0f89bfa").unwrap()),
                Fr::from_le_bytes_mod_order(&hex::decode("19a3fc0a56702bf417ba7fee3802593fa644470307043f7773279cd71d25d5e0").unwrap()),
            ],
        ];

        // Full list of 58 constants from PoseidonT3.sol
        let round_keys = vec![
            // Initial constants
            Fr::from_le_bytes_mod_order(&hex::decode("00f1445235f2148c5986587169fc1bcd887b08d4d00868df5696fff40956e864").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("08dff3487e8ac99e1f29a058d0fa80b930c728730b7ab36ce879f3890ecf73f5").unwrap()),
            // Round constants (56 rounds)
            Fr::from_le_bytes_mod_order(&hex::decode("2f27be690fdaee46c3ce28f7532b13c856c35342c84bda6e20966310fadc01d0").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("2b2ae1acf68b7b8d2416bebf3d4f6234b763fe04b8043ee48b8327bebca16cf2").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("0319d062072bef7ecca5eac06f97d4d55952c175ab6b03eae64b44c7dbf11cfa").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("28813dcaebaeaa828a376df87af4a63bc8b7bf27ad49c6298ef7b387bf28526d").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("2727673b2ccbc903f181bf38e1c1d40d2033865200c352bc150928adddf9cb78").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("234ec45ca27727c2e74abd2b2a1494cd6efbd43e340587d6b8fb9e31e65cc632").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("15b52534031ae18f7f862cb2cf7cf760ab10a8150a337b1ccd99ff6e8797d428").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("0dc8fad6d9e4b35f5ed9a3d186b79ce38e0e8a8d1b58b132d701d4eecf68d1f6").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("1bcd95ffc211fbca600f705fad3fb567ea4eb378f62e1fec97805518a47e4d9c").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("10520b0ab721cadfe9eff81b016fc34dc76da36c2578937817cb978d069de559").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("1f6d48149b8e7f7d9b257d8ed5fbbaf42932498075fed0ace88a9eb81f5627f6").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("1d9655f652309014d29e00ef35a2089bfff8dc1c816f0dc9ca34bdb5460c8705").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("04df5a56ff95bcafb051f7b1cd43a99ba731ff67e47032058fe3d4185697cc7d").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("0672d995f8fff640151b3d290cedaf148690a10a8c8424a7f6ec282b6e4be828").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("099952b414884454b21200d7ffafdd5f0c9a9dcc06f2708e9fc1d8209b5c75b9").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("052cba2255dfd00c7c483143ba8d469448e43586a9b4cd9183fd0e843a6b9fa6").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("0b8badee690adb8eb0bd74712b7999af82de55707251ad7716077cb93c464ddc").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("119b1590f13307af5a1ee651020c07c749c15d60683a8050b963d0a8e4b2bdd1").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("03150b7cd6d5d17b2529d36be0f67b832c4acfc884ef4ee5ce15be0bfb4a8d09").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("2cc6182c5e14546e3cf1951f173912355374efb83d80898abe69cb317c9ea565").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("005032551e6378c450cfe129a404b3764218cadedac14e2b92d2cd73111bf0f9").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("233237e3289baa34bb147e972ebcb9516469c399fcc069fb88f9da2cc28276b5").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("05c8f4f4ebd4a6e3c980d31674bfbe6323037f21b34ae5a4e80c2d4c24d60280").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("0a7b1db13042d396ba05d818a319f25252bcf35ef3aeed91ee1f09b2590fc65b").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("2a73b71f9b210cf5b14296572c9d32dbf156e2b086ff47dc5df542365a404ec0").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("1ac9b0417abcc9a1935107e9ffc91dc3ec18f2c4dbe7f22976a760bb5c50c460").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("12c0339ae08374823fabb076707ef479269f3e4d6cb104349015ee046dc93fc0").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("0b7475b102a165ad7f5b18db4e1e704f52900aa3253baac68246682e56e9a28e").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("037c2849e191ca3edb1c5e49f6e8b8917c843e379366f2ea32ab3aa88d7f8448").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("05a6811f8556f014e92674661e217e9bd5206c5c93a07dc145fdb176a716346f").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("29a795e7d98028946e947b75d54e9f044076e87a7b2883b47b675ef5f38bd66e").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("20439a0c84b322eb45a3857afc18f5826e8c7382c8a1585c507be199981fd22f").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("2e0ba8d94d9ecf4a94ec2050c7371ff1bb50f27799a84b6d4a2a6f2a0982c887").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("143fd115ce08fb27ca38eb7cce822b4517822cd2109048d2e6d0ddcca17d71c8").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("0c64cbecb1c734b857968dbbdcf813cdf8611659323dbcbfc84323623be9caf1").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("028a305847c683f646fca925c163ff5ae74f348d62c2b670f1426cef9403da53").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("2e4ef510ff0b6fda5fa940ab4c4380f26a6bcb64d89427b824d6755b5db9e30c").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("0081c95bc43384e663d79270c956ce3b8925b4f6d033b078b96384f50579400e").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("2ed5f0c91cbd9749187e2fade687e05ee2491b349c039a0bba8a9f4023a0bb38").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("30509991f88da3504bbf374ed5aae2f03448a22c76234c8c990f01f33a735206").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("1c3f20fd55409a53221b7c4d49a356b9f0a1119fb2067b41a7529094424ec6ad").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("10b4e7f3ab5df003049514459b6e18eec46bb2213e8e131e170887b47ddcb96c").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("2a1982979c3ff7f43ddd543d891c2abddd80f804c077d775039aa3502e43adef").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("1c74ee64f15e1db6feddbead56d6d55dba431ebc396c9af95cad0f1315bd5c91").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("07533ec850ba7f98eab9303cace01b4b9e4f2e8b82708cfa9c2fe45a0ae146a0").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("21576b438e500449a151e4eeaf17b154285c68f42d42c1808a11abf3764c0750").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("2f17c0559b8fe79608ad5ca193d62f10bce8384c815f0906743d6930836d4a9e").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("2d477e3862d07708a79e8aae946170bc9775a4201318474ae665b0b1b7e2730e").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("162f5243967064c390e095577984f291afba2266c38f5abcd89be0f5b2747eab").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("2b4cb233ede9ba48264ecd2c8ae50d1ad7a8596a87f29f8a7777a70092393311").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("2c8fbcb2dd8573dc1dbaf8f4622854776db2eece6d85c4cf4254e7c35e03b07a").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("1d6f347725e4816af2ff453f0cd56b199e1b61e9f601e9ade5e88db870949da9").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("204b0c397f4ebe71ebc2d8b3df5b913df9e6ac02b68d31324cd49af5c4565529").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("0c4cb9dc3c4fd8174f1149b3c63c3c2f9ecb827cd7dc25534ff8fb75bc79c502").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("174ad61a1448c899a25416474f4930301e5c49475279e0639a616ddc45bc7b54").unwrap()),
            Fr::from_le_bytes_mod_order(&hex::decode("1a96177bcf4d8d89f759df4ec2f3cde2eaaa28c177cc0fa13a9816d49a38d2ef").unwrap()),
        ];

        Poseidon {
            params: PoseidonParameters {
                mds_matrix,
                round_keys,
                full_rounds: 8,      // Matches PoseidonT3.sol
                partial_rounds: 48,  // Matches PoseidonT3.sol
                capacity,
                rate,
            },
        }
    }

    pub fn hash(&self, inputs: &[Fr]) -> Result<Fr, &'static str> {
        if inputs.len() != 2 {
            return Err("PoseidonT3 expects exactly 2 inputs");
        }

        // Initialize state with initial constants
        let mut state = vec![
            Fr::zero(),                    // state0
            inputs[0] + self.params.round_keys[0], // state1 = input0 + c0
            inputs[1] + self.params.round_keys[1], // state2 = input1 + c1
        ];
        let mut constant_idx = 2;

        let total_rounds = self.params.full_rounds + self.params.partial_rounds;

        for round in 0..total_rounds {
            // S-box (x^5)
            if round < self.params.full_rounds {
                // Full round: S-box on all elements
                for s in state.iter_mut() {
                    let square = s.square();
                    *s = square.square() * *s; // x^5
                }
            } else {
                // Partial round: S-box on state[0] only
                let square = state[0].square();
                state[0] = square.square() * state[0]; // state0^5
            }

            // Add round constant to state[0] only (matches PoseidonT3.sol)
            state[0] = state[0] + self.params.round_keys[constant_idx];
            constant_idx += 1;

            // MDS mixing
            let mut new_state = vec![Fr::zero(); 3];
            for i in 0..3 {
                for j in 0..3 {
                    new_state[i] += state[j] * self.params.mds_matrix[i][j];
                }
            }
            state = new_state;
        }

        Ok(state[0])
    }

    // Helper to get output as big-endian bytes
    pub fn hash_to_bytes(&self, inputs: &[Fr]) -> Result<Vec<u8>, &'static str> {
        let hash = self.hash(inputs)?;
        Ok(hash.into_bigint().to_bytes_be()) // Big-endian output
    }
}