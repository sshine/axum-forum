{ config, lib, pkgs, ... }:
let
  cfg = config.services.axum-forum;
in {
  options.services.axum-forum = {
    enable = lib.mkEnableOption "axum-forum service";

    package = lib.mkOption {
      type = lib.types.package;
      default = import ./default.nix { inherit pkgs; };
      description = "The axum-forum package to use";
    };

    dataDir = lib.mkOption {
      type = lib.types.path;
      default = "/var/lib/axum-forum";
      description = "Directory to store the SQLite database";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "axum-forum";
      description = "User account under which the service runs";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "axum-forum";
      description = "Group under which the service runs";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 3000;
      description = "Port on which axum-forum will listen";
    };
  };

  config = lib.mkIf cfg.enable {
    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.group;
      description = "axum-forum service user";
    };

    users.groups.${cfg.group} = {};

    systemd.services.axum-forum = {
      description = "axum-forum service";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/axum-forum";
        User = cfg.user;
        Group = cfg.group;

        # Set up runtime directory
        RuntimeDirectory = "axum-forum";
        RuntimeDirectoryMode = "0750";

        # Set up data directory for the SQLite database
        StateDirectory = "axum-forum";
        StateDirectoryMode = "0750";

        # Security hardening
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        NoNewPrivileges = true;

        Restart = "on-failure";
        RestartSec = "5s";
      };

      #environment = {
      #  DATABASE_URL = "sqlite:${cfg.dataDir}/forum.db";
      #  PORT = toString cfg.port;
      #  RUNTIME_DIR = "/run/axum-forum";
      #};
    };

    # Ensure the data directory exists with correct permissions
    system.activationScripts.axum-forum-data-dir = ''
      mkdir -p ${cfg.dataDir}
      chown ${cfg.user}:${cfg.group} ${cfg.dataDir}
      chmod 750 ${cfg.dataDir}
    '';
  };
}
