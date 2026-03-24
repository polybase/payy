{
  description = "Basic devshell for polybase";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustToolchain = pkgs.rust-bin.nightly."2023-01-10".default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        deployContracts = pkgs.writeShellScriptBin "deploy-scripts.sh" ''
          (cd eth && yarn --silent deploy:tests > ../pkg/node/tests/.env.test)
        '';

        run-node = pkgs.writeShellScriptBin "run-node.sh" ''
          (cd eth && npx hardhat node)
        '';

      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain

            pkg-config
            fontconfig
            openssl

            go
            nodejs
            python3
            yarn

            protobuf

            clang # required for rocksdb

            cargo-insta
            deployContracts
            run-node

          ];

          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib/";
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          ROLLUP_CONTRACT_ADDR="0xdc64a140aa3e981100a9beca4e685f962f0cf6c9";
          PROTOC = "${pkgs.protobuf}/bin/protoc";
        };
      });
}
