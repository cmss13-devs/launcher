import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import type { Server } from "../bindings";
import { Modal, ModalContent } from "./Modal";
import { LinkIconMap } from "./ServerItem";
import { faGlobe } from "@fortawesome/free-solid-svg-icons";
import { useId } from "react";
import { useTranslation } from "react-i18next";
import { useSettingsStore } from "../stores";

interface WhitelistModalProps {
  visible: boolean;
  onClose: () => void;
  onJoin: () => void;
  linkClick: (link: string | null) => void;
  server?: Server;
}

export const WhitelistRequiredModal = ({
  visible,
  onClose,
  onJoin,
  linkClick,
  server,
}: WhitelistModalProps) => {
  const { t } = useTranslation();
  const id = useId();
  const isWhitelisted = useSettingsStore((s) =>
    server?.id ? s.whitelistedServers.has(server.id) : false,
  );

  if (!server || !server.whitelisted) return;

  const handleToggle = async (checked: boolean) => {
    if (server.id) {
      await useSettingsStore.getState().setUserWhitelisted(server.id, checked);
    }
  };

  const handleJoin = () => {
    if (!isWhitelisted) return;
    onClose();
    onJoin();
  };

  return (
    <Modal visible={visible} onClose={onClose} title="Whitelist Required">
      <ModalContent>
        <p>This server requires you to be whitelisted before joining.</p>
        {server.whitelisted.description && (
          <p className="whitelist-description">{server.whitelisted.description}</p>
        )}
        {server.whitelisted.link && (
          <div className="server-links">
            <button
              type="button"
              className="button-secondary server-link-button"
              onClick={() => {
                onClose();
                linkClick(server.whitelisted?.link?.link ?? "");
              }}
              title={server.whitelisted.link.type}
            >
              <FontAwesomeIcon
                icon={LinkIconMap[server.whitelisted.link.type] ?? faGlobe}
              />
              <span>
                {server.whitelisted.link.type.charAt(0).toUpperCase() +
                  server.whitelisted.link.type.slice(1)}
              </span>
            </button>
          </div>
        )}
        <div className="styled-checkbox" style={{ marginTop: 12 }}>
          <input
            type="checkbox"
            id={id}
            checked={isWhitelisted}
            onChange={(e) => handleToggle(e.target.checked)}
          />
          <label htmlFor={id}>I am whitelisted on this server.</label>
        </div>
        <div style={{ marginTop: 12 }}>
          <button
            type="button"
            className="button"
            disabled={!isWhitelisted}
            onClick={handleJoin}
          >
            {t("common.join")} {server.name}
          </button>
        </div>
      </ModalContent>
    </Modal>
  );
};
