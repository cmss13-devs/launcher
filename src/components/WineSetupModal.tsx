import { useTranslation } from "react-i18next";
import type { RenderingPipeline, WineStatus } from "../bindings";
import type { WineSetupProgress } from "../types";
import { Modal, ModalCloseButton, ModalContent, ModalSpinner } from "./Modal";

interface WineSetupModalProps {
  visible: boolean;
  status: WineStatus;
  progress: WineSetupProgress | null;
  isSettingUp: boolean;
  renderingPipeline: RenderingPipeline;
  onSetup: (pipeline: RenderingPipeline) => void;
  onRenderingPipelineChange: (pipeline: RenderingPipeline) => void;
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
  const { t } = useTranslation();
  return (
    <ModalContent title={t("wine.errorTitle")}>
      <p>
        {status.error || t("wine.errorDefault")}
      </p>
      <div>
        <button type="button" className="button" onClick={onRetry}>
          {t("common.retry")}
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
  const { t } = useTranslation();
  const displayProgress = progress?.progress ?? 0;
  const displayMessage =
    progress?.message ?? t("wine.setupStarting");

  return (
    <ModalContent title={t("wine.setupTitle")}>
      <p>{displayMessage}</p>
      <div className="wine-progress-bar">
        <div
          className="wine-progress-fill"
          style={{ width: `${displayProgress}%` }}
        />
      </div>
      <p className="wine-progress-percent">{displayProgress}%</p>
      <p>{t("wine.setupWarning")}</p>
      <ModalSpinner />
    </ModalContent>
  );
};

const SetupRequiredContent = ({
  renderingPipeline,
  onRenderingPipelineChange,
  onSetup,
}: {
  renderingPipeline: RenderingPipeline;
  onRenderingPipelineChange: (pipeline: RenderingPipeline) => void;
  onSetup: (pipeline: RenderingPipeline) => void;
}) => {
  const { t } = useTranslation();
  return (
    <ModalContent title={t("wine.setupRequired")}>
      <p>{t("wine.setupRequiredDesc")}</p>
      <div className="wine-rendering-pipeline">
        <h4>{t("wine.renderingPipeline")}</h4>
        <p className="settings-description">{t("wine.renderingPipelineDesc")}</p>
        <div className="theme-options">
          <label className={`theme-option ${renderingPipeline === "dxvk" ? "selected" : ""}`}>
            <input
              type="radio"
              name="setupRenderingPipeline"
              value="dxvk"
              checked={renderingPipeline === "dxvk"}
              onChange={() => onRenderingPipelineChange("dxvk")}
            />
            <div className="theme-info">
              <span className="theme-name">{t("wine.dxvkName")}</span>
              <span className="theme-desc">{t("wine.dxvkDesc")}</span>
            </div>
          </label>
          <label className={`theme-option ${renderingPipeline === "wined3d" ? "selected" : ""}`}>
            <input
              type="radio"
              name="setupRenderingPipeline"
              value="wined3d"
              checked={renderingPipeline === "wined3d"}
              onChange={() => onRenderingPipelineChange("wined3d")}
            />
            <div className="theme-info">
              <span className="theme-name">{t("wine.wined3dName")}</span>
              <span className="theme-desc">{t("wine.wined3dDesc")}</span>
            </div>
          </label>
        </div>
      </div>
      <div>
        <button type="button" className="button" onClick={() => onSetup(renderingPipeline)}>
          {t("wine.startSetup")}
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
  const { t } = useTranslation();
  return (
    <ModalContent title={t("wine.setupFailed")}>
      <p>{error}</p>
      <p>{t("wine.youCanTry")}</p>
      <ul>
        <li>{t("wine.resetSuggestion")}</li>
        <li>{t("wine.checkLogs")}</li>
      </ul>
      <div>
        <button type="button" className="button" onClick={onRetry}>
          {t("common.tryAgain")}
        </button>
      </div>
    </ModalContent>
  );
};

const SetupCompleteContent = ({ onClose }: { onClose: () => void }) => {
  const { t } = useTranslation();
  return (
    <ModalContent title={t("wine.setupComplete")}>
      <div>
        <p>{t("wine.setupCompleteMsg")}</p>
      </div>
      <div>
        <button type="button" className="button" onClick={onClose}>
          {t("wine.continue")}
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
  renderingPipeline,
  onSetup,
  onRenderingPipelineChange,
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
        <SetupRequiredContent
          renderingPipeline={renderingPipeline}
          onRenderingPipelineChange={onRenderingPipelineChange}
          onSetup={onSetup}
        />
      )}
    </Modal>
  );
};
