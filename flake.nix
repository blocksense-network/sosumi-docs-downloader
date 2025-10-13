{
  description = "sosumi-docs-downloader";

  inputs = {
    nixos-modules.url = "github:metacraft-labs/nixos-modules";
    nixpkgs.follows = "nixos-modules/nixpkgs-unstable";
    flake-parts.follows = "nixos-modules/flake-parts";
    fenix.follows = "nixos-modules/fenix";
    crane.follows = "nixos-modules/crane";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  outputs = inputs: inputs.flake-parts.lib.mkFlake { inherit inputs; } {
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];

    perSystem = { inputs', self', lib, pkgs, system, ... }: let
      rustToolchain = inputs'.fenix.packages.stable.toolchain;
      craneLib = (inputs.crane.mkLib pkgs).overrideToolchain inputs.fenix.packages.${system}.stable.toolchain;
    in {
      _module.args.pkgs = import inputs.nixpkgs {
        inherit system;
        overlays = [];
      };

      checks = let
        preCommit = inputs.git-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            # Rust formatting and linting
            rustfmt = {
              enable = true;
              name = "rustfmt";
              entry = "rustfmt --edition 2021";
              language = "system";
              pass_filenames = true;
              files = "\\.rs$";
            };
            clippy = {
              enable = true;
              name = "clippy";
              entry = "cargo clippy -- -D warnings";
              language = "system";
              pass_filenames = false;
            };

            # TOML formatting
            taplo-fmt = {
              enable = true;
              name = "taplo fmt";
              entry = "taplo fmt";
              language = "system";
              pass_filenames = true;
              files = "\\.toml$";
            };
          };
          tools = {
            rustfmt = pkgs.rustfmt;
            taplo = pkgs.taplo;
          };
        };
      in {
        pre-commit-check = preCommit;
      };

      packages = let
      in {
        sosumi-docs-downloader = craneLib.buildPackage {
          src = ./.;

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.gcc
            # pkgs.stdenv.cc.cc
            # pkgs.stdenv.cc.bintools
          ];

          meta = {
            description = "Downloader for Sosumi documentation";
            license = lib.licenses.mit;
            platforms = lib.platforms.unix;
            mainProgram = "sosumi-docs-downloader";
          };
        };
        default = self'.packages.sosumi-docs-downloader;
      };

      apps = {
        sosumi-docs-downloader = {
          type = "app";
          program = lib.getExe self'.packages.sosumi-docs-downloader;
        };
        default = self'.apps.sosumi-docs-downloader;
      };

      devShells = let
        packages = [
          # TODO: Rust toolchain
          # rustPlatform.toolchain

          # Basic development tools
          pkgs.git
          pkgs.just
          pkgs.taplo

          # Cargo tools
          pkgs.cargo-outdated
          pkgs.cargo-watch
        ];
      in {
        default = pkgs.mkShell {
          buildInputs = packages ++ self'.checks.pre-commit-check.enabledPackages;

          shellHook = ''
            # Install git pre-commit hook invoking our Nix-defined hooks
            ${self'.checks.pre-commit-check.shellHook}
            echo "Sosumi docs downloader development environment loaded"
          '';
        };
      };
    };
  };
}
