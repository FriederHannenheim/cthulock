{
  description = "Wayland screen locker focused on customizability";
  inputs.nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forEachSupportedSystem =
        f:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            pkgs = import nixpkgs { inherit system; };
          }
        );

    in
    {
      packages = forEachSupportedSystem (
        { pkgs }:
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "cthulock";
            version = "0.1.0-git";
            src = self;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
            nativeBuildInputs = with pkgs; [
              rustPlatform.bindgenHook
              pkg-config
            ];
            buildInputs = with pkgs; [
              libxkbcommon
              pam
              libGL
            ];
            LD_LIBRARY_PATH = "${pkgs.libGL}/lib";
            cargoBuildType = "debug";
            cargoCheckType = "debug";

            dontStrip = true;
          };
        }
      );
      devShells = forEachSupportedSystem (
        { pkgs }:
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              pkg-config
              libxkbcommon

              cargo
              rustc
              rust-analyzer
              rustfmt
              labwc
            ];
          };
        }
      );
    };
}
