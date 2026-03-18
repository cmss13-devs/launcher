import type { WineSetupProgress, WineStatus } from "../types";
import { Modal, ModalCloseButton, ModalContent, ModalSpinner } from "./Modal";

interface WineSetupModalProps {
  visible: boolean;
  status: WineStatus;
  progress: WineSetupProgress | null;
  isSettingUp: boolean;
  onSetup: () => void;
  onClose: () => void;
  onRetry: () => void;
}

const WineErrorContent = ({
  status,
  onRetry,
}: {
  status: WineStatus;
  onRetry: () => void;
}) => {
  return (
    <ModalContent title="Wine Error">
      <p>
        {status.error ||
          "Failed to initialize bundled Wine. Please try again or check the logs for details."}
      </p>
      <div>
        <button type="button" className="button" onClick={onRetry}>
          Retry
        </button>
      </div>
    </ModalContent>
  );
};

const SetupProgressContent = ({
  progress,
}: {
  progress: WineSetupProgress | null;
}) => {
  const displayProgress = progress?.progress ?? 0;
  const displayMessage =
    progress?.message ?? "Starting Wine environment setup...";

  return (
    <ModalContent title="Setting Up Wine Environment">
      <p>{displayMessage}</p>
      <div className="wine-progress-bar">
        <div
          className="wine-progress-fill"
          style={{ width: `${displayProgress}%` }}
        />
      </div>
      <p className="wine-progress-percent">{displayProgress}%</p>
      <p>This may take several minutes. Please do not close the launcher.</p>
      <ModalSpinner />
    </ModalContent>
  );
};

const SetupRequiredContent = ({ onSetup }: { onSetup: () => void }) => {
  return (
    <ModalContent title="Wine Setup Required">
      <p>
        Launching DreamSeeker requires a one-time setup, may take up to 5
        minutes.
      </p>
      <div>
        <button type="button" className="button" onClick={onSetup}>
          Start Setup
        </button>
      </div>
    </ModalContent>
  );
};

const SetupErrorContent = ({
  error,
  onRetry,
}: {
  error: string;
  onRetry: () => void;
}) => {
  return (
    <ModalContent title="Setup Failed">
      <p>{error}</p>
      <p>You can try:</p>
      <ul>
        <li>Reset the Wine prefix from Settings</li>
        <li>Check the logs for more details</li>
      </ul>
      <div>
        <button type="button" className="button" onClick={onRetry}>
          Try Again
        </button>
      </div>
    </ModalContent>
  );
};

const SetupCompleteContent = ({ onClose }: { onClose: () => void }) => {
  return (
    <ModalContent title="Setup Complete">
      <div>
        <p>Wine setup complete!</p>
      </div>
      <div>
        <button type="button" className="button" onClick={onClose}>
          Continue
        </button>
      </div>
    </ModalContent>
  );
};

export const WineSetupModal = ({
  visible,
  status,
  progress,
  isSettingUp,
  onSetup,
  onClose,
  onRetry,
}: WineSetupModalProps) => {
  const wineError = status.error || !status.installed;
  const setupComplete =
    status.prefix_initialized &&
    status.webview2_installed &&
    !isSettingUp &&
    progress?.stage === "complete";
  const setupFailed = progress?.stage === "error";

  const canClose = !isSettingUp;

  return (
    <Modal visible={visible} onClose={canClose ? onClose : () => {}}>
      {canClose && <ModalCloseButton onClick={onClose} />}

      {wineError ? (
        <WineErrorContent status={status} onRetry={onRetry} />
      ) : isSettingUp ? (
        <SetupProgressContent progress={progress} />
      ) : setupFailed ? (
        <SetupErrorContent
          error={progress?.message ?? "Unknown error"}
          onRetry={onRetry}
        />
      ) : setupComplete ? (
        <SetupCompleteContent onClose={onClose} />
      ) : (
        <SetupRequiredContent onSetup={onSetup} />
      )}
    </Modal>
  );
};
