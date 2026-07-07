import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { ChevronRight, ChevronDown, Package, Settings2 } from "lucide-react";
import { uploadPlanService } from "../services/uploadPlanService";
import { getConnectionStatus } from "../../diagnostics/services/diagnosticsService";
import { Button } from "../../../components/ui/be-ui-button";

// Sub-components
import { ProfileSidebar } from "./modals/upload-plan/ProfileSidebar";
import { StrategySelector } from "./modals/upload-plan/StrategySelector";
import { ProviderSelector } from "./modals/upload-plan/ProviderSelector";

const getFilename = (p) => p.split(/[\\/]/).pop();

export function UploadPlanModal({ entries, onClose, onProceed, toast }) {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [telegramAuthorized, setTelegramAuthorized] = useState(false);
  const [profiles, setProfiles] = useState([]);
  const [items, setItems] = useState([]);
  const [activeProfileId, setActiveProfileId] = useState(null);
  const [profileDrafts, setProfileDrafts] = useState({});
  const [isCreatingProfile, setIsCreatingProfile] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);

  // Initialize data
  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        const [profilesResp, resolved, statusResp] = await Promise.all([
          uploadPlanService.getProfiles(),
          uploadPlanService.resolveBatch(entries.map((e) => ({ path: e.path }))),
          getConnectionStatus().catch(() => null),
        ]);
        if (!mounted) return;
        setProfiles((profilesResp as any) || []);
        setTelegramAuthorized(Boolean((statusResp as any)?.telegram?.authorized));

        const nameMap = new Map(entries.map((e) => [e.path, e.filename || getFilename(e.path)]));
        const resolvedItems = ((resolved as any) || []).map((r: any) => ({
          path: r.path,
          filename: nameMap.get(r.path) || getFilename(r.path),
          profileId: r.profileId ?? null,
          fileType: r.fileType,
          extension: r.extension,
        }));
        setItems(resolvedItems);

        const defaultId =
          resolvedItems[0]?.profileId ||
          profilesResp?.[0]?.id ||
          null;
        setActiveProfileId(defaultId);
      } catch (err) {
        console.error("Failed to load profiles:", err);
        toast?.show?.(t("upload.loadProfilesFailed"), "error");
      } finally {
        if (mounted) setLoading(false);
      }
    })();
    return () => {
      mounted = false;
    };
  }, [entries, toast, t]);

  const profilesById = useMemo(() => {
    const map = new Map();
    for (const p of profiles) {
      if (p?.id != null) map.set(p.id, p);
    }
    return map;
  }, [profiles]);

  const ensureProfileDraft = useMemo(() => (profileId) => {
    if (!profileId) return;
    setProfileDrafts((prev) => {
      if (prev[profileId]) return prev;
      const profile = profilesById.get(profileId);
      if (!profile) return prev;
      return { 
        ...prev, 
        [profileId]: { 
          name: profile.name, 
          plan: structuredClone(profile.plan) 
        } 
      };
    });
  }, [profilesById]);

  useEffect(() => {
    if (activeProfileId) ensureProfileDraft(activeProfileId);
  }, [activeProfileId, ensureProfileDraft]);

  const getProfileDraft = (profileId) => {
    if (!profileId) return null;
    return profileDrafts[profileId] || profilesById.get(profileId);
  };

  const handleCreateProfile = async () => {
    if (isCreatingProfile) return;
    setIsCreatingProfile(true);
    try {
      const newProfile = {
        name: t("upload.newProfile", "New Profile"),
        plan: profiles[0]?.plan || { 
          originalUpload: { 
            enabled: true, 
            strategy: "fast", 
            providers: ["discord", "telegram"], 
            priorityMode: "foreground" 
          },
          derivatives: { 
            hashes: { enabled: true, algorithms: ["blake3"] },
            zipPackage: { enabled: false, zipLevel: 6 }
          },
          advanced: {
            hardLimitMb: 12,
            fileLimitMb: 200,
            maxTotalUploadMb: 512,
            discordBatchSize: 10
          }
        },
      };
      const saved = await uploadPlanService.saveProfile(newProfile);
      setProfiles((prev) => [...prev, saved]);
      setActiveProfileId((saved as any).id);
      toast?.show?.(t("upload.profileCreated"), "success");
    } catch (err) {
      console.error("Failed to create profile:", err);
      toast?.show?.(t("upload.createProfileFailed"), "error");
    } finally {
      setIsCreatingProfile(false);
    }
  };

  const handleDeleteProfile = async (id) => {
    if (!id) return;
    try {
      await uploadPlanService.deleteProfile(id);
      setProfiles((prev) => prev.filter((p) => p.id !== id));
      if (activeProfileId === id) {
        setActiveProfileId(profiles.find((p) => p.id !== id)?.id || null);
      }
      toast?.show?.(t("upload.profileDeleted"), "success");
    } catch (err) {
      console.error("Failed to delete profile:", err);
      toast?.show?.(t("upload.deleteProfileFailed"), "error");
    }
  };

  const updateProfileDraft = (profileId, updater) => {
    if (!profileId) return;
    setProfileDrafts((prev) => {
      const current = prev[profileId] || { 
        name: profilesById.get(profileId)?.name, 
        plan: structuredClone(profilesById.get(profileId)?.plan) 
      };
      return { ...prev, [profileId]: updater(current) };
    });
  };

  const activeDraft = getProfileDraft(activeProfileId);
  const activePlan = activeDraft?.plan || null;
  const zipEnabled = activePlan?.derivatives?.zipPackage?.enabled ?? false;

  const getUploadPlanForFile = (item, persistChanges, updatedProfiles) => {
    const profileId = item.profileId ?? null;
    if (!profileId) return { profileId, uploadPlan: null };

    const profile = profilesById.get(profileId);
    const draft = getProfileDraft(profileId);
    
    if (draft && profile) {
      const isPlanChanged = JSON.stringify(profile.plan) !== JSON.stringify(draft.plan);
      const isNameChanged = draft.name !== profile.name;
      
      if (isPlanChanged || isNameChanged) {
        if (persistChanges) {
          updatedProfiles.push({ ...profile, name: draft.name, plan: draft.plan });
        }
        return { profileId, uploadPlan: draft.plan };
      }
    }
    return { profileId, uploadPlan: null };
  };

  const startUpload = async (persistChanges) => {
    if (!telegramAuthorized) {
      const usesTelegram = items.some((item) => {
        if (!item.profileId) return false;
        const plan = getProfileDraft(item.profileId)?.plan;
        const providers = plan?.originalUpload?.providers || [];
        return providers.includes("telegram");
      });

      if (usesTelegram) {
        toast?.show(t("upload.telegramNotAuthorizedSelected"), "error");
        return;
      }
    }

    const planByPath = new Map();
    const updatedProfiles = [];

    for (const item of items) {
      planByPath.set(item.path, getUploadPlanForFile(item, persistChanges, updatedProfiles));
    }

    if (persistChanges && updatedProfiles.length > 0) {
      await Promise.all(updatedProfiles.map(p => uploadPlanService.saveProfile(p)));
    }

    onProceed({ planByPath });
  };

  const handleSaveAndProceed = async () => {
    try {
      await startUpload(true);
    } catch (err) {
      console.error("Save profiles failed:", err);
      toast?.show?.(t("upload.savePrefsFailed"), "error");
    }
  };

  if (loading) {
    return (
      <div className="fixed inset-0 z-[160] flex items-center justify-center bg-black/60">
        <div className="rounded-2xl p-6 text-sm bg-[var(--gd-surface)] text-[var(--gd-on-surface)]">
          {t("common.loading")}
        </div>
      </div>
    );
  }

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      onClick={onClose}
      className="fixed inset-0 z-[160] flex items-center justify-center bg-black/60 p-4"
    >
      <motion.div
        initial={{ scale: 0.9, opacity: 0, y: 20 }}
        animate={{ scale: 1, opacity: 1, y: 0 }}
        className="upload-plan-popup bg-[var(--gd-surface)] border border-[var(--gd-outline)] w-[920px] max-w-full max-h-[90vh] flex flex-col gap-6 overflow-hidden shadow-2xl rounded-3xl text-[var(--gd-on-surface)]"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="modal-title"
      >
        <div className="flex-1 flex flex-row gap-8 overflow-hidden p-8 pb-4">
          <div className="flex flex-row flex-1 gap-8 overflow-hidden">
            <ProfileSidebar 
              profiles={profiles}
              activeProfileId={activeProfileId}
              onSelectProfile={setActiveProfileId}
              onDeleteProfile={handleDeleteProfile}
              onCreateProfile={handleCreateProfile}
              isCreating={isCreatingProfile}
            />

            <div className="flex-1 flex flex-col gap-6 overflow-y-auto pr-2 scrollbar-thin">
              <div className="flex flex-col gap-1 mb-2">
                <div className="flex items-center gap-3">
                    <div className="w-2 h-10 rounded-full bg-blue-500" />
                    <div className="flex flex-col flex-1">
                    <input
                      type="text"
                      value={activeDraft?.name || ""}
                      onChange={(e) => updateProfileDraft(activeProfileId, cur => ({ ...cur, name: e.target.value }))}
                      className="text-3xl font-bold tracking-tight bg-transparent border-none outline-none focus:ring-2 focus:ring-blue-500/20 rounded px-1 -ml-1 w-full"
                      placeholder={t("upload.profileNamePlaceholder", "Profile Name")}
                    />
                    </div>
                </div>
                <p className="text-[10px] opacity-40 font-bold uppercase tracking-wider pl-5">
                  {t("upload.customEditable", "Customizable User Profile")}
                </p>
              </div>

              {activeDraft && (
                <div className="flex flex-col gap-5">
                  <StrategySelector 
                    strategy={activePlan?.originalUpload?.strategy}
                    onChange={(val) => updateProfileDraft(activeProfileId, cur => {
                      let nextProviders = cur.plan.originalUpload?.providers || ["discord"];
                      if (val === "fast" || val === "safe") {
                        nextProviders = ["discord", "telegram"];
                      } else if (val === "none") {
                        // Keep current or default to discord if multiple
                        if (nextProviders.length > 1) {
                          nextProviders = ["discord"];
                        }
                      }
                      
                      return {
                        ...cur,
                        plan: { 
                          ...cur.plan, 
                          originalUpload: { 
                            ...cur.plan.originalUpload, 
                            strategy: val,
                            providers: nextProviders
                          } 
                        }
                      };
                    })}
                  />

                  {activePlan?.originalUpload?.strategy === "none" && (
                    <ProviderSelector 
                      providerMode={activePlan?.originalUpload?.providers?.[0] || "discord"}
                      onChange={(val) => updateProfileDraft(activeProfileId, cur => ({
                        ...cur,
                        plan: { 
                          ...cur.plan, 
                          originalUpload: { 
                            ...cur.plan.originalUpload, 
                            providers: [val]
                          } 
                        }
                      }))}
                      telegramAuthorized={telegramAuthorized}
                    />
                  )}

                  <div className="config-group">
                    <span className="config-group-title">
                      {t("upload.derivatives", "Derivatives")}
                    </span>
                    <Button
                      variant={zipEnabled ? "primary" : "ghost"}
                      size="md"
                      onClick={() => updateProfileDraft(activeProfileId, (cur) => ({
                        ...cur,
                        plan: {
                          ...cur.plan,
                          derivatives: {
                            ...cur.plan.derivatives,
                            zipPackage: {
                              ...cur.plan.derivatives?.zipPackage,
                              enabled: !zipEnabled,
                            },
                          },
                        },
                      }))}
                      aria-pressed={zipEnabled}
                      className={zipEnabled ? "" : "opacity-70 dark:opacity-50"}
                    >
                      <div className="flex items-center justify-center gap-2">
                        <Package size={18} />
                        <span className="text-[10px] font-bold uppercase">
                          {t("upload.compression", "Compression")}
                        </span>
                      </div>
                    </Button>
                  </div>

                  {/* Advanced Section */}
                  <div className="mt-8 border-t border-[var(--gd-outline-variant)] pt-6">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setShowAdvanced(!showAdvanced)}
                      className="gap-2 group"
                    >
                      <div className="w-6 h-6 rounded-lg bg-[var(--gd-surface-variant)] flex items-center justify-center text-[var(--gd-on-surface-variant)] group-hover:bg-blue-500/10 group-hover:text-blue-500 transition-colors">
                        {showAdvanced ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                      </div>
                      <span className="text-[11px] font-bold uppercase tracking-wider text-[var(--gd-on-surface-variant)] group-hover:text-blue-500 transition-colors">
                        {t("upload.advanced", "Advanced Settings")}
                      </span>
                      <Settings2 size={12} className="opacity-20 group-hover:opacity-100 transition-opacity" />
                    </Button>

                    {showAdvanced && (
                      <motion.div
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: "auto", opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        className="overflow-hidden"
                      >
                        <div className="grid grid-cols-2 gap-6 mt-6 pb-2">
                           {/* Zip Level */}
                           <div className="flex flex-col gap-2">
                              <label className="text-[10px] font-bold uppercase opacity-50 tracking-wide">{t("upload.zipLevel", "Zip Compression Level")}</label>
                              <input 
                                type="number"
                                min="0"
                                max="22"
                                value={activePlan?.derivatives?.zipPackage?.zipLevel ?? 6}
                                onChange={(e) => updateProfileDraft(activeProfileId, (cur) => ({
                                  ...cur,
                                  plan: {
                                    ...cur.plan,
                                    derivatives: {
                                      ...cur.plan.derivatives,
                                      zipPackage: { ...cur.plan.derivatives?.zipPackage, zipLevel: Number.parseInt(e.target.value, 10) || 0 }
                                    }
                                  }
                                }))}
                                className="bg-[var(--gd-surface-variant)] border border-[var(--gd-outline-variant)] rounded-xl px-4 py-3 text-xs focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                              />
                           </div>

                           {/* Hard Limit */}
                           <div className="flex flex-col gap-2">
                              <label className="text-[10px] font-bold uppercase opacity-50 tracking-wide">{t("upload.hardLimit", "Hard Limit (MB)")}</label>
                              <input 
                                type="number"
                                value={activePlan?.advanced?.hardLimitMb ?? ""}
                                placeholder="e.g. 15"
                                onChange={(e) => updateProfileDraft(activeProfileId, (cur) => ({
                                  ...cur,
                                  plan: {
                                    ...cur.plan,
                                    advanced: { ...cur.plan.advanced, hardLimitMb: Number.parseInt(e.target.value, 10) || undefined }
                                  }
                                }))}
                                className="bg-[var(--gd-surface-variant)] border border-[var(--gd-outline-variant)] rounded-xl px-4 py-3 text-xs focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                              />
                           </div>

                           {/* File Limit MB */}
                           <div className="flex flex-col gap-2">
                              <label className="text-[10px] font-bold uppercase opacity-50 tracking-wide">{t("upload.fileLimit", "File Limit (MB)")}</label>
                              <input 
                                type="number"
                                value={activePlan?.advanced?.fileLimitMb ?? ""}
                                placeholder="e.g. 2048"
                                onChange={(e) => updateProfileDraft(activeProfileId, (cur) => ({
                                  ...cur,
                                  plan: {
                                    ...cur.plan,
                                    advanced: { ...cur.plan.advanced, fileLimitMb: Number.parseInt(e.target.value, 10) || undefined }
                                  }
                                }))}
                                className="bg-[var(--gd-surface-variant)] border border-[var(--gd-outline-variant)] rounded-xl px-4 py-3 text-xs focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                              />
                           </div>

                           {/* Parallel Threads */}
                           <div className="flex flex-col gap-2">
                              <label className="text-[10px] font-bold uppercase opacity-50 tracking-wide">{t("upload.concurrency", "Parallel Threads")}</label>
                              <input 
                                type="number"
                                min="1"
                                max="10"
                                value={activePlan?.advanced?.concurrencyThreads ?? ""}
                                placeholder="Default"
                                onChange={(e) => updateProfileDraft(activeProfileId, (cur) => ({
                                  ...cur,
                                  plan: {
                                    ...cur.plan,
                                    advanced: { ...cur.plan.advanced, concurrencyThreads: Number.parseInt(e.target.value, 10) || undefined }
                                  }
                                }))}
                                className="bg-[var(--gd-surface-variant)] border border-[var(--gd-outline-variant)] rounded-xl px-4 py-3 text-xs focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                              />
                           </div>

                           {/* Discord Message Batch Size */}
                           <div className="flex flex-col gap-2">
                              <label className="text-[10px] font-bold uppercase text-indigo-400 tracking-wide">{t("upload.discordBatch", "Discord Message Batch")}</label>
                              <input 
                                type="number"
                                min="1"
                                max="10"
                                value={activePlan?.advanced?.discordBatchSize ?? ""}
                                placeholder="Default (10)"
                                onChange={(e) => updateProfileDraft(activeProfileId, (cur) => ({
                                  ...cur,
                                  plan: {
                                    ...cur.plan,
                                    advanced: { ...cur.plan.advanced, discordBatchSize: Number.parseInt(e.target.value, 10) || undefined }
                                  }
                                }))}
                                className="bg-[var(--gd-surface-variant)] border border-[var(--gd-outline-variant)] rounded-xl px-4 py-3 text-xs focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                              />
                           </div>

                           {/* Max Retries */}
                           <div className="flex flex-col gap-2">
                              <label className="text-[10px] font-bold uppercase opacity-50 tracking-wide">{t("upload.retries", "Max Retries")}</label>
                              <input 
                                type="number"
                                min="0"
                                max="10"
                                value={activePlan?.advanced?.retryCount ?? ""}
                                placeholder="0"
                                onChange={(e) => updateProfileDraft(activeProfileId, (cur) => ({
                                  ...cur,
                                  plan: {
                                    ...cur.plan,
                                    advanced: { ...cur.plan.advanced, retryCount: Number.parseInt(e.target.value, 10) || undefined }
                                  }
                                }))}
                                className="bg-[var(--gd-surface-variant)] border border-[var(--gd-outline-variant)] rounded-xl px-4 py-3 text-xs focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                              />
                           </div>

                           {/* Chunk Size MB */}
                           <div className="flex flex-col gap-2">
                              <label className="text-[10px] font-bold uppercase opacity-50 tracking-wide">{t("upload.chunkSize", "Override Chunk Size (MB)")}</label>
                              <input 
                                type="number"
                                min="1"
                                value={activePlan?.advanced?.chunkSizeMb ?? ""}
                                placeholder="Auto"
                                onChange={(e) => updateProfileDraft(activeProfileId, (cur) => ({
                                  ...cur,
                                  plan: {
                                    ...cur.plan,
                                    advanced: { ...cur.plan.advanced, chunkSizeMb: Number.parseInt(e.target.value, 10) || undefined }
                                  }
                                }))}
                                className="bg-[var(--gd-surface-variant)] border border-[var(--gd-outline-variant)] rounded-xl px-4 py-3 text-xs focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                              />
                           </div>

                           {/* Bandwidth Limit */}
                           <div className="flex flex-col gap-2">
                              <label className="text-[10px] font-bold uppercase opacity-50 tracking-wide">{t("upload.bandwidth", "Speed Limit (KB/s)")}</label>
                              <input 
                                type="number"
                                min="0"
                                value={activePlan?.advanced?.bandwidthLimitKbps ?? ""}
                                placeholder="Unlimited"
                                onChange={(e) => updateProfileDraft(activeProfileId, (cur) => ({
                                  ...cur,
                                  plan: {
                                    ...cur.plan,
                                    advanced: { ...cur.plan.advanced, bandwidthLimitKbps: Number.parseInt(e.target.value, 10) || undefined }
                                  }
                                }))}
                                className="bg-[var(--gd-surface-variant)] border border-[var(--gd-outline-variant)] rounded-xl px-4 py-3 text-xs focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                              />
                           </div>

                           {/* Webhook URL */}
                           <div className="flex flex-col gap-2 col-span-2">
                              <label className="text-[10px] font-bold uppercase opacity-50 tracking-wide">{t("upload.webhook", "Discord Webhook Notification URL")}</label>
                              <input 
                                type="text"
                                value={activePlan?.advanced?.webhookUrl ?? ""}
                                placeholder="https://discord.com/api/webhooks/..."
                                onChange={(e) => updateProfileDraft(activeProfileId, (cur) => ({
                                  ...cur,
                                  plan: {
                                    ...cur.plan,
                                    advanced: { ...cur.plan.advanced, webhookUrl: e.target.value || undefined }
                                  }
                                }))}
                                className="bg-[var(--gd-surface-variant)] border border-[var(--gd-outline-variant)] rounded-xl px-4 py-3 text-xs focus:ring-2 focus:ring-blue-500/50 outline-none transition-all font-mono"
                              />
                           </div>

                           {/* File Limit */}
                           <div className="flex flex-col gap-2">
                              <label className="text-[10px] font-bold uppercase opacity-50 tracking-wide">{t("upload.fileLimit", "File Limit (MB)")}</label>
                              <input 
                                type="number"
                                value={activePlan?.advanced?.fileLimitMb ?? ""}
                                placeholder="e.g. 2000"
                                onChange={(e) => updateProfileDraft(activeProfileId, (cur) => ({
                                  ...cur,
                                  plan: {
                                    ...cur.plan,
                                    advanced: { ...cur.plan.advanced, fileLimitMb: Number.parseInt(e.target.value, 10) || 0 }
                                  }
                                }))}
                                className="bg-[var(--gd-surface-variant)] border border-[var(--gd-outline-variant)] rounded-xl px-4 py-3 text-xs focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                              />
                           </div>

                           {/* Max RAM */}
                           <div className="flex flex-col gap-2">
                              <label className="text-[10px] font-bold uppercase opacity-50 tracking-wide">{t("upload.maxRam", "Max RAM Usage (MB)")}</label>
                              <input 
                                type="number"
                                value={activePlan?.advanced?.maxTotalUploadMb ?? ""}
                                placeholder="e.g. 512"
                                onChange={(e) => updateProfileDraft(activeProfileId, (cur) => ({
                                  ...cur,
                                  plan: {
                                    ...cur.plan,
                                    advanced: { ...cur.plan.advanced, maxTotalUploadMb: Number.parseInt(e.target.value, 10) || 0 }
                                  }
                                }))}
                                className="bg-[var(--gd-surface-variant)] border border-[var(--gd-outline-variant)] rounded-xl px-4 py-3 text-xs focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
                              />
                           </div>
                        </div>
                      </motion.div>
                    )}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>

        <div className="px-8 py-6 bg-[var(--gd-surface-variant)] border-t border-[var(--gd-outline-variant)] flex items-center justify-between">
           <div className="flex flex-col gap-1">
              <span className="text-[10px] font-bold uppercase tracking-wider opacity-40">
                {t("upload.executionSummary", "Execution Summary")}
              </span>
              <div className="flex items-center gap-4">
                 <div className="flex items-center gap-2">
                    <span className="text-2xl font-bold">{items.length}</span>
                    <span className="text-[10px] font-bold opacity-50 uppercase tracking-tighter">{t("upload.files", "Files")}</span>
                 </div>
              </div>
           </div>

           <div className="flex gap-3">
            <Button
              variant="primary"
              size="lg"
              onClick={handleSaveAndProceed}
              className="shadow-lg shadow-blue-500/20"
            >
              {t("upload.proceed", "Start Upload")}
            </Button>
           </div>
        </div>
      </motion.div>
    </motion.div>
  );
}

