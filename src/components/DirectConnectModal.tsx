import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useConnect, useError } from "../hooks";
import { Modal, ModalCloseButton } from "./Modal";

interface DirectConnectModalProps {
  visible: boolean;
  onClose: () => void;
}

export const DirectConnectModal = ({ visible, onClose }: DirectConnectModalProps) => {
  const { t } = useTranslation();
  const [address, setAddress] = useState("");
  const [connecting, setConnecting] = useState(false);
  const { showError } = useError();
  const { connectToAddress } = useConnect();

  const handleConnect = async () => {
    const trimmed = address.trim();
    if (!trimmed) return;

    setConnecting(true);
    try {
      const success = await connectToAddress(trimmed, "DirectConnect");
      if (success) onClose();
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err));
    } finally {
      setConnecting(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !connecting) {
      handleConnect();
    }
  };

  return (
    <Modal
      visible={visible}
      onClose={onClose}
      className="settings-modal"
      closeOnOverlayClick
    >
      <div className="modal-header">
        <h2>{t("directConnect.title")}</h2>
        <ModalCloseButton onClick={onClose} />
      </div>
      <div className="settings-modal-content">
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
      <div className="settings-modal-footer">
        <button
          type="button"
          className="button"
          onClick={handleConnect}
          disabled={connecting || !address.trim()}
        >
          {connecting ? "..." : t("common.connect")}
        </button>
      </div>
    </Modal>
  );
};
