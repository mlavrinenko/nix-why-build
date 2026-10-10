# The option index nix-why-build evaluates next to the toplevel: for each
# option that can pull a package in, its definitions with their files and what
# each one lists, so the walk's requests resolve to module files in Rust.
# Called as `nix eval --json <configuration> --apply "$(cat index.nix)"`.
cfg:
let
  get =
    path: set:
    builtins.foldl' (
      acc: key:
      if acc != null && builtins.isAttrs acc && builtins.hasAttr key acc then
        builtins.getAttr key acc
      else
        null
    ) set path;
  try =
    default: value:
    let
      result = builtins.tryEval value;
    in
    if result.success then result.value else default;
  split = option: builtins.filter builtins.isString (builtins.split "\\." option);
  drvOf = value: try null (if builtins.isAttrs value && value ? drvPath then value.drvPath else null);
  context =
    value:
    try [ ] (
      builtins.attrNames (
        builtins.getContext (
          if builtins.isAttrs value && value ? outPath then value.outPath else toString value
        )
      )
    );
  attrsOr = value: if builtins.isAttrs value then value else { };
  listOr = value: if builtins.isList value then value else [ ];
  file = definition: builtins.head (builtins.split ", via option " definition.file);
  definitions =
    options: option: listOr (try [ ] (get (split option ++ [ "definitionsWithLocations" ]) options));
  lists =
    options: names:
    builtins.listToAttrs (
      map (option: {
        name = option;
        value = map (d: {
          file = file d;
          drvs = builtins.filter (drv: drv != null) (map drvOf (listOr (try [ ] d.value)));
        }) (definitions options option);
      }) names
    );
  named =
    options: option:
    map (d: {
      file = file d;
      names = builtins.attrNames (attrsOr (try { } d.value));
    }) (definitions options option);
  attrs =
    options: names:
    builtins.listToAttrs (
      map (option: {
        name = option;
        value = named options option;
      }) names
    );
  sources =
    options: config: names:
    builtins.listToAttrs (
      map (option: {
        name = option;
        value = {
          entries = builtins.mapAttrs (_: entry: context (try null (entry.source or null))) (
            attrsOr (get (split option) config)
          );
          definitions = named options option;
        };
      }) names
    );
  units = [
    "services"
    "sockets"
    "timers"
    "paths"
    "mounts"
    "automounts"
    "slices"
    "targets"
  ];
  nixos = {
    prefix = "";
    glue = [ ];
    lists = lists cfg.options [
      "environment.systemPackages"
      "home.packages"
    ];
    attrs = attrs cfg.options (
      map (kind: "systemd.${kind}") units ++ map (kind: "systemd.user.${kind}") units
    );
    sources = sources cfg.options cfg.config [
      "environment.etc"
      "home.file"
      "xdg.configFile"
      "xdg.dataFile"
    ];
  };
  hmMeta = try null (get [ "home-manager" "users" "valueMeta" "attrs" ] cfg.options);
  hm = map (
    user:
    let
      options = hmMeta.${user}.configuration.options;
      config = cfg.config.home-manager.users.${user};
    in
    {
      prefix = "home-manager.users.${user}.";
      glue = builtins.filter (drv: drv != null) [
        (try null config.home.path.drvPath)
        (try null config.home-files.drvPath)
      ];
      lists = lists options [ "home.packages" ];
      attrs = attrs options (map (kind: "systemd.user.${kind}") units);
      sources = sources options config [
        "home.file"
        "xdg.configFile"
        "xdg.dataFile"
      ];
    }
  ) (if hmMeta == null then [ ] else builtins.attrNames hmMeta);
in
{
  scopes = [ nixos ] ++ hm;
}
