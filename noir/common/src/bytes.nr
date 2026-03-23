pub fn slice<let N: u32, let L: u32>(bytes: [u8; N], start: u32) -> [u8; L] {
    let end = start + L;
    assert(end <= N, "slice: slice end out of bounds");
    let mut result = [bytes[0]; L];
    for i in 0..L {
        result[i] = bytes[start + i];
    }
    result
}
