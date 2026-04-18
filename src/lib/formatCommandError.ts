import i18next from "i18next";
import type { CommandError } from "../bindings";

export function formatCommandError(err: CommandError): string {
  const t = i18next.t.bind(i18next);
  switch (err.type) {
    case "network":
      return t("errors.network", { detail: err.data });
    case "not_authenticated":
      return t("errors.not_authenticated");
    case "token_expired":
      return t("errors.token_expired");
    case "requires_2fa":
      return t("errors.requires_2fa");
    case "invalid_credentials":
      return t("errors.invalid_credentials");
    case "account_locked":
      return t("errors.account_locked");
    case "requires_linking":
      return t("errors.requires_linking", { url: err.data.url });
    case "not_found":
      return t("errors.not_found", { detail: err.data });
    case "io":
      return t("errors.io", { detail: err.data });
    case "not_configured":
      return t("errors.not_configured", { feature: err.data.feature });
    case "unsupported_platform":
      return t("errors.unsupported_platform", { feature: err.data.feature, platform: err.data.platform });
    case "busy":
      return t("errors.busy", { operation: err.data.operation });
    case "cancelled":
      return t("errors.cancelled", { operation: err.data.operation });
    case "timeout":
      return t("errors.timeout", { operation: err.data.operation });
    case "internal":
      return t("errors.internal", { detail: err.data });
    case "webview":
      return t("errors.webview", { detail: err.data });
    case "invalid_response":
      return t("errors.invalid_response", { detail: err.data });
    case "invalid_input":
      return t("errors.invalid_input", { detail: err.data });
  }
}
