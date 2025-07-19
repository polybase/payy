set -e

cd utxo
rm -rf target
nargo compile
bb write_vk --oracle_hash keccak -b ./target/utxo.json -o ./target
cd ..

cd agg_test
rm -rf target
nargo compile
bb write_vk --oracle_hash keccak -b ./target/agg_test.json -o ./target
cd ..

echo "Generating utxo proof"

cd utxo
nargo execute
bb write_vk -b ./target/utxo.json -o ./target --output_format bytes_and_fields
bb prove -b ./target/utxo.json -w ./target/utxo.gz -k ./target/vk  --output_format bytes_and_fields  -o ./target
bb verify -k ./target/vk -p ./target/proof -i ./target/public_inputs
cd ..


cd agg_test
# Populate agg_test/Prover.toml with proof vk and public inputs
TOML_CONTENT="proof="
TOML_CONTENT+=$(cat ../utxo/target/proof_fields.json)
TOML_CONTENT+="\n\npublic_inputs="
TOML_CONTENT+=$(cat ../utxo/target/public_inputs_fields.json)
TOML_CONTENT+="\n\nverification_key="
TOML_CONTENT+=$(cat ../utxo/target/vk_fields.json)

rm -f Prover.toml
echo "$TOML_CONTENT" > Prover.toml


nargo execute
bb write_vk -b ./target/agg_test.json -o ./target --output_format bytes_and_fields
bb prove -b ./target/agg_test.json -w ./target/agg_test.gz -k ./target/vk  --output_format bytes_and_fields  -o ./target
bb verify -k ./target/vk -p ./target/proof -i ./target/public_inputs
cd ..

echo "Done"
