import { useTranslation } from "react-i18next";
import { Modal, ModalCloseButton } from "./Modal";

interface ByondLoginModalProps {
  visible: boolean;
  onClose: () => void;
}

export const ByondLoginModal = ({ visible, onClose }: ByondLoginModalProps) => {
  const { t } = useTranslation();

  return (
    <Modal visible={visible} onClose={onClose} className="auth-modal byond-login-modal" closeOnOverlayClick>
      <ModalCloseButton onClick={onClose} />
      <h2>{t("settings.loginToByond")}</h2>
    </Modal>
  );
};
