# Beam Apps

Beam apps are Payy-controlled extension packages for `beam-cli`.

Source files under `beam-apps/apps/` are the source of truth. CI builds those
sources into static release artifacts under `beam-apps/registry-bundle/`, signs
the registry metadata, verifies the bundle, and then bakes the generated files
into the `beam-app-registry` Docker image. The registry bundle is generated
release output and is not checked into git.

The generated bundle serves two surfaces:

- Install artifacts consumed by Beam CLI: `index.json`, signed manifests, WASM
  modules, icon assets, and digest/signature metadata.
- Catalog data consumed by the Beam website: `catalog/apps.json` and
  `catalog/apps/<app>.json`, including display metadata, structured command
  docs, permission summaries, icon metadata, and README markdown.

The live registry is immutable static content for each image. Rollback is a
Kubernetes image rollback, not an in-place mutation of served registry files.

Local bundle build:

```bash
scripts/beam-app-registry/build.py
scripts/beam-app-registry/verify.py
```

Local registry server:

```bash
scripts/beam-app-registry/run-local.py
```

In another shell:

```bash
export BEAM_APP_REGISTRY_URL=http://127.0.0.1:8787
export BEAM_HOME="$(mktemp -d)"
cargo run -p beam-cli --bin beam -- apps install uniswap --dry-run
```

Set `BEAM_UNISWAP_PUBLIC_API_KEY` before starting `run-local.py` when testing
real Uniswap Trading API access. If it is unset, the local WASM embeds an empty
key, which is enough for registry install and permission testing.

Deployed registry smoke test:

```bash
scripts/beam-app-registry/smoke.py --base-url https://registry.beam.payy.network
```

Mainnet registry releases run through
`.github/workflows/beam-app-registry.release.mainnet.yml`. The workflow builds
the signed bundle, deploys the `beam-app-registry` Helm release, publishes the
`registry.beam.payy.network` Cloudflare `A` record to the GKE ingress IP, waits
for DNS plus the GKE `ManagedCertificate`, then runs the deployed smoke test.
The `CLOUDFLARE_API_TOKEN` GitHub secret must be allowed to edit DNS records in
the `payy.network` zone.

App source stays separate from Beam core. Product apps live under
`beam-apps/apps/<app>` and must not path-depend on `pkg/*` crates or inherit root
workspace dependencies. The Uniswap app is its own Rust workspace; CI installs
`wasm32-unknown-unknown`, injects the Payy-managed public Uniswap API key from
the `BEAM_UNISWAP_PUBLIC_API_KEY` GitHub secret, builds its release WASM,
verifies the generated registry bundle, and bakes only the signed static bundle
into the registry image.

Until a shared app SDK crate is published, product apps may vendor app-local host
ABI structs. Beam CLI remains the generic host/runtime and must not contain
product-specific app business logic.
