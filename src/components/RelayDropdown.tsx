import { faChevronDown, faChevronUp } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import type { RelayWithPing } from "../types";

interface RelayDropdownProps {
  relays: RelayWithPing[];
  selectedRelay: string;
  isOpen: boolean;
  onToggle: () => void;
  onSelect: (relayId: string) => void;
}

export const RelayDropdown = ({
  relays,
  selectedRelay,
  isOpen,
  onToggle,
  onSelect,
}: RelayDropdownProps) => {
  const allChecking = relays.length > 0 && relays.every((r) => r.checking);
  const selectedRelayData = relays.find((r) => r.id === selectedRelay);

  let selectedRelayName: string;
  if (allChecking) {
    selectedRelayName = "PINGING...";
  } else if (selectedRelayData) {
    selectedRelayName = selectedRelayData.name;
  } else {
    selectedRelayName = "Select";
  }

  return (
    <div className="relay-dropdown">
      <button
        type="button"
        className="relay-dropdown-button"
        onClick={onToggle}
        disabled={allChecking}
      >
        <span className="relay-dropdown-label">Relay:</span>
        <span className="relay-dropdown-value">{selectedRelayName}</span>
        <span className="relay-dropdown-arrow">
          <FontAwesomeIcon icon={isOpen ? faChevronUp : faChevronDown} />
        </span>
      </button>
      {isOpen && (
        <div className="relay-dropdown-menu">
          {[...relays]
            .sort((a, b) => {
              if (a.ping === null && b.ping === null) return 0;
              if (a.ping === null) return 1;
              if (b.ping === null) return -1;
              return a.ping - b.ping;
            })
            .map((relay) => {
            const isDisabled = relay.ping === null && !relay.checking;
            const isSelected = selectedRelay === relay.id;

            return (
              <label
                key={relay.id}
                className={`relay-option ${isSelected ? "selected" : ""} ${isDisabled ? "disabled" : ""}`}
              >
                <input
                  type="radio"
                  name="relay"
                  value={relay.id}
                  checked={isSelected}
                  onChange={() => onSelect(relay.id)}
                  disabled={isDisabled}
                />
                <span className="relay-name">{relay.name}</span>
                <span className="relay-ping">
                  {relay.checking
                    ? "..."
                    : relay.ping !== null
                      ? `${relay.ping}ms`
                      : "N/A"}
                </span>
              </label>
            );
          })}
        </div>
      )}
    </div>
  );
};
