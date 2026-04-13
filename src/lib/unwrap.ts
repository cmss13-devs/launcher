import type { CommandError } from "../bindings";
import { formatCommandError } from "./formatCommandError";

function isCommandError(e: unknown): e is CommandError {
  return typeof e === "object" && e !== null && "type" in e;
}

export function unwrap<T>(r: { status: "ok"; data: T } | { status: "error"; error: unknown }): T {
  if (r.status === "error") {
    if (isCommandError(r.error)) throw new Error(formatCommandError(r.error));
    throw new Error(String(r.error));
  }
  return r.data;
}
