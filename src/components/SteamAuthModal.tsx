import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { Modal, ModalCloseButton, ModalContent, ModalSpinner } from "./Modal";

export type SteamAuthModalState = "idle" | "loading" | "linking" | "error";

interface SteamAuthModalProps {
  visible: boolean;
  state: SteamAuthModalState;
  error?: string;
  linkingUrl?: string;
  authProviderName: string;
  onAuthenticate: (createAccount: boolean) => void;
  onClose: () => void;
}

export const SteamAuthModal = ({
  visible,
  state,
  error,
  linkingUrl,
  authProviderName,
  onAuthenticate,
  onClose,
}: SteamAuthModalProps) => {
  const { t } = useTranslation();
  const openLinkingUrl = async () => {
    if (linkingUrl) {
      await commands.openUrl(linkingUrl);
      onClose();
    }
  };

  return (
    <Modal visible={visible} onClose={onClose}>
      <ModalCloseButton onClick={onClose} />
      {state === "idle" && (
        <ModalContent title={t("auth.steamAuth")}>
          <p>{t("auth.steamAuthenticating")}</p>
          <ModalSpinner />
        </ModalContent>
      )}
      {state === "loading" && (
        <ModalContent title={t("auth.authenticating")}>
          <p>{t("auth.steamValidating")}</p>
          <ModalSpinner />
        </ModalContent>
      )}
      {state === "linking" && (
        <ModalContent title={t("auth.steamLinkingTitle")}>
          <p>{t("auth.steamNoAccount", { provider: authProviderName })}</p>
          <p>{t("auth.steamHaveAccount", { provider: authProviderName })}</p>
          <div className="auth-modal-buttons">
            <button type="button" className="button" onClick={openLinkingUrl}>
              {t("auth.steamYesLink")}
            </button>
            <button
              type="button"
              className="button-secondary"
              onClick={() => onAuthenticate(true)}
            >
              {t("auth.steamNoStart")}
            </button>
          </div>
        </ModalContent>
      )}
      {state === "error" && (
        <ModalContent title={t("auth.authFailed")}>
          <p className="auth-error-message">{error}</p>
          <button
            type="button"
            className="button"
            onClick={() => onAuthenticate(false)}
          >
            {t("common.tryAgain")}
          </button>
        </ModalContent>
      )}
    </Modal>
  );
};
