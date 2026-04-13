import { useState, useEffect } from "react";
import { commands } from "../bindings";

export type GameConnectionState = "idle" | "connecting" | "connected" | "restarting";

const CONNECTION_TIMEOUT_SECONDS = 30;

interface GameConnectionModalProps {
  visible: boolean;
  state: GameConnectionState;
  serverName: string | null;
  restartReason?: string | null;
  onClose: () => void;
}

export const GameConnectionModal = ({
  visible,
  state,
  serverName,
  restartReason,
  onClose,
}: GameConnectionModalProps) => {
  const [closing, setClosing] = useState(false);
  const [timeRemaining, setTimeRemaining] = useState(CONNECTION_TIMEOUT_SECONDS);

  useEffect(() => {
    if (state === "connecting" || state === "restarting") {
      setTimeRemaining(CONNECTION_TIMEOUT_SECONDS);
      const interval = setInterval(() => {
        setTimeRemaining((prev) => (prev > 0 ? prev - 1 : 0));
      }, 1000);
      return () => clearInterval(interval);
    }
  }, [state]);

  if (!visible) return null;

  const handleCloseGame = async () => {
    setClosing(true);
    try {
      await commands.killGame();
      onClose();
    } catch (err) {
      console.error("Failed to close game:", err);
    } finally {
      setClosing(false);
    }
  };

  const getStatusText = () => {
    switch (state) {
      case "restarting":
        return `Restarting ${serverName}...`;
      case "connecting":
        return `Connecting to ${serverName}...`;
      default:
        return `Connected to ${serverName}`;
    }
  };

  const showSpinner = state === "connecting" || state === "restarting";
  const progressPercent = ((CONNECTION_TIMEOUT_SECONDS - timeRemaining) / CONNECTION_TIMEOUT_SECONDS) * 100;

  return (
    <div className="game-connection-overlay">
      <div className="game-connection-modal">
        <div className="game-connection-status">
          {showSpinner && <div className="game-connection-spinner" />}
          <h2>{getStatusText()}</h2>
          {state === "restarting" && restartReason && (
            <p className="game-connection-reason">{restartReason}</p>
          )}
          {showSpinner && (
            <div className="game-connection-progress">
              <div
                className="game-connection-progress-bar"
                style={{ width: `${progressPercent}%` }}
              />
            </div>
          )}
        </div>
        <button
          type="button"
          className="button"
          onClick={handleCloseGame}
          disabled={closing}
        >
          {closing ? "Closing..." : "Close Game"}
        </button>
      </div>
    </div>
  );
};
