use common::get_address;
use poseidon::poseidon2;

fn main(owner_pk: Field, message_hash: Field, address: pub Field, message: pub Field) {
    let owner_address = get_address(owner_pk);
    assert(owner_address == address, "Address is not owned by the owner");

    let calc_message_hash = poseidon2::Poseidon2::hash([owner_pk, message], 2);
    assert(calc_message_hash == message_hash, "Message hash is invalid");
}

#[test]
fn test_main() {
    let pk = 101;
    let address = get_address(pk);
    let message = 1000;
    let message_hash = poseidon2::Poseidon2::hash([pk, message], 2);

    println(message_hash);
    println(address);

    main(pk, message_hash, address, message)
}
