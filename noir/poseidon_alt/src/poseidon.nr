use crate::constants::{FULL_ROUNDS, PARTIAL_ROUNDS, RATE, ROUND_CONSTANTS, MDS};

// S-box function (x^5)
fn sbox(x: Field) -> Field {
    let x2 = x * x;
    let x4 = x2 * x2;
    x4 * x
}

// Apply MDS matrix multiplication
fn apply_mds(state: [Field; 3]) -> [Field; 3] {
    let mut result = [0; 3];
    for i in 0..3 {
        for j in 0..3 {
            result[i] = result[i] + MDS[i][j] * state[j];
        }
    }
    result
}

// Full round function: S-box on all elements + MDS
fn full_round(mut state: [Field; 3], round_constants: [Field; 3]) -> [Field; 3] {
    // Add round constants and apply S-box to all elements
    for i in 0..3 {
        state[i] = sbox(state[i] + round_constants[i]);
    }
    // Apply MDS matrix
    apply_mds(state)
}

// Partial round function: S-box on first element only + MDS
fn partial_round(mut state: [Field; 3], round_constants: [Field; 3]) -> [Field; 3] {
    // Add round constants
    for i in 0..3 {
        state[i] = state[i] + round_constants[i];
    }
    // Apply S-box only to first element
    state[0] = sbox(state[0]);
    // Apply MDS matrix
    apply_mds(state)
}

// Main permutation function
pub fn permute(mut state: [Field; 3]) -> [Field; 3] {
    let r_f = FULL_ROUNDS / 2;
    let r_p = PARTIAL_ROUNDS;
    
    let mut round_idx = 0;
    
    // First half of full rounds
    for _ in 0..r_f {
        state = full_round(state, ROUND_CONSTANTS[round_idx]);
        round_idx = round_idx + 1;
    }
    
    // Partial rounds
    for _ in 0..r_p {
        state = partial_round(state, ROUND_CONSTANTS[round_idx]);
        round_idx = round_idx + 1;
    }
    
    // Second half of full rounds
    for _ in 0..r_f {
        state = full_round(state, ROUND_CONSTANTS[round_idx]);
        round_idx = round_idx + 1;
    }
    
    state
}

// Sponge construction for constant-length hashing
pub struct PoseidonSponge {
    state: [Field; 3],
    cache: [Field; 2],
    cache_size: u32,
}

impl PoseidonSponge {
    // Initialize with initial capacity element for constant-length domain separation
    pub fn new(initial_capacity: Field) -> Self {
        let mut state = [0; 3];
        state[2] = initial_capacity; // Set capacity element at index RATE (2)
        Self {
            state,
            cache: [0; 2],
            cache_size: 0,
        }
    }
    
    // Absorb a field element
    pub fn absorb(&mut self, input: Field) {
        if self.cache_size == RATE {
            // Cache is full, perform duplex round
            self.perform_duplex();
            self.cache[0] = input;
            self.cache_size = 1;
        } else {
            // Add to cache
            self.cache[self.cache_size] = input;
            self.cache_size = self.cache_size + 1;
        }
    }
    
    // Perform duplex operation (add cache to state and permute)
    fn perform_duplex(&mut self) {
        // Add cached values to state
        for i in 0..RATE {
            if i < self.cache_size {
                self.state[i] = self.state[i] + self.cache[i];
            }
        }
        // Apply permutation
        self.state = permute(self.state);
        // Clear cache
        self.cache_size = 0;
    }
    
    // Squeeze output
    pub fn squeeze(mut self) -> Field {
        // Final duplex to compress any remaining cached data
        self.perform_duplex();
        // Return first element
        self.state[0]
    }
}

// Hash function for 2 elements (matching the Rust implementation)
pub fn hash_2(left: Field, right: Field) -> Field {
    // Initial capacity element: (2 << 64) for ConstantLength<2>
    let initial_capacity = 2 * 0x10000000000000000; // 2 * 2^64
    
    let mut sponge = PoseidonSponge::new(initial_capacity);
    sponge.absorb(left);
    sponge.absorb(right);
    sponge.squeeze()
}

// Generic hash function for N elements
pub fn hash<let N: u32>(inputs: [Field; N]) -> Field {
    // Initial capacity element: (N << 64) for ConstantLength<N>
    let initial_capacity = (N as Field) * 0x10000000000000000; // N * 2^64
    
    let mut sponge = PoseidonSponge::new(initial_capacity);
    for i in 0..N {
        sponge.absorb(inputs[i]);
    }
    
    // Add padding (zero padding) to make total absorbed elements a multiple of RATE
    // Calculate how many zeros to add
    let k = (N + RATE - 1) / RATE; // Ceiling division
    let padding_count = k * RATE - N;
    for _ in 0..padding_count {
        sponge.absorb(0);
    }
    
    sponge.squeeze()
}