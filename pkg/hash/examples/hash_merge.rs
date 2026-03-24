use element::Element;
use hash::hash_merge;
use std::{hint::black_box, time::Instant};

const ITERS: usize = 1_000_000;

fn main() {
    let mut element = Element::new(5);

    let start = Instant::now();

    for _ in 0..ITERS {
        let new = hash_merge([element, black_box(Element::new(3))]);
        element = black_box(new);
    }

    let time = start.elapsed();

    println!("{ITERS} hash merges took {time:?}");
}
