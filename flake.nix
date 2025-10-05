{
  description = "swwws - Slideshow daemon for swww with automated wallpaper cycling and multi-monitor support";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      inherit (cargoToml.workspace.package) rust-version version;
      
      rustPlatform = pkgs.makeRustPlatform {
        cargo = pkgs.rust-bin.stable.${rust-version}.default;
        rustc = pkgs.rust-bin.stable.${rust-version}.default;
      };
    in {
      packages = {
        swwws = rustPlatform.buildRustPackage {
          pname = "swwws";
          inherit version;

          src = pkgs.lib.cleanSource ./.;

          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          # Skip tests in sandboxed build environment
          doCheck = false;

          meta = with pkgs.lib; {
            description = "Slideshow daemon for swww with automated wallpaper cycling and multi-monitor support";
            homepage = "https://github.com/Mjoyufull/swwws";
            license = licenses.mit;
            platforms = platforms.linux;
            mainProgram = "swwws-daemon";
          };
        };

        default = self.packages.${system}.swwws;
      };
    });
}
