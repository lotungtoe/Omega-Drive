import { createContext, useContext } from "react";

type UiState = Record<string, any>;
type UiActions = Record<string, any>;
type DriveController = Record<string, any>;

export const MainAppUiStateContext = createContext<UiState | null>(null);
export const MainAppUiActionsContext = createContext<UiActions | null>(null);
export const DriveControllerContext = createContext<DriveController | null>(null);

function useRequiredContext<T>(context: React.Context<T>, name: string): NonNullable<T> {
  const value = useContext(context);
  if (!value) {
    throw new Error(`${name} must be used within MainAppProvider`);
  }
  return value as NonNullable<T>;
}

export function useMainAppUiStateContext(): UiState {
  return useRequiredContext(MainAppUiStateContext, "useMainAppUiStateContext");
}

export function useMainAppUiActions(): UiActions {
  return useRequiredContext(MainAppUiActionsContext, "useMainAppUiActions");
}

export function useDriveControllerContext(): DriveController {
  return useRequiredContext(DriveControllerContext, "useDriveControllerContext");
}
