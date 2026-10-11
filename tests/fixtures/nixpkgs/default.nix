# A stand-in for nixpkgs: the arguments src/counterparts.nix imports it with,
# and derivations shaped like the attributes it reads. The asserts refuse what
# the import must not ask for, so a changed call shows up as an input that does
# not import and the lookups finding nothing.
{
  localSystem,
  config,
  overlays,
}:
assert config.allowUnfree && config.allowBroken && config.allowInsecurePredicate "any";
# allowUnsupportedSystem changes what some packages build, so the counterpart
# would match no cache.
assert !(config ? allowUnsupportedSystem);
assert overlays == [ ];
let
  mk =
    name: attrs:
    derivation {
      inherit name;
      builder = "/bin/sh";
      system = localSystem;
      # Part of the derivation, so the CUDA build differs from the plain one.
      cuda = attrs.cudaSupport or false;
    }
    // attrs;
  pkg =
    pname: version: attrs:
    mk "${pname}-${version}" ({ inherit pname version; } // attrs);
in
{
  # Found under the pname itself.
  hello = pkg "hello" "2.12" { };
  llama-cpp = pkg "llama-cpp" "0.5.0" { };
  # Found under the camelCase guess (glibc-locales).
  glibcLocales = pkg "glibc-locales" "2.40" { };
  # Found only by the index: no guess spells cacert from nss-cacert.
  cacert = pkg "nss-cacert" "3.126" { };
  # Found in the language sets.
  python3Packages = {
    requests = pkg "requests" "2.32.0" { };
    flask-cors = pkg "flask-cors" "5.0.0" { };
  };
  perlPackages.IOTty = pkg "IO-Tty" "1.20" { meta.broken = true; };
  # Package sets for variants.
  pkgsCuda.llama-cpp = pkg "llama-cpp" "0.5.0" { cudaSupport = true; };
  pkgsi686Linux.eglexternalplatform = pkg "eglexternalplatform" "1.2" { };
  # What meta says.
  corefonts = pkg "corefonts" "1" {
    meta = {
      unfree = true;
      hydraPlatforms = [ ];
    };
  };
  ffmpeg = pkg "ffmpeg" "7.1" {
    meta.hydraPlatforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
  };
  # Every field a lookup reads fails to evaluate: they fall back.
  fragile = mk "fragile-1" {
    version = throw "no version";
    meta = {
      unfree = throw "no unfree";
      broken = throw "no broken";
      hydraPlatforms = throw "no hydraPlatforms";
    };
  };
  # The system the import was given, as a version.
  system-probe = pkg "system-probe" localSystem { };
  # What is no candidate.
  aliased = throw "aliased away";
  notADerivation.version = "1";
  undrvable = mk "undrvable-1" { drvPath = throw "no drv"; };
}
