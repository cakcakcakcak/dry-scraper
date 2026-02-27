{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          openssl.dev
          pkg-config
          rustc
          rust-analyzer
          rustfmt
          clippy
        ];
        
        shellHook = ''
          echo "🦀 Rust development environment"
          echo "cargo: $(cargo --version)"
        '';
      };
    };
}
