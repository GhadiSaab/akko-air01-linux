{
  description = "Native Rust Linux configurator for the Akko Air 01";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
    in {
      packages = eachSystem (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "air01";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          postInstall = ''
            install -Dm644 packaging/air01.desktop $out/share/applications/air01.desktop
            install -Dm644 udev/70-air01.rules $out/lib/udev/rules.d/70-air01.rules
          '';
          meta = {
            description = "Akko Air 01 configuration CLI and terminal interface";
            license = pkgs.lib.licenses.gpl3Plus;
            platforms = systems;
            mainProgram = "air01";
          };
        };
      });
      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [ cargo rustc rustfmt clippy ];
        };
      });
      checks = eachSystem (pkgs: {
        build = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      });
    };
}
