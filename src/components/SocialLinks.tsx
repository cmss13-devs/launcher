import { faDiscord, faTwitch } from "@fortawesome/free-brands-svg-icons";
import { faBook, faComments } from "@fortawesome/free-solid-svg-icons";
import type { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { commands } from "../bindings";
import type { SocialLink } from "../bindings";

const iconMap: Record<string, IconDefinition> = {
  discord: faDiscord,
  twitch: faTwitch,
  forums: faComments,
  wiki: faBook,
};

interface SocialLinksProps {
  links: SocialLink[];
}

export const SocialLinks = ({ links }: SocialLinksProps) => {
  if (links.length === 0) {
    return null;
  }

  const handleClick = async (url: string) => {
    await commands.openUrl(url);
  };

  return (
    <div className="social-links">
      {links.map((link) => {
        const icon = iconMap[link.icon];
        if (!icon) return null;
        return (
          <button
            key={link.name}
            type="button"
            className="social-link-button"
            onClick={() => handleClick(link.url)}
            title={link.name}
          >
            <FontAwesomeIcon icon={icon} className="social-link-icon" />
          </button>
        );
      })}
    </div>
  );
};
