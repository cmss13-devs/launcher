import type { CommandError } from "../bindings";

export function formatCommandError(err: CommandError): string {
  switch (err.type) {
    case "network":
      return `Network error: ${err.data}`;
    case "not_authenticated":
      return "You must log in to continue.";
    case "token_expired":
      return "Your session has expired. Please log in again.";
    case "requires_2fa":
      return "Two-factor authentication required.";
    case "invalid_credentials":
      return "Invalid username or password.";
    case "account_locked":
      return "This account is locked.";
    case "requires_linking":
      return `Account linking required: ${err.data.url}`;
    case "not_found":
      return `Not found: ${err.data}`;
    case "io":
      return `I/O error: ${err.data}`;
    case "not_configured":
      return `${err.data.feature} is not configured.`;
    case "unsupported_platform":
      return `${err.data.feature} is not supported on ${err.data.platform}.`;
    case "busy":
      return `${err.data.operation} is already in progress.`;
    case "cancelled":
      return `${err.data.operation} was cancelled.`;
    case "timeout":
      return `${err.data.operation} timed out.`;
    case "internal":
      return `Internal error: ${err.data}`;
    case "webview":
      return `Webview error: ${err.data}`;
    case "invalid_response":
      return `Invalid response: ${err.data}`;
    case "invalid_input":
      return `Invalid input: ${err.data}`;
  }
}
