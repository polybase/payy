set -e

rm -rf target
rm -rf ./utxo/target
rm -rf ./agg_test/target

nargo compile
bb write_vk --oracle_hash keccak -b ./target/utxo.json -o ./utxo/target
bb write_vk --oracle_hash keccak -b ./target/agg_test.json -o ./agg_test/target

echo "Generating utxo proof"

nargo execute --package utxo
bb write_vk -b ./target/utxo.json -o ./utxo/target --output_format bytes_and_fields
bb prove -b ./target/utxo.json -w ./target/utxo.gz -k ./utxo/target/vk  --output_format bytes_and_fields  -o ./utxo/target
bb verify -k ./utxo/target/vk -p ./utxo/target/proof -i ./utxo/target/public_inputs


# Populate agg_test/Prover.toml with proof vk and public inputs
TOML_CONTENT="proof="
TOML_CONTENT+=$(cat ./utxo/target/proof_fields.json)
TOML_CONTENT+="\n\npublic_inputs="
TOML_CONTENT+=$(cat ./utxo/target/public_inputs_fields.json)
TOML_CONTENT+="\n\nverification_key="
TOML_CONTENT+=$(cat ./utxo/target/vk_fields.json)

rm -f ./agg_test/Prover.toml
echo "$TOML_CONTENT" > ./agg_test/Prover.toml


nargo execute --package agg_test
bb write_vk -b ./target/agg_test.json -o ./target --output_format bytes_and_fields
bb prove -b ./target/agg_test.json -w ./target/agg_test.gz -k ./target/vk  --output_format bytes_and_fields  -o ./target
bb verify -k ./target/vk -p ./target/proof -i ./target/public_inputs

echo "Done"
