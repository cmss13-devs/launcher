import type { LauncherFeatures } from "../bindings";
import type { useServerFilters } from "../hooks/useServerFilters";

type FilterState = ReturnType<typeof useServerFilters>;

interface ServerFilterPanelProps {
  features: LauncherFeatures;
  filters: FilterState;
  serverCount: number;
  playerCount: number;
}

export const ServerFilterPanel = ({
  features,
  filters,
  serverCount,
  playerCount,
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
    showSingleplayer,
    setShowSingleplayer,
    filtersOpen,
    setFiltersOpen,
    filtersRef,
    categories,
    hasOffline,
    hasHubStatus,
  } = filters;

  const tagCategories = categories.filter((c) => c !== "sandbox");
  const showHeader =
    features.server_stats ||
    features.server_search ||
    features.server_filters ||
    features.singleplayer;

  if (!showHeader) return null;

  return (
    <div className="server-header">
      {features.server_stats && (
        <div className="server-stats">
          <span className="stat-label">Servers</span>
          <span className="stat-value">{serverCount}</span>
          <span className="stat-label">Players</span>
          <span className="stat-value">{playerCount}</span>
        </div>
      )}
      {(features.server_search || features.server_filters || features.singleplayer) && (
        <div className="server-controls">
          {features.server_search && (
            <input
              type="text"
              className="search-input"
              placeholder="Search servers..."
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
                Filters{selectedTags.size > 0 ? ` (${selectedTags.size})` : ""}
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
                          <span>hub status</span>
                        </label>
                      )}
                      <label className="filter-checkbox">
                        <input
                          type="checkbox"
                          checked={show18Plus}
                          onChange={(e) => setShow18Plus(e.target.checked)}
                        />
                        <span>18+ servers</span>
                      </label>
                      {hasOffline && (
                        <label className="filter-checkbox">
                          <input
                            type="checkbox"
                            checked={showOffline}
                            onChange={(e) => setShowOffline(e.target.checked)}
                          />
                          <span>offline servers</span>
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
          {features.singleplayer && (
            <button
              type="button"
              className={`filters-button${showSingleplayer ? " active" : ""}`}
              onClick={() => setShowSingleplayer(!showSingleplayer)}
            >
              Singleplayer
            </button>
          )}
        </div>
      )}
    </div>
  );
};
