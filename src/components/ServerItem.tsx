import { faBell, faBellSlash, faShield, faUsers } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useState } from "react";
import { useConnect, useError } from "../hooks";
import { useConfigStore, useServerStore, useSettingsStore } from "../stores";
import type { Server } from "../types";
import { formatDuration } from "../utils";

interface ServerItemProps {
  server: Server;
  showHubStatus?: boolean;
  onLoginRequired: () => void;
  onSteamAuthRequired: (serverName?: string) => void;
  autoConnecting?: boolean;
}

export const ServerItem = ({
  server,
  showHubStatus = false,
  onLoginRequired,
  onSteamAuthRequired,
  autoConnecting = false,
}: ServerItemProps) => {
  const [connecting, setConnecting] = useState(false);
  const { showError } = useError();
  const { connect } = useConnect();

  const config = useConfigStore((s) => s.config);
  const relaysReady = useServerStore((s) => s.relaysReady);
  const notificationsEnabled = useSettingsStore((s) =>
    s.notificationServers.has(server.name),
  );
  const toggleServerNotifications = useSettingsStore(
    (s) => s.toggleServerNotifications,
  );

  const isOnline = server.status === "available";
  const data = server.data;
  const needsRelays = config?.features.relay_selector ?? false;

  const handleConnect = async () => {
    setConnecting(true);

    try {
      const result = await connect(server.name, "ServerItem.handleConnect");

      if (!result.success && result.auth_error) {
        if (result.auth_error.code === "auth_required") {
          onLoginRequired();
        } else if (result.auth_error.code === "steam_linking_required") {
          onSteamAuthRequired(server.name);
        } else {
          showError(result.auth_error.message);
        }
      } else if (!result.success) {
        showError(result.message);
      }
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err));
    } finally {
      setConnecting(false);
    }
  };

  const canConnect = isOnline && (!needsRelays || relaysReady);

  const handleToggleNotifications = async () => {
    try {
      await toggleServerNotifications(server.name, !notificationsEnabled);
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err));
    }
  };

  const modeMapParts = [
    data?.mode && data.mode.charAt(0).toUpperCase() + data.mode.slice(1),
    data?.map_name,
  ].filter(Boolean);

  const roundInfoParts = [
    data?.round_id && `#${data.round_id}`,
    data?.round_duration != null && formatDuration(data.round_duration),
  ].filter(Boolean);

  const securityLevelColor =
    data?.security_level === "red"
      ? "#f87171"
      : data?.security_level === "blue"
        ? "#60a5fa"
        : data?.security_level === "green"
          ? "#4ade80"
          : undefined;

  return (
    <div className={`server-item ${!isOnline ? "offline" : ""}`}>
      <div className="server-info">
        {showHubStatus ? (
          <div
            className="hub-status"
            // biome-ignore lint/security/noDangerouslySetInnerHtml: HTML from BYOND hub
            dangerouslySetInnerHTML={{ __html: server.hub_status }}
          />
        ) : (
          <>
            <div className="server-name">
              {server.name}
              {server.is_18_plus && <span className="badge badge-18plus">18+</span>}
            </div>
            {data ? (
              <div className="server-details">
                {modeMapParts.length > 0 && (
                  <div className="detail-line">{modeMapParts.join(" · ")}</div>
                )}
                {(roundInfoParts.length > 0 || data.security_level) && (
                  <div className="detail-line">
                    {roundInfoParts.join(" · ")}
                    {data.security_level &&
                      data.security_level !== "no_warning" && (
                        <>
                          {roundInfoParts.length > 0 && " · "}
                          <span style={{ color: securityLevelColor }}>
                            {data.security_level.charAt(0).toUpperCase() +
                              data.security_level.slice(1)}
                          </span>
                        </>
                      )}
                  </div>
                )}
              </div>
            ) : !isOnline ? (
              <div className="server-details">
                <div className="detail-line dim">Offline</div>
              </div>
            ) : null}
          </>
        )}
      </div>
      <div className="server-status">
        <div className="server-counts">
          <div className="player-count">
            <FontAwesomeIcon icon={faUsers} className="player-icon" />
            {isOnline ? (
              <>
                {server.players}
                {data?.popcap != null && `/${data.popcap}`}
              </>
            ) : (
              "--"
            )}
          </div>
          {data?.admins != null && data.admins > 0 && (
            <div className="admin-count">
              <FontAwesomeIcon icon={faShield} className="admin-icon" />
              {data.admins}
            </div>
          )}
        </div>
        <div className="connect-group">
          <button
            type="button"
            className="button connect-button"
            onClick={handleConnect}
            disabled={!canConnect || connecting || autoConnecting}
          >
            {connecting || autoConnecting ? "Connecting..." : "Connect"}
          </button>
          {data?.round_id != null && (
            <button
              type="button"
              className={`notify-toggle ${notificationsEnabled ? "enabled" : ""}`}
              onClick={handleToggleNotifications}
              title={
                notificationsEnabled
                  ? "Disable restart notifications"
                  : "Enable restart notifications"
              }
            >
              <FontAwesomeIcon icon={notificationsEnabled ? faBell : faBellSlash} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
