/**
 * Mock data and handlers for Upload Plan feature
 */

type MockArgs = Record<string, unknown>;

const mockProfiles = [
  {
    id: "balanced",
    name: "Balanced Upload",
    plan: {
      originalUpload: {
        enabled: true,
        strategy: "fast",
        providers: ["discord"],
        priorityMode: "foreground",
        providerMode: "discord"
      },
      derivatives: {
        hashes: { enabled: true, algorithms: ["blake3"] },
        zipPackage: { enabled: false }
      }
    }
  },
  {
    id: "archive",
    name: "Archive Mode",
    plan: {
      originalUpload: {
        enabled: true,
        strategy: "safe",
        providers: ["discord", "telegram"],
        priorityMode: "background",
        providerMode: "discord"
      },
      derivatives: {
        hashes: { enabled: true, algorithms: ["blake3"] },
        zipPackage: { enabled: true }
      }
    }
  }
];

const mockRules: unknown[] = [];

/**
 */
export function handleUploadPlanMock(cmd: string, args: MockArgs = {}): unknown {
  const rule = (args.rule ?? {}) as Record<string, unknown>;

  switch (cmd) {
    case 'get_upload_profiles':
      return mockProfiles;
    
    case "get_upload_profile_rules":
      return mockRules;

    case "save_upload_profile_rule":
      return {
        id: Math.random().toString(36).substring(2, 11),
        ...rule,
      };

    case "delete_upload_profile_rule":
      return { success: true };

    case 'save_upload_profile':
      console.info('[Mock] Saving profile:', args);
      return { success: true };

    case 'delete_upload_profile':
      console.info('[Mock] Deleting profile:', args);
      return { success: true };

    case 'get_connection_status':
      return {
        telegram: { authorized: true },
        discord: { authorized: true }
      };

    default:
      return null;
  }
}
