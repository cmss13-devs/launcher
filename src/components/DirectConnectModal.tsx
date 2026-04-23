import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, DirectConnectInfo } from "../bindings";
import { formatCommandError } from "../lib/formatCommandError";
import { useConnect, useError } from "../hooks";
import { Modal } from "./Modal";

interface DirectConnectModalProps {
  visible: boolean;
  onClose: () => void;
}

export const DirectConnectModal = ({ visible, onClose }: DirectConnectModalProps) => {
  const { t } = useTranslation();
  const [address, setAddress] = useState("");
  const [resolving, setResolving] = useState(false);
  const [connectInfo, setConnectInfo] = useState<DirectConnectInfo | null>(null);
  const { showError } = useError();
  const { connectToAddress } = useConnect();

  const handleResolve = async () => {
    const trimmed = address.trim();
    if (!trimmed) return;

    setResolving(true);
    try {
      const result = await commands.resolveDirectConnect(trimmed);
      if (result.status === "error") {
        showError(formatCommandError(result.error));
        return;
      }
      const info = result.data;
      if (info.trust === "HubVerified" || info.trust === "HubKnown") {
        await doConnect(trimmed);
      } else {
        setConnectInfo(info);
      }
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err));
    } finally {
      setResolving(false);
    }
  };

  const doConnect = async (addr: string) => {
    const success = await connectToAddress(addr, "DirectConnect");
    if (success) {
      handleClose();
    }
  };

  const handleConfirm = async () => {
    await doConnect(address.trim());
  };

  const handleClose = () => {
    setConnectInfo(null);
    onClose();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !resolving && !connectInfo) {
      handleResolve();
    }
  };

  if (connectInfo) {
    return (
      <Modal
        visible={visible}
        onClose={handleClose}
        className="settings-modal"
        closeOnOverlayClick
        title={t("directConnect.title")}
      >
        <div className="modal-body">
          <div className="settings-section">
            {connectInfo.trust === "SelfReported" ? (
              <>
                <p className="settings-description">
                  {t("directConnect.selfReportedWarning")}
                </p>
                <p className="settings-description" style={{ opacity: 0.7 }}>
                  {t("directConnect.selfReportedDetail")}
                </p>
              </>
            ) : (
              <p className="settings-description">
                {t("directConnect.byondOnlyInfo")}
              </p>
            )}
          </div>
        </div>
        <div className="modal-footer">
          <button type="button" className="button-secondary" onClick={handleClose}>
            {t("common.cancel")}
          </button>
          <button type="button" className="button" onClick={handleConfirm}>
            {t("common.connect")}
          </button>
        </div>
      </Modal>
    );
  }

  return (
    <Modal
      visible={visible}
      onClose={handleClose}
      className="settings-modal"
      closeOnOverlayClick
      title={t("directConnect.title")}
    >
      <div className="modal-body">
        <div className="settings-section">
          <p className="settings-description">{t("directConnect.hint")}</p>
          <input
            type="text"
            className="search-input direct-connect-input"
            placeholder={t("directConnect.placeholder")}
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            onKeyDown={handleKeyDown}
            autoFocus
          />
        </div>
      </div>
      <div className="modal-footer">
        <button
          type="button"
          className="button"
          onClick={handleResolve}
          disabled={resolving || !address.trim()}
        >
          {resolving ? "..." : t("common.connect")}
        </button>
      </div>
    </Modal>
  );
};
