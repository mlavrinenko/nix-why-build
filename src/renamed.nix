# Indexes the top level of one nixpkgs-like input by pname: each pname to the
# attributes that carry it, for roots whose pname no attribute guess finds.
# Called with `--impure` as `(this) (builtins.fromJSON "<query>")`; see
# src/renamed.rs.
query:
let
  try =
    default: value:
    let
      result = builtins.tryEval value;
    in
    if result.success then result.value else default;
  # An input that does not import, like one that throws, indexes nothing.
  pkgs = try { } (
    import query.path {
      localSystem = query.system;
      config = {
        allowUnfree = true;
        allowBroken = true;
        allowInsecurePredicate = _: true;
        # An alias only repeats an attribute, and evaluating them all costs
        # half the time and a warning each.
        allowAliases = false;
      };
      overlays = [ ];
    }
  );
  pname =
    name:
    try null (
      let
        value = pkgs.${name};
      in
      if builtins.isAttrs value && builtins.isString (value.pname or null) then value.pname else null
    );
  entries = builtins.filter (entry: entry.pname != null) (
    map (name: {
      inherit name;
      pname = pname name;
    }) (builtins.attrNames pkgs)
  );
in
builtins.mapAttrs (_: map (entry: entry.name)) (builtins.groupBy (entry: entry.pname) entries)
