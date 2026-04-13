import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { LauncherConfig, Server } from "../bindings";

export function useServerFilters(servers: Server[], config: LauncherConfig | null) {
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());
  const [show18Plus, setShow18Plus] = useState(false);
  const [showOffline, setShowOffline] = useState(false);
  const [showHubStatus, setShowHubStatus] = useState(false);
  const [showSingleplayer, setShowSingleplayer] = useState(false);
  const [filtersOpen, setFiltersOpen] = useState(false);
  const filtersRef = useRef<HTMLDivElement>(null);

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
    setSelectedTags((prev) => {
      const next = new Set(prev);
      if (on) next.add(tag);
      else next.delete(tag);
      return next;
    });
  }, []);

  const categories = useMemo(() => {
    const tagSet = new Set<string>();
    for (const server of servers) {
      if (server.tags) for (const tag of server.tags) tagSet.add(tag);
    }
    const sorted = Array.from(tagSet).sort();

    const pvpIndex = sorted.findIndex((t) => t.toLowerCase() === "pvp");
    if (pvpIndex > 0) {
      const [pvp] = sorted.splice(pvpIndex, 1);
      sorted.unshift(pvp);
    }

    if (config?.features.singleplayer) sorted.push("sandbox");
    return sorted;
  }, [servers, config?.features.singleplayer]);

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

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter((server) =>
        server.name.toLowerCase().includes(query),
      );
    }

    if (!show18Plus) {
      filtered = filtered.filter((server) => !server.is_18_plus);
    }

    if (!showOffline && !config?.features.show_offline_servers) {
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
    searchQuery,
    show18Plus,
    showOffline,
    config?.features.show_offline_servers,
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
    showSingleplayer,
    setShowSingleplayer,
    filtersOpen,
    setFiltersOpen,
    filtersRef,
    categories,
    hasOffline,
    hasHubStatus,
    filteredServers,
  };
}
