# Beam App Registry Fixtures

`scripts/beam-app-registry/build.py --fixtures` emits fixture bundles here:

- `valid` - installable Uniswap registry bundle.
- `invalid-digest` - index points at a module digest that does not match the
  bundled WASM artifact.
- `missing-fields` - app manifest omits a required field.
- `unsupported-beam` - app requires a future Beam version.
- `malformed-permissions` - app declares an invalid selector.
- `broad-wildcard` - app deliberately omits optional contract, selector,
  and spender scopes so Beam must display broad wildcard permissions.
