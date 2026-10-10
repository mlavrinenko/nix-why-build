# A second nixpkgs, one release ahead of ../nixpkgs.
{
  localSystem,
  config,
  overlays,
}:
let
  pkg =
    pname: version:
    derivation {
      name = "${pname}-${version}";
      builder = "/bin/sh";
      system = localSystem;
    }
    // {
      inherit pname version;
    };
in
{
  hello = pkg "hello" "2.13";
  llama-cpp = pkg "llama-cpp" "0.6.0";
}
