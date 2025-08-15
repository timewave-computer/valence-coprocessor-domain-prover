{
  description = "Valence coprocessor domain prover";

  nixConfig.extra-experimental-features = "nix-command flakes";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    
    flake-parts.url = "github:hercules-ci/flake-parts";
    fp-addons.url = "github:timewave-computer/flake-parts-addons";

    devshell.url = "github:numtide/devshell";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    sp1-nix.url = "github:timewave-computer/sp1.nix";
    crate2nix.url = "github:timewave-computer/crate2nix";
  };

  outputs = inputs@{ self, flake-parts, ... }:
    flake-parts.lib.mkFlake {inherit inputs;} ({moduleWithSystem, ...}: {
      imports = [
        inputs.devshell.flakeModule
        inputs.crate2nix.flakeModule
        inputs.fp-addons.flakeModules.tools
      ];

      systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];

      perSystem = {
        lib,
        config,
        inputs',
        pkgs,
        system,
        ...
      }: {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [ inputs.rust-overlay.overlays.default ];
        };

        crate2nix = {
          cargoNix = ./Cargo.nix;
          devshell.name = "default";
          toolchain = {
            rust = pkgs.rust-bin.nightly.latest.default;
            cargo = pkgs.rust-bin.nightly.latest.default;
          };
          defaultOverride = attrs: {
            includePaths = [ ./elf ];
            meta.mainProgram = attrs.crateName;
          };
          inherit (inputs'.sp1-nix.tools) crateOverrides;
        };

        packages = {
          service = config.crate2nix.packages.valence-coprocessor-domain-prover-service;
        };

        checks = {
          service = config.crate2nix.checks.valence-coprocessor-domain-prover-service;
        };

        devshells.default = {
          packages = with pkgs; [
            curl
            jq
            clang
            taplo
            toml-cli
            lld
          ];
          
          env = [
            {
              name = "OPENSSL_DIR";
              value = "${pkgs.lib.getDev pkgs.openssl}";
            }
            {
              name = "OPENSSL_LIB_DIR";
              value = "${pkgs.lib.getLib pkgs.openssl}/lib";
            }
            {
              name = "LIBCLANG_PATH";
              value = pkgs.lib.makeLibraryPath [ pkgs.libclang ];
            }
          ];
          
        };
      };

      flake.nixosModules.service = moduleWithSystem (
        { self', ... }:
        { lib, config, ...}:
        let
          cfg = config.services.valence-coprocessor.domain-prover;
        in
        {
          options = {
            services.valence-coprocessor.domain-prover = {
              package = lib.mkOption {
                type = lib.types.package;
                default = self'.packages.service;
              };
              flags = lib.mkOption {
                type = lib.types.listOf (lib.types.str);
                default = [];
              };
            };
          };
          config = {
            systemd.services = {
              valence-coprocessor-domain-prover = {
                enable = true;
                serviceConfig = {
                  Type = "simple";
                  DynamicUser = true;
                  StateDirectory = "valence-coprocessor-domain-prover";
                  ExecStart = "${lib.getExe cfg.package} ${lib.escapeShellArgs cfg.flags}";
                };
                wantedBy = [ "multi-user.target" ];
              };
            };
          };
        }
      );
    });
}
