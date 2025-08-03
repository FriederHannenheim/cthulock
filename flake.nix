{
  description = "Wayland screen locker focused on customizability";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
    in
    flake-utils.lib.eachSystem supportedSystems (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        nativeBuildInputs = with pkgs; [
          rustPlatform.bindgenHook
          pkg-config
        ];
        buildInputs = with pkgs; [
          libclang
          libxkbcommon
          linux-pam
          libGL
          wayland
          makeWrapper
          fontconfig
        ];
      in rec
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "cthulock";
          version = "0.1.0-git";
          src = self;

          inherit buildInputs nativeBuildInputs;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          LD_LIBRARY_PATH = "${pkgs.libGL}/lib";
          cargoBuildType = "debug";
          cargoCheckType = "debug";

          dontStrip = true;

          postInstall = ''
            wrapProgram $out/bin/cthulock --prefix LD_LIBRARY_PATH : "${
                pkgs.lib.makeLibraryPath [
                  pkgs.wayland
                  pkgs.libGL
                  pkgs.fontconfig
                ]
              }"
          '';
        };
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          packages = with pkgs; [
            clippy
            cargo
            rustc
            rust-analyzer
            rustfmt
            labwc
          ];
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
        };
        nixosModules.default = {
          config,
          pkgs,
          lib,
          ...
        }:
        let
          cfg = config.programs.cthulock;
        in
        {
          options.programs.cthulock.enable = lib.mkEnableOption "Installs Cthulock and creates a pam service for it";
          config = lib.mkIf cfg.enable {
            environment.systemPackages = [ packages.default ];
            security.pam.services."cthulock" = {};
          };
        };
      }
    );
}
