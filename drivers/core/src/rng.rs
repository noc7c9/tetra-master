use rand::{
    distributions::{uniform, Distribution, Standard},
    thread_rng, Rng as _, SeedableRng as _,
};

pub use rand::distributions::WeightedIndex;

pub type Seed = u64;

// Wrapper around a rand random number generator
// allowing use of the Rng methods without having to import any traits
pub struct Rng {
    pub initial_seed: Seed,
    gen: rand_pcg::Pcg32,
}

impl Default for Rng {
    fn default() -> Self {
        Self::new()
    }
}

impl Rng {
    pub fn new() -> Self {
        let seed = thread_rng().gen();
        Self::from_seed(seed)
    }

    pub fn from_seed(seed: Seed) -> Self {
        Self {
            initial_seed: seed,
            gen: rand_pcg::Pcg32::seed_from_u64(seed),
        }
    }

    pub fn gen<T>(&mut self) -> T
    where
        Standard: Distribution<T>,
    {
        self.gen.gen()
    }

    pub fn gen_range<T, R>(&mut self, range: R) -> T
    where
        T: uniform::SampleUniform,
        R: uniform::SampleRange<T>,
    {
        self.gen.gen_range(range)
    }

    pub fn sample<T, D>(&mut self, dist: D) -> T
    where
        D: Distribution<T>,
    {
        self.gen.sample(dist)
    }

    /// Helper static method that initializes a WeightedIndex distribution
    ///
    /// panics if the weights are empty, if any weight is < 0, or if the sum of the weights is 0
    pub fn weighted_index<X, I>(weights: I) -> WeightedIndex<X>
    where
        I: IntoIterator,
        I::Item: uniform::SampleBorrow<X>,
        X: Clone
            + Default
            + uniform::SampleUniform
            + std::cmp::PartialOrd
            + for<'a> std::ops::AddAssign<&'a X>,
    {
        WeightedIndex::new(weights).unwrap()
    }
}
