type FolderLike = {
  id: number | string;
  parent_id?: number | string | null;
};

export function buildBreadcrumbs<T extends FolderLike>(
  currentFolderId: number | string | null,
  folders: T[],
): T[] {
  const folderMap = new Map<number, T>();
  folders.forEach((folder) => folderMap.set(Number(folder.id), folder));

  const list: T[] = [];
  const visited = new Set<number | string>();
  let tempId = currentFolderId;

  while (tempId && !visited.has(tempId)) {
    visited.add(tempId);
    const folder = folderMap.get(Number(tempId));
    if (!folder) break;
    list.unshift(folder);
    tempId = folder.parent_id ?? null;
  }

  return list;
}
