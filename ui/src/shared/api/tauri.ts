import { listen } from "@tauri-apps/api/event";

// ponytail: single browser-vs-Tauri gate. call.ts duplicates this predicate
// inline — keep both in sync; everything else imports from here.
export const isTauriRuntime = (): boolean =>
  typeof globalThis.window !== "undefined" && Reflect.has(globalThis, "__TAURI_INTERNALS__");

type Unlisten = () => void;

export const safeListen = <T>(
  event: string,
  handler: (e: { payload: T }) => void,
): Promise<Unlisten> => {
  if (!isTauriRuntime()) return Promise.resolve(() => {});
  return listen<T>(event, handler);
};
