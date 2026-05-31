import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";
import { commands } from "../bindings";
import { unwrap } from "../lib/unwrap";
import type { RelayWithPing, Server } from "../bindings";

interface ServerUpdateEvent {
  servers: Server[];
}

interface ServerErrorEvent {
  error: string;
}

type PingMap = Partial<Record<string, number | null>>;

interface ServerPingUpdate {
  pings: PingMap;
}

interface ServerStore {
  servers: Server[];
  loading: boolean;
  error: string | null;
  relays: RelayWithPing[];
  selectedRelay: string;
  relaysReady: boolean;
  lastUpdated: number | null;
  pings: PingMap;

  setSelectedRelay: (id: string) => void;
  initListener: () => Promise<UnlistenFn>;
  initRelays: () => Promise<UnlistenFn>;
}

const relaysResolved = (relays: RelayWithPing[]): boolean => {
  const hasValidPing = relays.some((r) => r.ping !== null && !r.checking);
  const allDone = relays.length > 0 && relays.every((r) => !r.checking);
  return hasValidPing || allDone;
};

export const useServerStore = create<ServerStore>()((set) => ({
  servers: [],
  loading: true,
  error: null,
  relays: [],
  selectedRelay: "",
  relaysReady: false,
  lastUpdated: null,
  pings: {},

  setSelectedRelay: async (selectedRelay) => {
    set({ selectedRelay });
    await commands.setSelectedRelay(selectedRelay);
  },

  initListener: async () => {
    try {
      const servers = unwrap(await commands.getServers());
      if (servers.length > 0) {
        set({ servers, loading: false, error: null, lastUpdated: Date.now() });
      }
    } catch (err) {
      console.error("Failed to get initial servers:", err);
    }

    const unlistenUpdate = await listen<ServerUpdateEvent>(
      "servers-updated",
      (event) => {
        set({ servers: event.payload.servers, loading: false, error: null, lastUpdated: Date.now() });
      }
    );

    const unlistenError = await listen<ServerErrorEvent>(
      "servers-error",
      (event) => {
        set({ error: event.payload.error, loading: false });
      }
    );

    try {
      const pings = unwrap(await commands.getServerPings());
      set({ pings });
    } catch (err) {
      console.error("Failed to get initial pings:", err);
    }

    const unlistenPings = await listen<ServerPingUpdate>(
      "server-pings-updated",
      (event) => {
        set({ pings: event.payload.pings });
      }
    );

    return () => {
      unlistenUpdate();
      unlistenError();
      unlistenPings();
    };
  },

  initRelays: async () => {
    try {
      const relays = unwrap(await commands.getRelays());
      const ready = relaysResolved(relays);
      set({ relays, relaysReady: ready });

      const selectedRelay = unwrap(await commands.getSelectedRelay());
      set({ selectedRelay });
    } catch (err) {
      console.error("Failed to get initial relays:", err);
    }

    const unlistenRelaysUpdated = await listen<RelayWithPing[]>(
      "relays-updated",
      (event) => {
        const relays = event.payload;
        const isReady = relaysResolved(relays);
        set({ relays, relaysReady: isReady });
      }
    );

    const unlistenRelaySelected = await listen<string>(
      "relay-selected",
      (event) => {
        set({ selectedRelay: event.payload, relaysReady: true });
      }
    );

    return () => {
      unlistenRelaysUpdated();
      unlistenRelaySelected();
    };
  },
}));
