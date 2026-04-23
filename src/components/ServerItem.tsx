import { faDiscord, faGithub, faSignalMessenger } from "@fortawesome/free-brands-svg-icons";
import { faBell, faBellSlash, faBook, faChevronDown, faCircleCheck, faComments, faGlobe, faShield, faStar, faUsers } from "@fortawesome/free-solid-svg-icons";
import type { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import type { MouseEvent } from "react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { useConnect, useError } from "../hooks";
import { useConfigStore, useServerStore, useSettingsStore } from "../stores";
import type { Server } from "../bindings";
import { formatDuration } from "../utils";
import { Modal } from "./Modal";

const linkIconMap: Record<string, IconDefinition> = {
  discord: faDiscord,
  wiki: faBook,
  web: faGlobe,
  github: faGithub,
  forum: faComments,
  signal: faSignalMessenger,
};

interface ServerItemProps {
  server: Server;
  showHubStatus?: boolean;
  autoConnecting?: boolean;
}

export const ServerItem = ({
  server,
  showHubStatus = false,
  autoConnecting = false,
}: ServerItemProps) => {
  const { t } = useTranslation();
  const [connecting, setConnecting] = useState(false);
  const [infoOpen, setInfoOpen] = useState(false);
  const [pendingUrl, setPendingUrl] = useState<string | null>(null);
  const { showError } = useError();
  const { connect } = useConnect();

  const hasInfo = !!(server.description || (server.links && server.links.length > 0));

  const config = useConfigStore((s) => s.config);
  const relaysReady = useServerStore((s) => s.relaysReady);
  const notificationsEnabled = useSettingsStore((s) =>
    s.notificationServers.has(server.name),
  );
  const toggleServerNotifications = useSettingsStore(
    (s) => s.toggleServerNotifications,
  );
  const isFavorited = useSettingsStore((s) => server.id ? s.favoriteServers.has(server.id) : false);
  const toggleFavorite = useSettingsStore((s) => s.toggleFavoriteServer);

  const handleHubStatusClick = (e: MouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement;
    const anchor = target.closest("a");
    if (anchor?.href) {
      e.preventDefault();
      commands.openUrl(anchor.href);
    }
  };

  const isOnline = server.status === "available";
  const data = server.data;
  const needsRelays = config?.features.relay_selector ?? false;

  const handleConnect = async () => {
    setConnecting(true);
    try {
      const success = await connect(server.name, "ServerItem.handleConnect");
      if (success && server.id) {
        useSettingsStore.getState().saveLastPlayedServer(server.id);
      }
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err));
    } finally {
      setConnecting(false);
    }
  };

  const supportsHub = server.auth_methods?.includes("hub") ?? false;

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
    <>
      <Modal
        visible={pendingUrl !== null}
        onClose={() => setPendingUrl(null)}
        closeOnOverlayClick
        title={t("servers.openExternalLink")}
      >
          <p className="external-link-prompt">{t("servers.externalLinkPrompt")}</p>
          <p className="external-link-url">{pendingUrl}</p>
          <div className="external-link-actions">
            <button
              type="button"
              className="button"
              onClick={() => {
                if (pendingUrl) commands.openUrl(pendingUrl);
                setPendingUrl(null);
              }}
            >
              {t("common.open")}
            </button>
            <button
              type="button"
              className="button-cancel"
              onClick={() => setPendingUrl(null)}
            >
              {t("common.cancel")}
            </button>
          </div>
      </Modal>
      <div className={`server-item ${!isOnline ? "offline" : ""} ${infoOpen ? "expanded" : ""}`}>
        <div className="server-item-row">
          <div className="server-info">
            {showHubStatus ? (
              <div
                className="hub-status"
                onClick={handleHubStatusClick}
                onKeyDown={() => {}}
                // biome-ignore lint/security/noDangerouslySetInnerHtml: HTML from BYOND hub
                dangerouslySetInnerHTML={{ __html: server.hub_status ?? "" }}
              />
            ) : (
              <>
                <div className="server-name">
                  {server.name}
                  {!data && server.verified_domain && (
                    <button
                      type="button"
                      className="badge badge-verified"
                      title={server.verified_domain}
                      onClick={() => setPendingUrl(`https://${server.verified_domain}`)}
                    >
                      <FontAwesomeIcon icon={faCircleCheck} /> {server.verified_domain}
                    </button>
                  )}
                  {!data && server.is_18_plus && <span className="badge badge-18plus">18+</span>}
                  {!data && server.tags?.filter((tag) => tag !== "18+").map((tag) => (
                    <span key={tag} className="badge badge-tag">{tag}</span>
                  ))}
                </div>
                {data ? (
                  <div className="server-details">
                    <div className="detail-line">
                      {[
                        ...modeMapParts,
                        ...roundInfoParts,
                        ...(data.security_level && data.security_level !== "no_warning"
                          ? ["__security__"]
                          : []),
                      ]
                        .map((part, i) =>
                          part === "__security__" ? (
                            <span key="sec" style={{ color: securityLevelColor }}>
                              {i > 0 && " · "}
                              {data.security_level!.charAt(0).toUpperCase() +
                                data.security_level!.slice(1)}
                            </span>
                          ) : (
                            <span key={String(part)}>
                              {i > 0 && " · "}
                              {part}
                            </span>
                          ),
                        )}
                    </div>
                    {(server.verified_domain || server.is_18_plus || (server.tags && server.tags.length > 0)) && (
                      <div className="server-tags">
                        {server.verified_domain && (
                          <button
                            type="button"
                            className="badge badge-verified"
                            title={server.verified_domain}
                            onClick={() => setPendingUrl(`https://${server.verified_domain}`)}
                          >
                            <FontAwesomeIcon icon={faCircleCheck} /> {server.verified_domain}
                          </button>
                        )}
                        {server.is_18_plus && <span className="badge badge-18plus">18+</span>}
                        {server.tags?.filter((tag) => tag !== "18+").map((tag) => (
                          <span key={tag} className="badge badge-tag">{tag}</span>
                        ))}
                      </div>
                    )}
                  </div>
                ) : !isOnline ? (
                  <div className="server-details">
                    <div className="detail-line dim">{t("servers.offline")}</div>
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
            {hasInfo && (
              <button
                type="button"
                className={`notify-toggle ${infoOpen ? "enabled" : ""}`}
                onClick={() => setInfoOpen(!infoOpen)}
                title="Server info"
              >
                <FontAwesomeIcon icon={faChevronDown} style={{ transform: infoOpen ? "rotate(180deg)" : undefined, transition: "transform 0.15s" }} />
              </button>
            )}
            <div className="connect-group">
              <button
                type="button"
                className="button connect-button"
                onClick={handleConnect}
                disabled={!canConnect || connecting || autoConnecting}
              >
                {connecting || autoConnecting ? "..." : (
                  <>
                    {config?.features.connect_logo && (
                      <img
                        src={supportsHub ? "/logo-ss13.png" : "/byond.png"}
                        alt=""
                        className="connect-auth-icon"
                      />
                    )}
                    {t("common.join")}
                  </>
                )}
              </button>
              {data?.round_id != null && (
                <button
                  type="button"
                  className={`notify-toggle ${notificationsEnabled ? "enabled" : ""}`}
                  onClick={handleToggleNotifications}
                  title={
                    notificationsEnabled
                      ? t("servers.disableNotifications")
                      : t("servers.enableNotifications")
                  }
                >
                  <FontAwesomeIcon icon={notificationsEnabled ? faBell : faBellSlash} />
                </button>
              )}
              {server.id && (
                <button
                  type="button"
                  className={`notify-toggle favorite-toggle ${isFavorited ? "favorited" : ""}`}
                  onClick={() => toggleFavorite(server.id!, !isFavorited)}
                  title={isFavorited ? t("servers.unfavorite") : t("servers.favorite")}
                >
                  <FontAwesomeIcon icon={faStar} />
                </button>
              )}
            </div>
          </div>
        </div>
        {infoOpen && (
          <div className="server-info-panel">
            {server.description && (
              <div className="server-description">{server.description}</div>
            )}
            {server.links && server.links.length > 0 && (
              <div className="server-links">
                {server.links.map((link) => (
                  <button
                    key={link.link}
                    type="button"
                    className="button-secondary server-link-button"
                    onClick={() => setPendingUrl(link.link)}
                    title={link.type}
                  >
                    <FontAwesomeIcon icon={linkIconMap[link.type] ?? faGlobe} />
                    <span>{link.type.charAt(0).toUpperCase() + link.type.slice(1)}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </>
  );
};
