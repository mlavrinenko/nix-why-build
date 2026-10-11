{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    qahq.url = "github:mlavrinenko/qahq";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    {
      self,
      qahq,
      flake-utils,
      nixpkgs,
      ...
    }:
    let
      crate = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
    in
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        # The overlay is the one definition of the package, so `nix build`,
        # the flake check and a consumer's `pkgs.nix-why-build` build the same
        # derivation.
        pkgs = (import nixpkgs) {
          inherit system;
          overlays = [ self.overlays.default ];
        };

        # nixpkgs' rustc ships no `llvm-tools-preview` component, so
        # cargo-llvm-cov cannot find llvm-cov/llvm-profdata in the sysroot.
        # Point it at a standalone LLVM of the SAME major as rustc's — a
        # mismatched llvm-profdata rejects the profraw format.
        llvm = pkgs.llvmPackages_21.llvm;
      in
      {
        packages = {
          # For `nix build` & `nix run`.
          default = pkgs.nix-why-build;
        }
        // pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          # The Linux release binary: static musl, so it runs on NixOS (which
          # has no /lib64/ld-linux) and on any distro. `pkgsStatic` re-imports
          # nixpkgs with the same overlays, so this is the overlay's package
          # function, not a copy. `nix build .#static` reproduces the release.
          static = pkgs.pkgsStatic.nix-why-build;
        };

        # `nix flake check` builds the package `nix run` installs, so a
        # `Cargo.lock` the vendoring rejects, or a source file the build
        # misses, fails CI instead of the README's first install line.
        checks.package = self.packages.${system}.default;

        # `nix fmt` formats the `.nix` files; `just nix-fmt` checks them.
        formatter = pkgs.nixfmt;

        # For `nix develop`:
        devShells.default = pkgs.mkShell {
          # RUSTC_WRAPPER comes from the host session (e.g. a system-wide build
          # cache like kache); the dev shell inherits it and it never reaches the
          # `nix build` sandbox. No wrapper pinned here.
          nativeBuildInputs = [
            qahq.packages.${system}.cargo-crap
            qahq.packages.${system}.ejectest
            qahq.packages.${system}.jscpd
            qahq.packages.${system}.linecop
            # Memoized command runner. Every arm of `just check` runs as
            # `mmz just <subgate>`, so an arm whose declared inputs are
            # unchanged since it last passed is skipped. Rules and their input
            # scopes live in .mmz/config.yaml. `mmz --is-fresh --tag gate`
            # asserts a prior pass without running anything.
            qahq.packages.${system}.mmz
            qahq.packages.${system}.outdatty
          ]
          ++ (with pkgs; [
            rustc
            cargo
            cargo-llvm-cov
            cargo-machete
            cargo-mutants
            cargo-deny
            # `just licenses` renders THIRD-PARTY-LICENSES.md from Cargo.lock;
            # the release workflow attaches it to the GitHub release.
            cargo-about
            # cargo-nextest is the test runner `just test` gates on — doctests
            # run in a separate `cargo test --doc` pass, which is why `just
            # test` spells out both. cargo-outdated is a dev aid, deliberately
            # not wired into `just check`: a new upstream release is not a
            # reason for a gate to go red.
            cargo-nextest
            cargo-outdated
            # `just count-tests` parses nextest's JSON listing with jq.
            jq
            clippy
            rustfmt
            just
            # `wt` forks and lands worktrees: `just fork` and `just land` run
            # it, and .config/wt.toml holds the hooks it fires.
            worktrunk
            # `just nix-fmt` and `just actionlint` gate the `.nix` files and the
            # workflows in `just check`; nixfmt is also the flake's `formatter`.
            nixfmt
            actionlint
            # `just renovate` lists outdated actions, crates and flake inputs
            # with Renovate, in a no-push dry run. A dev aid like cargo-outdated,
            # never a gate.
            renovate
            nixd
            rust-analyzer
          ]);
          shellHook = ''
            export LLVM_COV=${llvm}/bin/llvm-cov
            export LLVM_PROFDATA=${llvm}/bin/llvm-profdata
            # Wire the tracked pre-commit hook (.githooks/) on every shell
            # entry, so a fresh clone's first `nix develop` installs it and
            # a hooksPath edit is picked up live. Quiet: log noise would
            # print on every shell a contributor opens.
            just install-hooks > /dev/null 2>&1
          '';
        };
      }
    )
    // {
      # For a NixOS or home-manager configuration: add this to
      # `nixpkgs.overlays` and `pkgs.nix-why-build` appears. It sits outside
      # `eachDefaultSystem` because an overlay is not per system.
      overlays.default = final: _prev: {
        nix-why-build = final.rustPlatform.buildRustPackage {
          pname = crate.name;
          inherit (crate) version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          # `just check` gates the suite, so the package build skips it.
          doCheck = false;
          # nixpkgs' cargo hook turns Cargo.toml's `strip = true` off and leaves
          # stripping to stdenv, which drops only debug info; strip the
          # binary's symbols too.
          stripAllList = [ "bin" ];
          meta = {
            inherit (crate) description;
            homepage = crate.repository;
            license = final.lib.licenses.mit;
            mainProgram = crate.name;
          };
        };
      };
    };
}
