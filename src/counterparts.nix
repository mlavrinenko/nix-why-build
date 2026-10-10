# Looks each root up in every nixpkgs-like input of the configuration, under
# the attribute guesses nix-why-build passes. Called with `--impure` as
# `(this) (builtins.fromJSON "<query>")`; see src/counterparts.rs.
query:
let
  try =
    default: value:
    let
      result = builtins.tryEval value;
    in
    if result.success then result.value else default;
  get =
    path: set:
    builtins.foldl' (
      acc: key:
      if acc != null && builtins.isAttrs acc && try false (builtins.hasAttr key acc) then
        try null (builtins.getAttr key acc)
      else
        null
    ) set path;
  load =
    input:
    try null (
      import input.path {
        localSystem = query.system;
        config = {
          allowUnfree = true;
          allowBroken = true;
          # No allowUnsupportedSystem: it changes what some packages build
          # (pkgsCuda.llama-cpp), so the counterpart would match no cache.
          allowInsecurePredicate = _: true;
        };
        overlays = [ ];
      }
    );
  sets = builtins.filter (set: set.pkgs != null) (
    map (input: {
      inherit (input) label;
      pkgs = load input;
    }) query.inputs
  );
  describe =
    set: variant: path:
    let
      base = if variant == "" then set.pkgs else get [ variant ] set.pkgs;
      value = if base == null then null else get path base;
    in
    if !(builtins.isAttrs value) || !(try false (value ? drvPath)) then
      null
    else
      {
        input = set.label;
        attr = builtins.concatStringsSep "." path;
        inherit variant;
        drv = try null value.drvPath;
        version = try null (value.version or null);
        unfree = try false (value.meta.unfree or false);
        broken = try false (value.meta.broken or false);
        hydraPlatforms = try null (value.meta.hydraPlatforms or null);
      };
  candidates =
    root:
    builtins.filter (candidate: candidate != null && candidate.drv != null) (
      builtins.concatMap (
        set: builtins.concatMap (variant: map (describe set variant) root.attrs) root.variants
      ) sets
    );
in
{
  hello = map (set: {
    input = set.label;
    drv = try null set.pkgs.hello.drvPath;
  }) sets;
  roots = map candidates query.roots;
}
