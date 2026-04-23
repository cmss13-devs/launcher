import { useTranslation } from "react-i18next";
import { Modal, ModalSpinner } from "./Modal";

interface ByondLoginModalProps {
  visible: boolean;
  loggingIn: boolean;
  onClose: () => void;
}

export const ByondLoginModal = ({ visible, loggingIn, onClose }: ByondLoginModalProps) => {
  const { t } = useTranslation();
  const showSpinner = !visible && loggingIn;

  return (
    <Modal
      visible={visible || showSpinner}
      onClose={onClose}
      className="auth-modal byond-login-modal"
      closeOnOverlayClick={showSpinner}
      title={t("settings.loginToByond")}
    >
      {showSpinner && (
        <div className="byond-login-spinner">
          <p>{t("settings.byondWaiting")}</p>
          <ModalSpinner />
        </div>
      )}
    </Modal>
  );
};
