// Barrel — re-exports the unified API object for backward compatibility
// New code should import from the per-feature api modules directly.
import { DriveApi } from "../shared/api/DriveApi";
export type {
  DriveScope,
  IdLike,
  JsonRecord,
  Nullable,
  PaginatedFilesResponse,
  StatsResponse,
  TenantDescriptor,
  FoldersResponse,
} from "../shared/api/types";
export { DriveApi };
