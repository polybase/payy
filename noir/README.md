# Noir Circuits

This directory contains the Noir circuits for the Payy Network.

## Updating Circuits

When updating circuits, you must perform the following steps:

1. Run `./generate_fixtures.sh` to generate the new fixtures for the circuits.

   The script downloads the required `solc` binary into `~/.polybase/solc/` if it is missing.

2. Update the circuit in `pkg/barretenberg/src/circuits/`to handle new circuit inputs

3. Update `pkg/zk-primitives/src/<circuit_name>.rs` to circuit data types


## Installing Noir

Install Noir (`noirup` and `nargo`):

```bash
curl -L https://raw.githubusercontent.com/noir-lang/noirup/refs/heads/main/install | bash
noirup
```

Install the specific `nargo` version, `1.0.0-beta.14`:

```bash
 noirup -v 1.0.0-beta.14
```

Confirm it:

```bash
$ nargo --version
nargo version = 1.0.0-beta.14
noirc version = 1.0.0-beta.14+60ccd48e18ad8ce50d5ecda9baf813b712145051
(git version hash: 60ccd48e18ad8ce50d5ecda9baf813b712145051, is dirty: false)
```

Install proving backend (`bbup` `bb`):

```bash
curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/refs/heads/master/barretenberg/bbup/install | bash
bbup
```

See [Noir docs](https://noir-lang.org/docs) for more information.

## Install specific bb version

```sh
bbup -v 3.0.0-manual.20251030
```

Confirm it:

```bash
$ bb --version
3.0.0-manual.20251030
```

## Testing proof generation manually using CLI (non-recursive)

Assumes `bb` version: `v3.0.0-manual.20251030`.

To test proof generation manually, you can run the following steps:

```bash
# set to the package name you want to test
PACKAGE_NAME=utxo
```

### 1. Compile the circuit

```bash
nargo compile --package $PACKAGE_NAME
```

(will output a compiled program in `target/$PACKAGE_NAME.json`)

### 2. Generate the witness

Relies on the Prover.toml file, with valid inputs.

```bash
nargo execute --package $PACKAGE_NAME
```

(will output a witness file in `target/${PACKAGE_NAME}.gz`)

### 3. Generate the proof

```bash
bb prove --scheme ultra_honk -b target/${PACKAGE_NAME}.json -w target/${PACKAGE_NAME} -o target
```

(will output a proof file in `target/${PACKAGE_NAME}_proof`)

### 4. Verify the proof

Generate the verification key (non-recursive):

```bash
bb  write_vk --scheme ultra_honk -b target/${PACKAGE_NAME}.json -o target
```

To generate as fields use the `--output_format fields` flag.

Generate the verification key (recursive):

```bash
bb  write_vk --scheme ultra_honk --honk_recursion 1 --init_kzg_accumulator -b target/${PACKAGE_NAME}.json -o target
```

Verify the proof:

```bash
bb verify --scheme ultra_honk  -p target/proof -k target/vk -v
```

Check the verified output is: `verified: 1`


## Troubleshooting

  1. Ensure you have written a test within Noir and tested with `Prover.toml` inputs

  1. Re-run `./generate_fixtures.sh` if you have made any changes to the circuits

  2. Ensure the correct number of public inputs is being set in the circuit Prove trait

  3. Ensure the deconstruction and reconstruction of public inputs and proof is valid

  4. Check the order of the public inputs matches the noir main.nr file
