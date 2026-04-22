import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { LauncherFeatures } from "../bindings";
import type { useServerFilters } from "../hooks/useServerFilters";
import { useSettingsStore } from "../stores";
import { AgeVerificationModal } from "./AgeVerificationModal";

type FilterState = ReturnType<typeof useServerFilters>;

export type ViewMode = "home" | "browse" | "singleplayer";

interface ServerFilterPanelProps {
  features: LauncherFeatures;
  filters: FilterState;
  serverCount: number;
  playerCount: number;
  viewMode: ViewMode;
  onViewModeChange: (mode: ViewMode) => void;
}

export const ServerFilterPanel = ({
  features,
  filters,
  serverCount,
  playerCount,
  viewMode,
  onViewModeChange,
}: ServerFilterPanelProps) => {
  const {
    searchQuery,
    setSearchQuery,
    selectedTags,
    toggleTag,
    show18Plus,
    setShow18Plus,
    showOffline,
    setShowOffline,
    showHubStatus,
    setShowHubStatus,
    filtersOpen,
    setFiltersOpen,
    filtersRef,
    categories,
    hasOffline,
    hasHubStatus,
  } = filters;

  const { t } = useTranslation();
  const ageVerified = useSettingsStore((s) => s.ageVerified);
  const saveAgeVerified = useSettingsStore((s) => s.saveAgeVerified);
  const [ageModalVisible, setAgeModalVisible] = useState(false);

  const handle18PlusChange = (checked: boolean) => {
    if (checked && !ageVerified) {
      setAgeModalVisible(true);
    } else {
      setShow18Plus(checked);
    }
  };

  const handleAgeVerified = async () => {
    await saveAgeVerified();
    setAgeModalVisible(false);
    setShow18Plus(true);
  };

  const tagCategories = categories.filter((c) => c !== "sandbox");
  const isBrowse = viewMode === "browse";

  return (
    <>
      <AgeVerificationModal
        visible={ageModalVisible}
        onVerified={handleAgeVerified}
        onClose={() => setAgeModalVisible(false)}
      />
      <div className="server-header">
        <div className="view-tabs">
          <button
            type="button"
            className={`view-tab${viewMode === "home" ? " active" : ""}`}
            onClick={() => onViewModeChange("home")}
          >
            {t("nav.home")}
          </button>
          <button
            type="button"
            className={`view-tab${viewMode === "browse" ? " active" : ""}`}
            onClick={() => onViewModeChange("browse")}
          >
            {t("nav.browse")}
          </button>
          {features.singleplayer && (
            <button
              type="button"
              className={`view-tab${viewMode === "singleplayer" ? " active" : ""}`}
              onClick={() => onViewModeChange("singleplayer")}
            >
              {t("servers.singleplayer")}
            </button>
          )}
        </div>
        {isBrowse && (
          <div className="server-controls">
            {features.server_stats && (
              <div className="server-stats">
                <span className="stat-label">{t("servers.serversStat")}</span>
                <span className="stat-value">{serverCount}</span>
                <span className="stat-label">{t("servers.playersStat")}</span>
                <span className="stat-value">{playerCount}</span>
              </div>
            )}
            {features.server_search && (
              <input
                type="text"
                className="search-input"
                placeholder={t("servers.searchPlaceholder")}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            )}
            {(features.server_filters || tagCategories.length > 0) && (
              <div className="filters-dropdown" ref={filtersRef}>
                <button
                  type="button"
                  className={`filters-button${selectedTags.size > 0 ? " active" : ""}`}
                  onClick={() => setFiltersOpen(!filtersOpen)}
                >
                  {selectedTags.size > 0 ? t("servers.filtersCount", { count: selectedTags.size }) : t("servers.filters")}
                </button>
                {filtersOpen && (
                  <div className="filters-menu">
                    {features.server_filters && (
                      <>
                        {hasHubStatus && (
                          <label className="filter-checkbox">
                            <input
                              type="checkbox"
                              checked={showHubStatus}
                              onChange={(e) => setShowHubStatus(e.target.checked)}
                            />
                            <span>{t("servers.hubStatus")}</span>
                          </label>
                        )}
                        <label className="filter-checkbox">
                          <input
                            type="checkbox"
                            checked={show18Plus}
                            onChange={(e) => handle18PlusChange(e.target.checked)}
                          />
                          <span>{t("servers.eighteenPlus")}</span>
                        </label>
                        {hasOffline && (
                          <label className="filter-checkbox">
                            <input
                              type="checkbox"
                              checked={showOffline}
                              onChange={(e) => setShowOffline(e.target.checked)}
                            />
                            <span>{t("servers.offlineServers")}</span>
                          </label>
                        )}
                      </>
                    )}
                    {features.server_filters && tagCategories.length > 0 && (
                      <div className="filter-divider" />
                    )}
                    {tagCategories.map((tag) => (
                      <label className="filter-checkbox" key={tag}>
                        <input
                          type="checkbox"
                          checked={selectedTags.has(tag)}
                          onChange={(e) => toggleTag(tag, e.target.checked)}
                        />
                        <span>{tag}</span>
                      </label>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </>
  );
};
