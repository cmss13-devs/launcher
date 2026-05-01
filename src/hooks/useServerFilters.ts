import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { LauncherConfig, Server } from "../bindings";
import { useSettingsStore, type StoredFilters } from "../stores/settingsStore";

export function useServerFilters(servers: Server[], config: LauncherConfig | null) {
  const filters = useSettingsStore((s) => s.filters);
  const saveFilters = useSettingsStore((s) => s.saveFilters);

  const searchQuery = filters.searchQuery;
  const [filtersOpen, setFiltersOpen] = useState(false);
  const filtersRef = useRef<HTMLDivElement>(null);

  const selectedTags = filters.tags;
  const show18Plus = filters.show18Plus;
  const showOffline = filters.showOffline ?? config?.features.show_offline_servers ?? false;
  const showHubStatus = filters.showHubStatus;
  const selectedRegions = filters.regions;
  const selectedLanguages = filters.languages;

  const updateFilters = useCallback((patch: Partial<StoredFilters>) => {
    saveFilters({ ...filters, ...patch });
  }, [filters, saveFilters]);

  const setShow18Plus = useCallback((value: boolean) => {
    updateFilters({ show18Plus: value });
  }, [updateFilters]);

  const setShowOffline = useCallback((value: boolean) => {
    updateFilters({ showOffline: value });
  }, [updateFilters]);

  const setSearchQuery = useCallback((value: string) => {
    updateFilters({ searchQuery: value });
  }, [updateFilters]);

  const setShowHubStatus = useCallback((value: boolean) => {
    updateFilters({ showHubStatus: value });
  }, [updateFilters]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (filtersRef.current && !filtersRef.current.contains(event.target as Node)) {
        setFiltersOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const toggleTag = useCallback((tag: string, on: boolean) => {
    const next = new Set(filters.tags);
    if (on) next.add(tag);
    else next.delete(tag);
    saveFilters({ ...filters, tags: next });
  }, [filters, saveFilters]);

  const toggleRegion = useCallback((region: string, on: boolean) => {
    const next = new Set(filters.regions);
    if (on) next.add(region);
    else next.delete(region);
    saveFilters({ ...filters, regions: next });
  }, [filters, saveFilters]);

  const toggleLanguage = useCallback((language: string, on: boolean) => {
    const next = new Set(filters.languages);
    if (on) next.add(language);
    else next.delete(language);
    saveFilters({ ...filters, languages: next });
  }, [filters, saveFilters]);

  const categories = useMemo(() => {
    const tagSet = new Set<string>();
    for (const server of servers) {
      if (server.tags) for (const tag of server.tags) tagSet.add(tag);
    }
    tagSet.delete("18+");
    const sorted = Array.from(tagSet).sort();

    const pvpIndex = sorted.findIndex((t) => t.toLowerCase() === "pvp");
    if (pvpIndex > 0) {
      const [pvp] = sorted.splice(pvpIndex, 1);
      sorted.unshift(pvp);
    }

    if (config?.features.singleplayer) sorted.push("sandbox");
    return sorted;
  }, [servers, config?.features.singleplayer]);

  const regions = useMemo(() => {
    const regionSet = new Set<string>();
    for (const server of servers) {
      if (server.region) regionSet.add(server.region);
    }
    return Array.from(regionSet).sort();
  }, [servers]);

  const languages = useMemo(() => {
    const langSet = new Set<string>();
    for (const server of servers) {
      if (server.language) langSet.add(server.language);
    }
    return Array.from(langSet).sort();
  }, [servers]);

  const hasOffline = useMemo(
    () => servers.some((s) => s.status !== "available"),
    [servers],
  );
  const hasHubStatus = useMemo(
    () => servers.some((s) => (s.hub_status ?? "").length > 0),
    [servers],
  );

  const filteredServers = useMemo(() => {
    const seen = new Set<string>();
    const uniqueServers = servers.filter((server) => {
      if (seen.has(server.url)) return false;
      seen.add(server.url);
      return true;
    });

    let filtered =
      selectedTags.size > 0
        ? uniqueServers.filter((server) =>
            server.tags?.some((t) => selectedTags.has(t)),
          )
        : uniqueServers;

    if (selectedRegions.size > 0) {
      filtered = filtered.filter((server) =>
        server.region && selectedRegions.has(server.region),
      );
    }

    if (selectedLanguages.size > 0) {
      filtered = filtered.filter((server) =>
        server.language && selectedLanguages.has(server.language),
      );
    }

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter((server) =>
        server.name.toLowerCase().includes(query),
      );
    }

    if (!show18Plus) {
      filtered = filtered.filter((server) => !server.is_18_plus);
    }

    if (!showOffline) {
      filtered = filtered.filter((server) => server.status === "available");
    }

    return filtered.sort((a, b) => {
      const aOnline = a.status === "available";
      const bOnline = b.status === "available";
      if (aOnline !== bOnline) return aOnline ? -1 : 1;
      return (b.players ?? 0) - (a.players ?? 0);
    });
  }, [
    servers,
    selectedTags,
    selectedRegions,
    selectedLanguages,
    searchQuery,
    show18Plus,
    showOffline,
  ]);

  return {
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
    filteredServers,
  };
}
