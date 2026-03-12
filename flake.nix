{
  description = "A devShell example";

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
            rust-bin.stable.latest.default
          ]; # buildInputs
        }; # default
      }
    ); # devShells
  }; # outputs
}
