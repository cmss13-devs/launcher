import { faArrowUpRightFromSquare } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { LauncherFeatures } from "../bindings";
import type { useServerFilters } from "../hooks/useServerFilters";

function getLanguageDisplayName(code: string): string {
  try {
    const name = new Intl.DisplayNames([navigator.language, "en"], {
      type: "language",
    }).of(code);
    if (name) return name.toLowerCase();
  } catch {}
  return code;
}
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
  showHome: boolean;
  onDirectConnect?: () => void;
}

export const ServerFilterPanel = ({
  features,
  filters,
  serverCount,
  playerCount,
  viewMode,
  onViewModeChange,
  showHome,
  onDirectConnect,
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
    selectedRegions,
    toggleRegion,
    regions,
    selectedLanguages,
    toggleLanguage,
    languages,
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

  const languageLabels = useMemo(
    () => new Map(languages.map((code) => [code, getLanguageDisplayName(code)])),
    [languages],
  );

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
          {showHome && (
            <button
              type="button"
              className={`view-tab${viewMode === "home" ? " active" : ""}`}
              onClick={() => onViewModeChange("home")}
            >
              {t("nav.home")}
            </button>
          )}
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
          {features.direct_connect && onDirectConnect && (
            <button
              type="button"
              className="view-tab"
              onClick={onDirectConnect}
            >
              {t("common.connect")}{" "}
              <FontAwesomeIcon icon={faArrowUpRightFromSquare} />
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
            {(features.server_filters || tagCategories.length > 0) &&
              (() => {
                const activeCount = selectedTags.size + selectedRegions.size + selectedLanguages.size;
                return (
                  <div className="filters-dropdown" ref={filtersRef}>
                    <button
                      type="button"
                      className={`filters-button${activeCount > 0 ? " active" : ""}`}
                      onClick={() => setFiltersOpen(!filtersOpen)}
                    >
                      {activeCount > 0
                        ? t("servers.filtersCount", { count: activeCount })
                        : t("servers.filters")}
                    </button>
                    {filtersOpen && (
                      <div className="filters-menu">
                        {features.server_filters && (
                          <>
                            {hasHubStatus && (
                              <label className="styled-checkbox">
                                <input
                                  type="checkbox"
                                  checked={showHubStatus}
                                  onChange={(e) =>
                                    setShowHubStatus(e.target.checked)
                                  }
                                />
                                <span>{t("servers.hubStatus")}</span>
                              </label>
                            )}
                            <label className="styled-checkbox">
                              <input
                                type="checkbox"
                                checked={show18Plus}
                                onChange={(e) =>
                                  handle18PlusChange(e.target.checked)
                                }
                              />
                              <span>{t("servers.eighteenPlus")}</span>
                            </label>
                            {hasOffline && (
                              <label className="styled-checkbox">
                                <input
                                  type="checkbox"
                                  checked={showOffline}
                                  onChange={(e) =>
                                    setShowOffline(e.target.checked)
                                  }
                                />
                                <span>{t("servers.offlineServers")}</span>
                              </label>
                            )}
                          </>
                        )}
                        {features.server_filters &&
                          tagCategories.length > 0 && (
                            <div className="filter-divider" />
                          )}
                        <div className="filter-row">
                          {tagCategories.map((tag) => (
                            <label className="styled-checkbox" key={tag}>
                              <input
                                type="checkbox"
                                checked={selectedTags.has(tag)}
                                onChange={(e) =>
                                  toggleTag(tag, e.target.checked)
                                }
                              />
                              <span>{tag}</span>
                            </label>
                          ))}
                        </div>
                        {regions.length > 0 && (
                          <>
                            <div className="filter-divider" />
                            <div className="filter-row">
                              {regions.map((region) => (
                                <label className="styled-checkbox" key={region}>
                                  <input
                                    type="checkbox"
                                    checked={selectedRegions.has(region)}
                                    onChange={(e) =>
                                      toggleRegion(region, e.target.checked)
                                    }
                                  />
                                  <span>{region}</span>
                                </label>
                              ))}
                            </div>
                          </>
                        )}
                        {languages.length > 0 && (
                          <>
                            <div className="filter-divider" />
                            <div className="filter-row">
                              {languages.map((lang) => (
                                <label className="styled-checkbox" key={lang}>
                                  <input
                                    type="checkbox"
                                    checked={selectedLanguages.has(lang)}
                                    onChange={(e) =>
                                      toggleLanguage(lang, e.target.checked)
                                    }
                                  />
                                  <span>{languageLabels.get(lang) ?? lang}</span>
                                </label>
                              ))}
                            </div>
                          </>
                        )}
                      </div>
                    )}
                  </div>
                );
              })()}
          </div>
        )}
      </div>
    </>
  );
};
