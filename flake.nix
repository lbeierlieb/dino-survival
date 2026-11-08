{
  description = "Fire, Water, Storm, Lightning: a dinosaur party game (Bevy)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      crane,
      ...
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;

      # Everything this flake provides, as { <system> = { packages, apps, checks, devShells }; }.
      # Evaluated once per system and shared by all outputs, so nixpkgs is imported only once.
      bySystem = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
          lib = pkgs.lib;

          toolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
            targets = [ "wasm32-unknown-unknown" ];
          };
          craneLib = (crane.mkLib pkgs).overrideToolchain (_: toolchain);

          cargoToml = lib.importTOML ./Cargo.toml;
          pname = cargoToml.package.name;
          version = cargoToml.package.version;

          # Must match the `wasm-bindgen` version pinned in Cargo.lock.
          wasm-bindgen-cli = pkgs.wasm-bindgen-cli_0_2_130;

          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./src
              ./web
            ];
          };

          # Libraries Bevy loads at runtime on Linux (windowing + GPU).
          runtimeLibs = lib.optionals pkgs.stdenv.hostPlatform.isLinux (
            with pkgs;
            [
              vulkan-loader
              libGL
              libxkbcommon
              wayland
              libx11
              libxcursor
              libxi
              libxrandr
            ]
          );
          buildLibs = lib.optionals pkgs.stdenv.hostPlatform.isLinux (
            with pkgs;
            [
              udev
              alsa-lib
            ]
          );

          # Each target builds its dependencies once (`*Deps`); its package and clippy
          # check reuse them, so Bevy isn't compiled again for every derivation.
          nativeArgs = {
            inherit pname version src;
            strictDeps = true;
            doCheck = false;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = buildLibs ++ runtimeLibs;
          };
          nativeDeps = craneLib.buildDepsOnly nativeArgs;

          wasmArgs = {
            pname = "${pname}-wasm";
            inherit version src;
            strictDeps = true;
            doCheck = false;
            CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
            CARGO_PROFILE = "wasm-release";
          };
          wasmDeps = craneLib.buildDepsOnly wasmArgs;

          native = craneLib.buildPackage (
            nativeArgs
            // {
              cargoArtifacts = nativeDeps;
              nativeBuildInputs =
                nativeArgs.nativeBuildInputs ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.makeWrapper ];
              postFixup = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
                wrapProgram $out/bin/${pname} \
                  --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs}
              '';
              meta.mainProgram = pname;
            }
          );

          wasm = craneLib.mkCargoDerivation (
            wasmArgs
            // {
              cargoArtifacts = wasmDeps;
              doInstallCargoArtifacts = false;
              nativeBuildInputs = [
                wasm-bindgen-cli
                pkgs.binaryen
              ];
              buildPhaseCargoCommand = "cargoWithProfile build --locked";
              installPhaseCommand = ''
                mkdir -p $out
                wasm-bindgen --target web --no-typescript \
                  --out-dir $out --out-name ${pname} \
                  target/wasm32-unknown-unknown/wasm-release/${pname}.wasm
                wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
                  -o $out/${pname}_bg.wasm $out/${pname}_bg.wasm
                cp web/index.html $out/
              '';
              dontFixup = true;
            }
          );

          # Dev shell only: `cargo run` becomes an optimized build with quick incremental
          # rebuilds (the `release-dev` profile). Other cargo commands pass through.
          cargo-dev = pkgs.writeShellScriptBin "cargo" ''
            if [ "''${1:-}" = "run" ]; then
              shift
              exec ${toolchain}/bin/cargo run --profile release-dev "$@"
            fi
            exec ${toolchain}/bin/cargo "$@"
          '';

          # `cargo clippy` for one target, failing on any warning.
          clippy =
            args: deps:
            craneLib.cargoClippy (
              args
              // {
                cargoArtifacts = deps;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

          # Lint a Nix file with a tool that exits non-zero on problems.
          nixLint =
            name: tool: command:
            pkgs.runCommand name { nativeBuildInputs = [ tool ]; } ''
              ${command} ${./flake.nix}
              touch $out
            '';

          # `nix run .#serve-wasm` serves the wasm build on http://localhost:8080
          serve-wasm = pkgs.writeShellApplication {
            name = "serve-wasm";
            runtimeInputs = [ pkgs.python3 ];
            text = ''
              echo "Serving on http://localhost:8080"
              python3 -m http.server 8080 --directory ${wasm}
            '';
          };
        in
        {
          packages = {
            default = native;
            inherit native wasm serve-wasm;
          };

          apps = {
            default = {
              type = "app";
              program = lib.getExe native;
              meta.description = "Play the game";
            };
            serve-wasm = {
              type = "app";
              program = lib.getExe serve-wasm;
              meta.description = "Serve the web build on http://localhost:8080";
            };
          };

          checks = {
            fmt = craneLib.cargoFmt { inherit pname version src; };
            clippy-native = clippy nativeArgs nativeDeps;
            clippy-wasm = clippy wasmArgs wasmDeps;
            nixfmt = nixLint "nixfmt" pkgs.nixfmt "nixfmt --check";
            deadnix = nixLint "deadnix" pkgs.deadnix "deadnix --fail";
          };

          devShells.default = pkgs.mkShell {
            packages = [
              toolchain
              wasm-bindgen-cli
              pkgs.binaryen
              pkgs.pkg-config
              pkgs.python3
              pkgs.nixfmt
              pkgs.deadnix
            ]
            ++ buildLibs
            ++ runtimeLibs;
            LD_LIBRARY_PATH = lib.makeLibraryPath runtimeLibs;
            shellHook = ''
              export PATH=${cargo-dev}/bin:$PATH
            '';
          };
        }
      );
    in
    {
      packages = forAllSystems (system: bySystem.${system}.packages);
      apps = forAllSystems (system: bySystem.${system}.apps);
      checks = forAllSystems (system: bySystem.${system}.checks);
      devShells = forAllSystems (system: bySystem.${system}.devShells);
    };
}
