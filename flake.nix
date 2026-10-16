{
  description = "tsukimi development environment";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ];
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              meson
              ninja
              pkg-config
              just
              cargo
              rustc
              rustfmt
              clippy
              rust-analyzer
              git
              blueprint-compiler
              desktop-file-utils
              appstream
              libxml2
              glib
            ];

            buildInputs = with pkgs; [
              openssl
              dbus
              glib
              gtk4
              libadwaita
              libepoxy
              mpv-unwrapped
              libglycin
              glycin-loaders
              gst_all_1.gstreamer
              gst_all_1.gst-plugins-base
              gst_all_1.gst-plugins-good
              gst_all_1.gst-plugins-bad
              gsettings-desktop-schemas
            ];

            RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

            shellHook = ''
              export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk4}/share/gsettings-schemas/${pkgs.gtk4.name}:${pkgs.glycin-loaders}/share''${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}"
            '';
          };
        }
      );
    };
}
