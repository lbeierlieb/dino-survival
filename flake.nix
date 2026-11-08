{
  description = "Fire, Water, Storm, Lightning: a dinosaur party game (Bevy)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
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
        rustPlatform = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };

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

        native = rustPlatform.buildRustPackage {
          inherit pname version src;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [
            pkgs.pkg-config
          ]
          ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.makeWrapper ];
          buildInputs = buildLibs ++ runtimeLibs;
          doCheck = false;
          postFixup = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            wrapProgram $out/bin/${pname} \
              --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs}
          '';
          meta.mainProgram = pname;
        };

        wasm = rustPlatform.buildRustPackage {
          pname = "${pname}-wasm";
          inherit version src;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [
            wasm-bindgen-cli
            pkgs.binaryen
          ];
          doCheck = false;
          buildPhase = ''
            runHook preBuild
            cargo build --profile wasm-release --frozen --target wasm32-unknown-unknown
            runHook postBuild
          '';
          installPhase = ''
            runHook preInstall
            mkdir -p $out
            wasm-bindgen --target web --no-typescript \
              --out-dir $out --out-name ${pname} \
              target/wasm32-unknown-unknown/wasm-release/${pname}.wasm
            wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
              -o $out/${pname}_bg.wasm $out/${pname}_bg.wasm
            cp web/index.html $out/
            runHook postInstall
          '';
          dontFixup = true;
        };

        # Dev shell only: `cargo run` becomes an optimized build with quick incremental
        # rebuilds (the `release-dev` profile). Other cargo commands pass through.
        cargo-dev = pkgs.writeShellScriptBin "cargo" ''
          if [ "''${1:-}" = "run" ]; then
            shift
            exec ${toolchain}/bin/cargo run --profile release-dev "$@"
          fi
          exec ${toolchain}/bin/cargo "$@"
        '';

        # `cargo clippy` for one target, failing on any warning. Builds nothing but a marker file.
        clippy =
          name: targetArgs: extraBuildInputs:
          rustPlatform.buildRustPackage {
            pname = "${pname}-clippy-${name}";
            inherit version src;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = extraBuildInputs;
            doCheck = false;
            buildPhase = ''
              runHook preBuild
              cargo clippy --frozen --all-targets ${targetArgs} -- -D warnings
              runHook postBuild
            '';
            installPhase = "touch $out";
            dontFixup = true;
          };

        # Lint a Nix file with a tool that exits non-zero on problems.
        nixLint =
          name: tool: command:
          pkgs.runCommand name { nativeBuildInputs = [ tool ]; } ''
            ${command} ${./flake.nix}
            touch $out
          '';

        # `cargo fmt --check`, using the toolchain's rustfmt so it agrees with the dev shell.
        # rustfmt follows `mod` declarations from the crate root.
        fmt = pkgs.runCommand "${pname}-fmt" { nativeBuildInputs = [ toolchain ]; } ''
          rustfmt --check --edition 2024 ${src}/src/main.rs
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
          default = flake-utils.lib.mkApp { drv = native; };
          serve-wasm = flake-utils.lib.mkApp { drv = serve-wasm; };
        };

        checks = {
          inherit fmt;
          clippy-native = clippy "native" "" (buildLibs ++ runtimeLibs);
          clippy-wasm = clippy "wasm" "--target wasm32-unknown-unknown" [ ];
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
}
