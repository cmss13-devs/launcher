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
        <ModalContent title="Steam Authentication">
          <p>Authenticating with Steam...</p>
          <ModalSpinner />
        </ModalContent>
      )}
      {state === "loading" && (
        <ModalContent title="Authenticating...">
          <p>Validating your Steam account...</p>
          <ModalSpinner />
        </ModalContent>
      )}
      {state === "linking" && (
        <ModalContent title="Account Linking">
          <p>No {authProviderName} account is linked to your Steam account.</p>
          <p>Do you have an existing {authProviderName} account?</p>
          <div className="auth-modal-buttons">
            <button type="button" className="button" onClick={openLinkingUrl}>
              Yes, link my account
            </button>
            <button
              type="button"
              className="button-secondary"
              onClick={() => onAuthenticate(true)}
            >
              No, start now
            </button>
          </div>
        </ModalContent>
      )}
      {state === "error" && (
        <ModalContent title="Authentication Failed">
          <p className="auth-error-message">{error}</p>
          <button
            type="button"
            className="button"
            onClick={() => onAuthenticate(false)}
          >
            Try Again
          </button>
        </ModalContent>
      )}
    </Modal>
  );
};
