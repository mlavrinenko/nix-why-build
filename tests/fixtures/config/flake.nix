# Configurations with the shapes src/index.nix reads, built from `builtins`
# alone: no module system, no nixpkgs, no network.
{
  outputs =
    { self }:
    let
      drv =
        name:
        derivation {
          inherit name;
          builder = "/bin/sh";
          system = "x86_64-linux";
        };
      hello = drv "hello-2.12";
      htop = drv "htop-3.3";
      fonts = drv "fonts-1";
      # The module system's shape: a file, with `, via option` when a module
      # reached it through a flake module.
      definition = file: value: { inherit file value; };
      option = definitions: { definitionsWithLocations = definitions; };
    in
    {
      # A NixOS system with one home-manager user.
      nixosConfigurations.tank = {
        options = {
          environment = {
            systemPackages = option [
              (definition "/m/a.nix, via option flake.modules.nixos.a" [ hello ])
              (definition "/m/b.nix" [ "not a package" ])
              (definition "/m/c.nix" (throw "does not evaluate"))
            ];
            etc = option [
              (definition "/m/d.nix" {
                "hello.conf" = { };
                "plain" = { };
              })
              (definition "/m/e.nix" { "htop.conf" = { }; })
            ];
          };
          systemd = {
            services = option [
              (definition "/m/f.nix" {
                sshd = { };
                "nix-daemon" = { };
              })
              (definition "/m/g.nix" { sshd = { }; })
            ];
            user.timers = option [ (definition "/m/h.nix" { backup = { }; }) ];
          };
          home-manager.users.valueMeta.attrs.alice.configuration.options = {
            home = {
              packages = option [ (definition "/m/i.nix" [ htop ]) ];
              file = option [ (definition "/m/j.nix" { ".config/hello" = { }; }) ];
            };
            xdg.configFile = option [ (definition "/m/k.nix" { "fonts/fonts.conf" = { }; }) ];
            systemd.user.services = option [ (definition "/m/l.nix" { syncthing = { }; }) ];
          };
        };
        config = {
          environment.etc = {
            # A derivation, and a string that refers to one.
            "hello.conf".source = hello;
            "htop.conf".source = "${htop}/etc/htop.conf";
            "plain".text = "no source";
          };
          home-manager.users.alice = {
            home = {
              path = drv "home-manager-path";
              file.".config/hello".source = hello;
            };
            home-files = drv "home-manager-files";
            xdg.configFile."fonts/fonts.conf".source = "${fonts}/fonts.conf";
          };
        };
      };
      # No home-manager, and nothing set.
      nixosConfigurations.bare = {
        options = { };
        config = { };
      };
    };
}
