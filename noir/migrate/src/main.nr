use common::get_address;
use poseidon_alt::hash;

fn main(owner_pk: Field, old_address: pub Field, new_address: pub Field) {
    let calc_new_address = get_address(owner_pk);
    assert(calc_new_address == new_address, "New address is not owned by the owner");

    let calc_old_address = get_old_address(owner_pk);
    assert(calc_old_address == old_address, "Old address is not owned by the owner");
}

fn get_old_address(owner_pk: Field) -> Field {
    hash([owner_pk, 0])
}

#[test]
fn test_main() {
    let pk = 101;
    let new_address = get_address(pk);
    let old_address = get_old_address(pk);

    println(new_address);
    println(old_address);

    main(pk, old_address, new_address)
}
