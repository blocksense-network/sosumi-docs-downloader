{
  description = "sosumi-docs-downloader";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    git-hooks,
  }: let
    systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forAllSystems = nixpkgs.lib.genAttrs systems;
  in {
    checks = forAllSystems (system: let
      pkgs = import nixpkgs { inherit system; };
      preCommit = git-hooks.lib.${system}.run {
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
    });

    packages = forAllSystems (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        sosumi-downloader = pkgs.rustPlatform.buildRustPackage {
          pname = "sosumi-docs-downloader";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          buildInputs = [];
          nativeBuildInputs = [];
        };
      in {
        sosumi-docs-downloader = sosumi-downloader;
        default = sosumi-downloader;
      }
    );

    apps = forAllSystems (system: {
      sosumi-docs-downloader = {
        type = "app";
        program = "${self.packages.${system}.sosumi-docs-downloader}/bin/sosumi-docs-downloader";
      };
      default = self.apps.${system}.sosumi-docs-downloader;
    });

    devShells = forAllSystems (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };

      packages = [
        # Rust toolchain
        (pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rustfmt" "clippy"];
        })

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
        buildInputs = packages ++ self.checks.${system}.pre-commit-check.enabledPackages;

        shellHook = ''
          # Install git pre-commit hook invoking our Nix-defined hooks
          ${self.checks.${system}.pre-commit-check.shellHook}
          echo "Sosumi docs downloader development environment loaded"
        '';
      };
    });
  };
}