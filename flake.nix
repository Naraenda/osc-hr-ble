{
  description = "a rust dev flake";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = { 
    nixpkgs, rust-overlay, ... 
  }: let
    forAllSystems = with nixpkgs.lib; genAttrs systems.flakeExposed;
    overlays = [ (import rust-overlay) ];
  in
  {
    devShells = forAllSystems (system:
      let
        pkgs = import nixpkgs {
          inherit system overlays;
        }; # pkgs
      in {
        default = with pkgs; mkShell {
          buildInputs = [
            pkg-config
            dbus
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-analyzer" "rust-src" ];
            })
          ]; # buildInputs

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ pkgs.dbus ]}:$LD_LIBRARY_PATH"
          '';
        }; # default
      }
    ); # devShells
  }; # outputs
}
