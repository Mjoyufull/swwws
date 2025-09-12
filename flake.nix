{
  description = "SWWWS - Simple Wayland Wallpaper Slideshow";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    ...
  }: let
    inherit (nixpkgs) lib;
    systems = [
      "x86_64-linux"
    ];
    pkgsFor = lib.genAttrs systems (system:
      import nixpkgs {
        localSystem.system = system;
        overlays = [(import rust-overlay)];
      });
    cargoToml = lib.importTOML ./Cargo.toml;
    inherit (cargoToml.workspace.package) rust-version;
  in {
    packages =
      lib.mapAttrs (system: pkgs: {
        swwws = let
          rust = pkgs.rust-bin.stable.${rust-version}.default;

          rustPlatform = pkgs.makeRustPlatform {
            cargo = rust;
            rustc = rust;
          };
        in
          rustPlatform.buildRustPackage {
            pname = "swwws";

            src = pkgs.nix-gitignore.gitignoreSource [] ./.;
            inherit (cargoToml.workspace.package) version;

            cargoLock.lockFile = ./Cargo.lock;

            buildInputs = with pkgs; [
              # swwws requires swww as a runtime dependency
            ];

            doCheck = false; # Integration tests do not work in sandbox environment

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            # No postInstall needed for swwws currently

            meta = {
              description = "Simple Wayland Wallpaper Slideshow - A daemon for automated wallpaper cycling using swww";
              license = lib.licenses.mit;
              platforms = lib.platforms.linux;
              mainProgram = "swwws-daemon";
            };
          };

        default = self.packages.${system}.swwws;
      })
      pkgsFor;

    formatter = lib.mapAttrs (_: pkgs: pkgs.alejandra) pkgsFor;

    devShells =
      lib.mapAttrs (system: pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [self.packages.${system}.swwws];

          packages = [pkgs.rust-bin.stable.${rust-version}.default];
        };
      })
      pkgsFor;

    overlays.default = final: prev: {inherit (self.packages.${prev.system}) swwws;};
  };
}
