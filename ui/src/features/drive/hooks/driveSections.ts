export const DRIVE_SECTION_HOME = "home";
export const DRIVE_SECTION_MY = "mydrive";
export const DRIVE_SECTION_SHARED = "shareddrive";
export const DRIVE_SECTION_RECENT = "recent";
export const DRIVE_SECTION_STARRED = "starred";
export const DRIVE_SECTION_TRANSFERS = "transfers";
export const DRIVE_SECTION_TRASH = "trash";

export function getDriveScopeForSection(section) {
  if (section === DRIVE_SECTION_SHARED) {
    return "shared";
  }
  if (section === DRIVE_SECTION_MY) {
    return "my";
  }
  return null;
}

export function isScopedDriveSection(section) {
  return section === DRIVE_SECTION_MY || section === DRIVE_SECTION_SHARED;
}

export function getRootDriveSection(section) {
  if (section === DRIVE_SECTION_SHARED) {
    return DRIVE_SECTION_SHARED;
  }
  return DRIVE_SECTION_MY;
}

export function getItemDriveScope(item) {
  return item?.drive_scope || item?.driveScope || null;
}
