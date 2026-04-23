import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { Modal, ModalContent, ModalSpinner } from "./Modal";

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

  const titleMap: Record<SteamAuthModalState, string> = {
    idle: t("auth.steamAuth"),
    loading: t("auth.authenticating"),
    linking: t("auth.steamLinkingTitle"),
    error: t("auth.authFailed"),
  };

  return (
    <Modal visible={visible} onClose={onClose} title={titleMap[state]}>
      {state === "idle" && (
        <ModalContent>
          <p>{t("auth.steamAuthenticating")}</p>
          <ModalSpinner />
        </ModalContent>
      )}
      {state === "loading" && (
        <ModalContent>
          <p>{t("auth.steamValidating")}</p>
          <ModalSpinner />
        </ModalContent>
      )}
      {state === "linking" && (
        <ModalContent>
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
        <ModalContent>
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
