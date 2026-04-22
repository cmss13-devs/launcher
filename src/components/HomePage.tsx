import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { Server } from "../bindings";
import { useSettingsStore } from "../stores";
import { ServerItem } from "./ServerItem";

interface HomePageProps {
  servers: Server[];
}

export const HomePage = ({ servers }: HomePageProps) => {
  const { t } = useTranslation();
  const lastPlayedServer = useSettingsStore((s) => s.lastPlayedServer);
  const favoriteServers = useSettingsStore((s) => s.favoriteServers);

  const lastPlayed = useMemo(
    () => servers.find((s) => s.id && s.id === lastPlayedServer && s.status === "available"),
    [servers, lastPlayedServer],
  );

  const favorites = useMemo(
    () => servers.filter((s) => s.id && favoriteServers.has(s.id)),
    [servers, favoriteServers],
  );

  const hasContent = lastPlayed || favorites.length > 0;

  return (
    <div className="home-page">
      {lastPlayed && (
        <div className="home-section">
          <div className="home-section-title">{t("home.continuePlaying")}</div>
          <div className="server-list home-server-list">
            <ServerItem server={lastPlayed} />
          </div>
        </div>
      )}
      {favorites.length > 0 && (
        <div className="home-section">
          <div className="home-section-title">{t("home.favorites")}</div>
          <div className="server-list home-server-list">
            {favorites.map((server) => (
              <ServerItem key={server.url} server={server} />
            ))}
          </div>
        </div>
      )}
      {!hasContent && (
        <div className="home-empty">{t("home.noFavorites")}</div>
      )}
    </div>
  );
};
