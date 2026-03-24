#!/bin/bash
set -eu

BACKEND=${BACKEND:-bb}

PACKAGE_NAME="utxo"
RECURSION_PACKAGE_NAME="recursion"



nargo execute ${PACKAGE_NAME}_witness --package ${PACKAGE_NAME}

# Timing for the first proof
start_time=$(date +%s.%N)
time $BACKEND prove -b ./target/${PACKAGE_NAME}.json -w ./target/${PACKAGE_NAME}_witness.gz -o ./target/${PACKAGE_NAME}_proof --recursive
end_time=$(date +%s.%N)
duration=$(echo "$end_time - $start_time" | bc -l)
printf "Proof ${PACKAGE_NAME} time: %.1f seconds\n" "$duration"

# Generate inputs to recursion
$BACKEND write_vk -b ./target/${PACKAGE_NAME}.json -o ./target/${PACKAGE_NAME}_key --recursive
$BACKEND vk_as_fields -k ./target/${PACKAGE_NAME}_key -o ./target/${PACKAGE_NAME}_vk_as_fields
VK_HASH=$(jq -r '.[0]' ./target/${PACKAGE_NAME}_vk_as_fields)
VK_AS_FIELDS=$(jq -r '.[1:]' ./target/${PACKAGE_NAME}_vk_as_fields)

$BACKEND proof_as_fields -p ./target/${PACKAGE_NAME}_proof -k ./target/${PACKAGE_NAME}_key -o ./target/${PACKAGE_NAME}_proof_as_fields
FULL_PROOF_AS_FIELDS=$(jq -r '.[0:]' ./target/${PACKAGE_NAME}_proof_as_fields)
# echo $FULL_PROOF_AS_FIELDS

# Count the number of public inputs in Verifier.toml
PUBLIC_INPUT_COUNT=10;
echo "Public input count: $PUBLIC_INPUT_COUNT"

PUBLIC_INPUTS=$(echo $FULL_PROOF_AS_FIELDS | jq -r ".[:$PUBLIC_INPUT_COUNT]")
PROOF_AS_FIELDS=$(echo $FULL_PROOF_AS_FIELDS | jq -r ".[$PUBLIC_INPUT_COUNT:]")

RECURSE_LEAF_PROVER_TOML=./${RECURSION_PACKAGE_NAME}/Prover.toml

# Clear the file
> $RECURSE_LEAF_PROVER_TOML

echo "key_hash = \"$VK_HASH\"" >> $RECURSE_LEAF_PROVER_TOML
echo "verification_key = $VK_AS_FIELDS"  >> $RECURSE_LEAF_PROVER_TOML
echo "proof = $PROOF_AS_FIELDS" >> $RECURSE_LEAF_PROVER_TOML
echo "public_inputs = $PUBLIC_INPUTS" >> $RECURSE_LEAF_PROVER_TOML

nargo execute ${RECURSION_PACKAGE_NAME}_witness --package ${RECURSION_PACKAGE_NAME}

# Timing for the recursion proof
start_time=$(date +%s.%N)  # Corrected format
$BACKEND prove -b ./target/${RECURSION_PACKAGE_NAME}.json -w ./target/${RECURSION_PACKAGE_NAME}_witness.gz -o ./target/${RECURSION_PACKAGE_NAME}_proof
end_time=$(date +%s.%N)
duration=$(echo "$end_time - $start_time" | bc -l)
printf "Proof ${RECURSION_PACKAGE_NAME} time: %.1f seconds\n" "$duration"

# Verify the generated recursive proof
$BACKEND write_vk -b ./target/${-}.json -o ./target/${RECURSION_PACKAGE_NAME}_key
$BACKEND verify -p ./target/${RECURSION_PACKAGE_NAME}_proof -k ./target/${RECURSION_PACKAGE_NAME}_key
