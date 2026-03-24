// lint-long-file-override allow-max-lines=400
use std::convert::TryInto;
use std::fmt;
use std::iter;
use std::marker::PhantomData;

use ff::FromUniformBytes; // Used by Spec trait bound

// --- Paste relevant content from poseidon-base/src/primitives.rs ---
// (State, SpongeRate, Mds types; Spec, CachedSpec traits; permute fn;
//  SpongeMode, Absorbing, Squeezing; Sponge struct and impls;
//  Domain trait, ConstantLength; Hash struct and impls)

/// The type used to hold permutation state.
pub type State<F, const T: usize> = [F; T];
/// The type used to hold sponge rate.
pub type SpongeRate<F, const RATE: usize> = [Option<F>; RATE];
/// The type used to hold the MDS matrix and its inverse.
pub type Mds<F, const T: usize> = [[F; T]; T];

/// A specification for a Poseidon permutation.
pub trait Spec<F: FromUniformBytes<64> + Ord, const T: usize, const RATE: usize>:
    Copy + fmt::Debug
{
    fn full_rounds() -> usize;
    fn partial_rounds() -> usize;
    fn sbox(val: F) -> F;
    fn secure_mds() -> usize; // May not be used if constants() is overridden
    fn constants() -> (Vec<[F; T]>, Mds<F, T>, Mds<F, T>);
    // Default impl of constants() using grain might be removed if not needed,
    // as P128Pow5T3 overrides it. For now, keep it for completeness or remove if it pulls `grain`.
    // If kept, `grain.rs` and `mds.rs` (generators) would be needed.
    // For this minimal version, we assume P128Pow5T3's override is sufficient.
}

pub trait CachedSpec<F: FromUniformBytes<64> + Ord, const T: usize, const RATE: usize>:
    Spec<F, T, RATE>
{
    fn cached_round_constants() -> &'static [[F; T]];
    fn cached_mds() -> &'static Mds<F, T>;
    fn cached_mds_inv() -> &'static Mds<F, T>;
}

pub fn permute<
    F: FromUniformBytes<64> + Ord,
    S: Spec<F, T, RATE>,
    const T: usize,
    const RATE: usize,
>(
    state: &mut State<F, T>,
    mds: &Mds<F, T>,
    round_constants: &[[F; T]],
) {
    let r_f = S::full_rounds() / 2;
    let r_p = S::partial_rounds();

    let apply_mds = |state: &mut State<F, T>| {
        let mut new_state = [F::ZERO; T];
        #[allow(clippy::needless_range_loop)]
        for i in 0..T {
            for j in 0..T {
                new_state[i] += mds[i][j] * state[j];
            }
        }
        *state = new_state;
    };

    let full_round = |state: &mut State<F, T>, rcs: &[F; T]| {
        for (word, rc) in state.iter_mut().zip(rcs.iter()) {
            *word = S::sbox(*word + rc);
        }
        apply_mds(state);
    };

    let part_round = |state: &mut State<F, T>, rcs: &[F; T]| {
        for (word, rc) in state.iter_mut().zip(rcs.iter()) {
            *word += rc;
        }
        state[0] = S::sbox(state[0]);
        apply_mds(state);
    };

    iter::empty()
        .chain(std::iter::repeat_n(
            &full_round as &dyn Fn(&mut State<F, T>, &[F; T]),
            r_f,
        ))
        .chain(std::iter::repeat_n(
            &part_round as &dyn Fn(&mut State<F, T>, &[F; T]),
            r_p,
        ))
        .chain(std::iter::repeat_n(
            &full_round as &dyn Fn(&mut State<F, T>, &[F; T]),
            r_f,
        ))
        .zip(round_constants.iter())
        .fold(state, |state, (round, rcs)| {
            round(state, rcs);
            state
        });
}

fn poseidon_sponge_internal<
    // Renamed to avoid conflict if original poseidon_sponge is kept
    F: FromUniformBytes<64> + Ord,
    S: CachedSpec<F, T, RATE>, // Changed Spec to CachedSpec
    const T: usize,
    const RATE: usize,
>(
    state: &mut State<F, T>,
    input: Option<(&Absorbing<F, RATE>, usize)>,
    // mds_matrix and round_constants are now fetched from CachedSpec
) -> Squeezing<F, RATE> {
    if let Some((Absorbing(input_data), layout_offset)) = input {
        assert!(layout_offset <= T - RATE);
        for (word, value) in state.iter_mut().skip(layout_offset).zip(input_data.iter()) {
            *word += value.expect("poseidon_sponge is called with a padded input");
        }
    }

    permute::<F, S, T, RATE>(state, S::cached_mds(), S::cached_round_constants());

    let mut output = [None; RATE];
    for (word, value) in output.iter_mut().zip(state.iter()) {
        *word = Some(*value);
    }
    Squeezing(output)
}

mod private_sponge {
    // Renamed to avoid conflict
    pub trait SealedSpongeMode {}
    impl<F, const RATE: usize> SealedSpongeMode for super::Absorbing<F, RATE> {}
    impl<F, const RATE: usize> SealedSpongeMode for super::Squeezing<F, RATE> {}
}

pub trait SpongeMode: private_sponge::SealedSpongeMode + Clone {}

#[derive(Debug, Copy, Clone)]
pub struct Absorbing<F, const RATE: usize>(pub SpongeRate<F, RATE>);
#[derive(Debug, Copy, Clone)]
pub struct Squeezing<F, const RATE: usize>(pub SpongeRate<F, RATE>);

impl<F: Clone, const RATE: usize> SpongeMode for Absorbing<F, RATE> {}
impl<F: Clone, const RATE: usize> SpongeMode for Squeezing<F, RATE> {}

impl<F: fmt::Debug, const RATE: usize> Absorbing<F, RATE> {
    pub fn init_with(val: F) -> Self {
        Self(
            iter::once(Some(val))
                .chain((1..RATE).map(|_| None))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        )
    }
}

#[derive(Clone)]
pub(crate) struct Sponge<
    F: FromUniformBytes<64> + Ord,
    S: CachedSpec<F, T, RATE>,
    M: SpongeMode,
    const T: usize,
    const RATE: usize,
> {
    mode: M,
    state: State<F, T>,
    layout: usize,
    _marker: PhantomData<S>,
}

impl<F: FromUniformBytes<64> + Ord, S: CachedSpec<F, T, RATE>, const T: usize, const RATE: usize>
    Sponge<F, S, Absorbing<F, RATE>, T, RATE>
{
    pub(crate) fn new(initial_capacity_element: F, layout: usize) -> Self {
        let mode = Absorbing([None; RATE]);
        let mut state = [F::ZERO; T];
        state[(RATE + layout) % T] = initial_capacity_element;

        Sponge {
            mode,
            state,
            layout,
            _marker: PhantomData,
        }
    }

    pub(crate) fn absorb(&mut self, value: F) {
        for entry in self.mode.0.iter_mut() {
            if entry.is_none() {
                *entry = Some(value);
                return;
            }
        }
        let _ = poseidon_sponge_internal::<F, S, T, RATE>(
            &mut self.state,
            Some((&self.mode, self.layout)),
        );
        self.mode = Absorbing::init_with(value);
    }

    pub(crate) fn finish_absorbing(mut self) -> Sponge<F, S, Squeezing<F, RATE>, T, RATE> {
        let mode = poseidon_sponge_internal::<F, S, T, RATE>(
            &mut self.state,
            Some((&self.mode, self.layout)),
        );
        Sponge {
            mode,
            state: self.state,
            layout: self.layout,
            _marker: PhantomData,
        }
    }
}

impl<F: FromUniformBytes<64> + Ord, S: CachedSpec<F, T, RATE>, const T: usize, const RATE: usize>
    Sponge<F, S, Squeezing<F, RATE>, T, RATE>
{
    pub(crate) fn squeeze(&mut self) -> F {
        loop {
            for entry in self.mode.0.iter_mut() {
                if let Some(e) = entry.take() {
                    return e;
                }
            }
            self.mode = poseidon_sponge_internal::<F, S, T, RATE>(&mut self.state, None);
        }
    }
}

pub trait Domain<F: FromUniformBytes<64> + Ord, const RATE: usize> {
    type Padding: IntoIterator<Item = F>;
    fn name() -> String;
    fn initial_capacity_element() -> F;
    fn padding(input_len: usize) -> Self::Padding;
    fn layout(_width: usize) -> usize {
        0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ConstantLength<const L: usize>;

impl<F: FromUniformBytes<64> + Ord, const RATE: usize, const L: usize> Domain<F, RATE>
    for ConstantLength<L>
{
    type Padding = iter::RepeatN<F>;
    fn name() -> String {
        format!("ConstantLength<{L}>")
    }
    fn initial_capacity_element() -> F {
        F::from_u128((L as u128) << 64)
    }
    fn padding(input_len: usize) -> Self::Padding {
        assert_eq!(input_len, L);
        let k = L.div_ceil(RATE);
        std::iter::repeat_n(F::ZERO, k * RATE - L)
    }
}

#[derive(Clone)]
pub struct Hash<
    F: FromUniformBytes<64> + Ord,
    S: CachedSpec<F, T, RATE>,
    D: Domain<F, RATE>,
    const T: usize,
    const RATE: usize,
> {
    sponge: Sponge<F, S, Absorbing<F, RATE>, T, RATE>,
    _domain: PhantomData<D>,
}

impl<
    F: FromUniformBytes<64> + Ord,
    S: CachedSpec<F, T, RATE>,
    D: Domain<F, RATE>,
    const T: usize,
    const RATE: usize,
> fmt::Debug for Hash<F, S, D, T, RATE>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hash")
            .field("width", &T)
            .field("rate", &RATE)
            .field("R_F", &S::full_rounds())
            .field("R_P", &S::partial_rounds())
            .field("domain", &D::name())
            .finish()
    }
}

impl<
    F: FromUniformBytes<64> + Ord,
    S: CachedSpec<F, T, RATE>,
    D: Domain<F, RATE>,
    const T: usize,
    const RATE: usize,
> Hash<F, S, D, T, RATE>
{
    pub fn init() -> Self {
        Hash {
            sponge: Sponge::new(D::initial_capacity_element(), D::layout(T)),
            _domain: PhantomData,
        }
    }
}

impl<
    F: FromUniformBytes<64> + Ord,
    S: CachedSpec<F, T, RATE>,
    const T: usize,
    const RATE: usize,
    const L: usize,
> Hash<F, S, ConstantLength<L>, T, RATE>
{
    pub fn hash(mut self, message: [F; L]) -> F {
        for value in message
            .into_iter()
            .chain(<ConstantLength<L> as Domain<F, RATE>>::padding(L))
        {
            self.sponge.absorb(value);
        }
        self.sponge.finish_absorbing().squeeze()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Bn256Fr as Fp, P128Pow5T3};

    use super::{ConstantLength, Hash, Spec, permute};
    type OrchardNullifier = P128Pow5T3<Fp>;

    #[test]
    fn orchard_spec_equivalence() {
        let message = [Fp::from(6), Fp::from(42)];

        let (round_constants, mds, _) = OrchardNullifier::constants();

        let hasher = Hash::<_, OrchardNullifier, ConstantLength<2>, 3, 2>::init();
        let result = hasher.hash(message);

        // The result should be equivalent to just directly applying the permutation and
        // taking the first state element as the output.
        let mut two_to_sixty_five = Fp::from(1 << 63);
        two_to_sixty_five = two_to_sixty_five.double();
        two_to_sixty_five = two_to_sixty_five.double();
        let mut state = [message[0], message[1], two_to_sixty_five];
        permute::<_, OrchardNullifier, 3, 2>(&mut state, &mds, &round_constants);
        assert_eq!(state[0], result);
    }
}
