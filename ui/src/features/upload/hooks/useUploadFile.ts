import { useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  buildUploadEntries,
  selectFiles,
  confirmOverwrite,
  processPurge,
  executeUploadLoop,
  showUploadSummary,
  resumeUploadByPath,
} from '../services/uploadService';
import { uploadPlanService } from "../services/uploadPlanService";
import { toUserMessage } from '../../../shared/services/errors/toUserMessage';
import { getDriveScopeForSection } from '../../drive/hooks/driveSections';

export function useUploadFile(
  currentFolderId,
  activeSection,
  files,
  trash,
  refresh,
  toast,
  openUploadPlanModal
) {
  const lastUploadRef = useRef({ time: 0, paths: "" });


  const uploadPaths = useCallback(async (manualPaths) => {
    let paths = Array.isArray(manualPaths) ? manualPaths : null;
    
    // Debounce: prevent duplicate calls within 500ms for the same files
    const now = Date.now();
    const pathsKey = paths ? paths.join('|') : "";
    if (paths && now - lastUploadRef.current.time < 500 && pathsKey === lastUploadRef.current.paths) {
      console.info("Ignoring duplicate uploadPaths call (debounced)");
      return;
    }
    lastUploadRef.current = { time: now, paths: pathsKey };

    try {
      if (!paths || paths.length === 0) {
        paths = await selectFiles();
      }
      if (!paths || paths.length === 0) return;

      const entries = buildUploadEntries(paths, files, trash, currentFolderId);
      const collisions = entries.filter((e) => Boolean(e.collidingFile));
      toast.show(`Preparing ${entries.length} file(s) for upload...`, 'info');

      let overwriteConfirmed = true;
      if (collisions.length > 0) {
        overwriteConfirmed = await confirmOverwrite(collisions);
      }

      let blockedByPurge = new Set<string>();
      if (overwriteConfirmed && collisions.length > 0) {
        blockedByPurge = await processPurge(collisions as any);
      }

      let planByPath = new Map();
      if (openUploadPlanModal) {
        const planResult = await openUploadPlanModal(entries);
        if (!planResult) return;
        planByPath = planResult.planByPath || new Map();
      }

      const result = await executeUploadLoop(
        entries,
        overwriteConfirmed,
        blockedByPurge,
        currentFolderId,
        getDriveScopeForSection(activeSection) || "my",
        planByPath
      );

      await refresh();
      showUploadSummary(toast, result);
    } catch (err) {
      const msg = toUserMessage(err);
      console.error('Upload failed:', err);
      toast.show(msg.message, 'error');
    }
  }, [activeSection, currentFolderId, files, trash, refresh, toast, openUploadPlanModal]);

  const importUrl = useCallback(async (url, metadata, cookies_browser) => {
    try {
      const entry = {
        path: url,
        filename: metadata.title || 'Untitled Media',
        size: metadata.filesize_approx || 0,
        isExternal: true,
        metadata
      };

      let profileId = null;
      let uploadPlan = null;

      if (openUploadPlanModal) {
        const planResult = await openUploadPlanModal([entry]);
        if (planResult) {
          const plan = planResult.planByPath.get(url);
          profileId = plan?.profileId;
          uploadPlan = plan?.uploadPlan;
        }
      }

      const sessionId = Math.random().toString(36).substring(2, 15);

      await invoke('start_url_import', {
        url,
        cookiesBrowser: cookies_browser,
        folderId: currentFolderId,
        driveScope: getDriveScopeForSection(activeSection) || "my",
        sessionId,
        profileId,
        uploadPlan
      });

      toast.show(`Started importing "${entry.filename}"...`, 'success');
      await refresh();
    } catch (err) {
      const msg = toUserMessage(err);
      console.error('URL Import failed:', err);
      toast.show(msg.message, 'error');
    }
  }, [activeSection, currentFolderId, refresh, toast, openUploadPlanModal]);

  const resumeUpload = useCallback(async (fileObj) => {
    if (!fileObj.local_path) {
      toast.show("Could not find local file path to reload.", "error");
      return;
    }
    try {
      toast.show(`Continuing processing "${fileObj.filename}"...`, 'info');
      await resumeUploadByPath(fileObj);
      await refresh();
    } catch (err) {
      const msg = toUserMessage(err);
      console.error('Resume upload failed:', err);
      toast.show(msg.message, 'error');
    }
  }, [refresh, toast]);

  return { uploadPaths, resumeUpload, importUrl };
}
