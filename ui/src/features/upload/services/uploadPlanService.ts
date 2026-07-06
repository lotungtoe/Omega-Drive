import { DriveApi } from "../../../api/index";
import type { IdLike, JsonRecord } from "../../../shared/api/types";

export const uploadPlanService = {
  getProfiles: (): Promise<unknown> => DriveApi.getUploadProfiles(),
  saveProfile: (profile: JsonRecord): Promise<unknown> => DriveApi.saveUploadProfile(profile),
  deleteProfile: (id: IdLike): Promise<unknown> => DriveApi.deleteUploadProfile(id),
  restoreDefaults: (): Promise<unknown> => DriveApi.restoreDefaultProfiles(),
  getRules: (profileId: IdLike | null = null): Promise<unknown> => DriveApi.getUploadProfileRules(profileId),
  saveRule: (rule: JsonRecord): Promise<unknown> => DriveApi.saveUploadProfileRule(rule),
  deleteRule: (id: IdLike): Promise<unknown> => DriveApi.deleteUploadProfileRule(id),
  saveRulesBulk: (profileId: IdLike, orderedRuleIds: IdLike[]): Promise<unknown> =>
    DriveApi.saveUploadProfileRulesBulk(profileId, orderedRuleIds),
  resolveBatch: (items: JsonRecord[]): Promise<unknown> =>
    DriveApi.resolveUploadProfileForBatch(items),
};
