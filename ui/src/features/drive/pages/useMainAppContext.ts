import { createContext, useContext } from "react";

export const MainAppUiStateContext = createContext(null);
export const MainAppUiActionsContext = createContext(null);
export const DriveControllerContext = createContext(null);

function useRequiredContext(context, name) {
  const value = useContext(context);
  if (!value) {
    throw new Error(`${name} must be used within MainAppProvider`);
  }
  return value;
}

export function useMainAppUiStateContext() {
  return useRequiredContext(MainAppUiStateContext, "useMainAppUiStateContext");
}

export function useMainAppUiActions() {
  return useRequiredContext(MainAppUiActionsContext, "useMainAppUiActions");
}

export function useDriveControllerContext() {
  return useRequiredContext(DriveControllerContext, "useDriveControllerContext");
}
