const fs = require('fs');
const path = require('path');

const SRC = path.resolve(__dirname, '../src');

// 1. Define old -> new path mappings for all files
// Key: old relative path (from src/), Value: new relative path (from src/)
const MOVE_MAP = {
  // === SHARED: api core ===
  'api/call.ts': 'shared/api/call.ts',
  'api/types.ts': 'shared/api/types.ts',
  'api/mocks.ts': 'shared/api/mocks.ts',

  // === SHARED: components ===
  'components/Common.tsx': 'shared/components/Common.tsx',
  'components/Icons.tsx': 'shared/components/Icons.tsx',
  'components/Toasts.tsx': 'shared/components/Toasts.tsx',
  'components/ErrorBoundary.tsx': 'shared/components/ErrorBoundary.tsx',
  'components/OverlayLoader.tsx': 'shared/components/OverlayLoader.tsx',
  'components/ProgressOverlay.tsx': 'shared/components/ProgressOverlay.tsx',

  // === SHARED: ui/atoms (all) ===
  'ui/atoms/Logo.tsx': 'shared/ui/atoms/Logo.tsx',
  'ui/atoms/BtnNewFolder.tsx': 'shared/ui/atoms/BtnNewFolder.tsx',
  'ui/atoms/BtnRefresh.tsx': 'shared/ui/atoms/BtnRefresh.tsx',
  'ui/atoms/BtnSettings.tsx': 'shared/ui/atoms/BtnSettings.tsx',
  'ui/atoms/BtnSync.tsx': 'shared/ui/atoms/BtnSync.tsx',
  'ui/atoms/BtnThemeToggle.tsx': 'shared/ui/atoms/BtnThemeToggle.tsx',
  'ui/atoms/BtnUpload.tsx': 'shared/ui/atoms/BtnUpload.tsx',
  'ui/atoms/BtnViewToggle.tsx': 'shared/ui/atoms/BtnViewToggle.tsx',
  'ui/atoms/SortButton.tsx': 'shared/ui/atoms/SortButton.tsx',
  'ui/atoms/TxtSearch.tsx': 'shared/ui/atoms/TxtSearch.tsx',
  'ui/atoms/BreadcrumbItem.tsx': 'shared/ui/atoms/BreadcrumbItem.tsx',

  // === SHARED: utils ===
  'utils/index.ts': 'shared/utils/index.ts',
  'utils/formatPlaybackTime.ts': 'shared/utils/formatPlaybackTime.ts',
  'utils/windowMode.ts': 'shared/utils/windowMode.ts',

  // === SHARED: hooks ===
  'hooks/useWindowStateTracker.ts': 'shared/hooks/useWindowStateTracker.ts',

  // === SHARED: services ===
  'services/featureLog.ts': 'shared/services/featureLog.ts',
  'services/errors/normalizeError.ts': 'shared/services/errors/normalizeError.ts',
  'services/errors/reportError.ts': 'shared/services/errors/reportError.ts',
  'services/errors/toUserMessage.ts': 'shared/services/errors/toUserMessage.ts',
  'services/errors/types.ts': 'shared/services/errors/types.ts',

  // === FEATURE: drive ===
  'api/files.ts': 'features/drive/api/files.ts',
  'api/folders.ts': 'features/drive/api/folders.ts',
  'components/drive/FileGrid.tsx': 'features/drive/components/FileGrid.tsx',
  'components/drive/Header.tsx': 'features/drive/components/Header.tsx',
  'components/drive/Sidebar.tsx': 'features/drive/components/Sidebar.tsx',
  'components/drive/TenantScopeDropdown.tsx': 'features/drive/components/TenantScopeDropdown.tsx',
  'components/drive/Toolbar.tsx': 'features/drive/components/Toolbar.tsx',
  'components/drive/FileCard/dragPreview.ts': 'features/drive/components/FileCard/dragPreview.ts',
  'components/drive/FileCard/FileCard.tsx': 'features/drive/components/FileCard/FileCard.tsx',
  'components/drive/FileCard/FileCardMenu.tsx': 'features/drive/components/FileCard/FileCardMenu.tsx',
  'components/drive/FileCard/useFileCardLogic.ts': 'features/drive/components/FileCard/useFileCardLogic.ts',
  'components/drive/Toolbar/ListHeader.tsx': 'features/drive/components/Toolbar/ListHeader.tsx',
  'components/drive/Toolbar/SortBar.tsx': 'features/drive/components/Toolbar/SortBar.tsx',
  'components/modals/NewFolderModal.tsx': 'features/drive/components/NewFolderModal.tsx',
  'components/modals/DeleteConfirmModal.tsx': 'features/drive/components/DeleteConfirmModal.tsx',
  'components/modals/SharedDriveSetupModal.tsx': 'features/drive/components/SharedDriveSetupModal.tsx',
  'components/modals/UrlImportModal.tsx': 'features/drive/components/UrlImportModal.tsx',
  'components/modals/TenantManagerModal.tsx': 'features/drive/components/TenantManagerModal.tsx',
  'components/modals/ProviderOnboardingModal.tsx': 'features/drive/components/ProviderOnboardingModal.tsx',
  'hooks/drive/driveSections.ts': 'features/drive/hooks/driveSections.ts',
  'hooks/drive/useBreadcrumbs.ts': 'features/drive/hooks/useBreadcrumbs.ts',
  'hooks/drive/useCreateFolder.ts': 'features/drive/hooks/useCreateFolder.ts',
  'hooks/drive/useDeepLink.ts': 'features/drive/hooks/useDeepLink.ts',
  'hooks/drive/useDeleteFile.ts': 'features/drive/hooks/useDeleteFile.ts',
  'hooks/drive/useDrive.tsx': 'features/drive/hooks/useDrive.tsx',
  'hooks/drive/useDriveController.ts': 'features/drive/hooks/useDriveController.ts',
  'hooks/drive/useDriveEventSubscriptions.ts': 'features/drive/hooks/useDriveEventSubscriptions.ts',
  'hooks/drive/useDriveMutations.ts': 'features/drive/hooks/useDriveMutations.ts',
  'hooks/drive/useDriveQuery.ts': 'features/drive/hooks/useDriveQuery.ts',
  'hooks/drive/useFileFiltering.ts': 'features/drive/hooks/useFileFiltering.ts',
  'hooks/drive/useFileSorting.ts': 'features/drive/hooks/useFileSorting.ts',
  'hooks/drive/useKeyboardActions.ts': 'features/drive/hooks/useKeyboardActions.ts',
  'hooks/drive/useMainAppUiState.ts': 'features/drive/hooks/useMainAppUiState.ts',
  'hooks/drive/usePageNavigation.ts': 'features/drive/hooks/usePageNavigation.ts',
  'services/drive/driveService.ts': 'features/drive/services/driveService.ts',
  'services/utils/drive.ts': 'features/drive/services/driveUtils.ts',
  'pages/DrivePage.tsx': 'features/drive/pages/DrivePage.tsx',
  'pages/main-app/MainAppContent.tsx': 'features/drive/pages/MainAppContent.tsx',
  'pages/main-app/MainAppProvider.tsx': 'features/drive/pages/MainAppProvider.tsx',
  'pages/main-app/useMainAppContext.ts': 'features/drive/pages/useMainAppContext.ts',

  // === FEATURE: player ===
  'api/playback.ts': 'features/player/api.ts',
  'api/mpv.ts': 'features/player/api/mpv.ts',
  'components/player/GlobalAudioBridge.tsx': 'features/player/components/GlobalAudioBridge.tsx',
  'components/player/MiniAudioPlayer.tsx': 'features/player/components/MiniAudioPlayer.tsx',
  'components/player/NativePlayerOverlay.tsx': 'features/player/components/NativePlayerOverlay.tsx',
  'components/player/NativePlayerOverlay.css': 'features/player/components/NativePlayerOverlay.css',
  'hooks/drive/usePlaybackLauncher.ts': 'features/player/hooks/usePlaybackLauncher.ts',
  'hooks/player/useHlsPlayer.ts': 'features/player/hooks/useHlsPlayer.ts',
  'hooks/player/usePlaybackState.ts': 'features/player/hooks/usePlaybackState.ts',
  'hooks/player/usePlaybackSync.ts': 'features/player/hooks/usePlaybackSync.ts',
  'hooks/player/usePlayerConfig.ts': 'features/player/hooks/usePlayerConfig.ts',
  'hooks/player/usePlayerDiagnostics.ts': 'features/player/hooks/usePlayerDiagnostics.ts',
  'hooks/player/useSubtitleUpload.ts': 'features/player/hooks/useSubtitleUpload.ts',
  'services/player/playerService.ts': 'features/player/services/playerService.ts',
  'pages/StandaloneVideoWindow.tsx': 'features/player/pages/StandaloneVideoWindow.tsx',

  // === FEATURE: upload ===
  'api/upload.ts': 'features/upload/api.ts',
  'hooks/drive/useUploadFile.ts': 'features/upload/hooks/useUploadFile.ts',
  'services/upload/uploadService.ts': 'features/upload/services/uploadService.ts',
  'services/upload/uploadPlanService.ts': 'features/upload/services/uploadPlanService.ts',
  'components/modals/UploadPlanModal.tsx': 'features/upload/components/UploadPlanModal.tsx',
  'components/modals/upload-plan/Icons.tsx': 'features/upload/components/modals/upload-plan/Icons.tsx',
  'components/modals/upload-plan/ProfileSidebar.tsx': 'features/upload/components/modals/upload-plan/ProfileSidebar.tsx',
  'components/modals/upload-plan/ProviderSelector.tsx': 'features/upload/components/modals/upload-plan/ProviderSelector.tsx',
  'components/modals/upload-plan/RuleEditor.tsx': 'features/upload/components/modals/upload-plan/RuleEditor.tsx',
  'components/modals/upload-plan/StrategySelector.tsx': 'features/upload/components/modals/upload-plan/StrategySelector.tsx',
  'debug/UploadPlanSandbox.tsx': 'features/upload/components/UploadPlanSandbox.tsx',
  'debug/uploadPlanMocks.ts': 'features/upload/components/uploadPlanMocks.ts',

  // === FEATURE: download/transfers ===
  'hooks/drive/useDownloadFile.ts': 'features/download/hooks/useDownloadFile.ts',
  'hooks/drive/useTransfersList.ts': 'features/download/hooks/useTransfersList.ts',
  'hooks/drive/useStatusMonitor.ts': 'features/download/hooks/useStatusMonitor.ts',
  'hooks/downloads/useDownloads.ts': 'features/download/hooks/useDownloads.ts',
  'services/download/downloadService.ts': 'features/download/services/downloadService.ts',
  'pages/TransfersPage.tsx': 'features/download/pages/TransfersPage.tsx',

  // === FEATURE: settings ===
  'api/settings.ts': 'features/settings/api.ts',
  'services/settings/settingsService.ts': 'features/settings/services/settingsService.ts',
  'components/modals/SettingsModal.tsx': 'features/settings/components/SettingsModal.tsx',

  // === FEATURE: preview ===
  'components/modals/PreviewModal.tsx': 'features/preview/components/PreviewModal.tsx',
  'components/modals/preview/AudioPreview.tsx': 'features/preview/components/AudioPreview.tsx',
  'components/modals/preview/DocxPreview.tsx': 'features/preview/components/DocxPreview.tsx',
  'components/modals/preview/FileDetailsPreview.tsx': 'features/preview/components/FileDetailsPreview.tsx',
  'components/modals/preview/ImagePreview.tsx': 'features/preview/components/ImagePreview.tsx',
  'components/modals/preview/PdfPreview.tsx': 'features/preview/components/PdfPreview.tsx',
  'components/modals/preview/SheetPreview.tsx': 'features/preview/components/SheetPreview.tsx',
  'components/modals/preview/TextPreview.tsx': 'features/preview/components/TextPreview.tsx',

  // === FEATURE: diagnostics ===
  'services/diagnostics/diagnosticsService.ts': 'features/diagnostics/services/diagnosticsService.ts',

  // === FEATURE: extensions ===
  'api/extensions.ts': 'features/extensions/api.ts',
};

// Build reverse map for import rewriting
const oldToNew = {};
const newToOld = {};
for (const [oldPath, newPath] of Object.entries(MOVE_MAP)) {
  oldToNew[oldPath] = newPath;
  const oldNoExt = oldPath.replace(/\.(ts|tsx|css)$/, '');
  const newNoExt = newPath.replace(/\.(ts|tsx|css)$/, '');
  oldToNew[oldNoExt] = newNoExt;
  newToOld[newPath] = oldPath;
}

// 2. Helper: resolve path mappings for imports
function resolveImport(fromFile, importPath) {
  if (!importPath.startsWith('.')) return null; // not a relative import
  const fromDir = path.posix.dirname(fromFile).replace(/\\/g, '/');
  const absolute = path.posix.resolve('/', fromDir, importPath);
  // Try with and without extensions
  const candidates = [
    absolute,
    absolute + '.ts',
    absolute + '.tsx',
    absolute + '.css',
    absolute + '/index.ts',
    absolute + '/index.tsx',
  ];
  for (const c of candidates) {
    const relative = c.startsWith('/') ? c.slice(1) : c;
    if (oldToNew[relative]) return oldToNew[relative];
    // Check if file was moved
    if (oldToNew[relative + '/index']) return oldToNew[relative + '/index'];
  }
  return null;
}

// 3. Collect all files
function getAllFiles(dir, baseDir = '') {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    const relative = baseDir ? `${baseDir}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      // Skip node_modules and .git
      if (entry.name === 'node_modules' || entry.name === '.git' || entry.name === '.claude') continue;
      if (entry.name === 'features' || entry.name === 'shared') continue; // skip new dirs
      files.push(...getAllFiles(fullPath, relative));
    } else if (/\.(ts|tsx|css)$/.test(entry.name)) {
      files.push({ fullPath, relative });
    }
  }
  return files;
}

// 4. Process: move files and update imports
function main() {
  const allFiles = getAllFiles(SRC);
  const renamed = new Set();

  // First pass: move files to new locations
  for (const file of allFiles) {
    const newRelative = oldToNew[file.relative];
    if (!newRelative) continue;
    const targetPath = path.join(SRC, newRelative);
    fs.mkdirSync(path.dirname(targetPath), { recursive: true });
    try {
      fs.renameSync(file.fullPath, targetPath);
      renamed.add(file.relative);
      console.log(`MOVED: ${file.relative} -> ${newRelative}`);
    } catch (e) {
      console.error(`FAILED: ${file.relative} -> ${newRelative}`, e.message);
    }
  }

  // Handle api/index.ts separately: split into per-feature + shared barrel
  const apiIndexPath = path.join(SRC, 'api/index.ts');
  if (fs.existsSync(apiIndexPath)) {
    // Move it to shared/api/index.ts first (we'll keep it as shared fallback)
    const sharedDir = path.join(SRC, 'shared/api');
    fs.mkdirSync(sharedDir, { recursive: true });
    fs.renameSync(apiIndexPath, path.join(sharedDir, 'index.old.ts'));
    console.log('MOVED: api/index.ts -> shared/api/index.old.ts (needs manual split)');
  }

  // Second pass: update imports in all remaining files (old and new locations)
  // Some files may have been moved to features/ or shared/
  const remainingFiles = getAllFiles(SRC, '').filter(
    f => !renamed.has(f.relative) && !f.relative.includes('node_modules') && !f.relative.includes('/index.old.ts')
  );

  // Also scan the new feature and shared dirs
  const newDirs = ['features', 'shared'];
  for (const dir of newDirs) {
    const dirPath = path.join(SRC, dir);
    if (fs.existsSync(dirPath)) {
      remainingFiles.push(...getAllFiles(dirPath, dir));
    }
  }

  const importRegex = /from\s+['"](\.[^'"]+)['"]/g;

  for (const file of remainingFiles) {
    const content = fs.readFileSync(file.fullPath, 'utf-8');
    let updated = content;
    let match;
    let changed = false;

    // Reset regex
    importRegex.lastIndex = 0;

    const newContent = content.replace(importRegex, (match, importPath) => {
      const resolved = resolveImport(file.relative, importPath);
      if (resolved) {
        // Compute new relative path from this file's new location
        const fileDir = path.posix.dirname(file.relative);
        let relativePath = path.posix.relative(fileDir, resolved).replace(/\\/g, '/');
        if (!relativePath.startsWith('.')) relativePath = './' + relativePath;
        // Remove duplicate /index
        relativePath = relativePath.replace(/\/index$/, '');
        if (relativePath !== importPath) {
          changed = true;
          return `from '${relativePath}'`;
        }
      }
      return match;
    });

    if (changed) {
      fs.writeFileSync(file.fullPath, newContent, 'utf-8');
      console.log(`UPDATED: ${file.relative}`);
    }
  }

  // Handle test/setup.ts separately
  const setupPath = path.join(SRC, 'test/setup.ts');
  if (fs.existsSync(setupPath)) {
    const setupContent = fs.readFileSync(setupPath, 'utf-8');
    const newSetup = setupContent.replace(
      /from\s+['"](\.[^'"]+)['"]/g,
      (match, imp) => {
        const resolved = resolveImport('test/setup.ts', imp);
        if (resolved) {
          const rel = path.posix.relative('test', resolved).replace(/\\/g, '/');
          return `from './${rel}'`;
        }
        return match;
      }
    );
    if (newSetup !== setupContent) {
      fs.writeFileSync(setupPath, newSetup, 'utf-8');
      console.log('UPDATED: test/setup.ts');
    }
  }

  console.log('\n=== Migration complete ===');
  console.log(`Files moved: ${Object.keys(MOVE_MAP).length}`);
  console.log(`Files scanned for imports: ${remainingFiles.length}`);
  console.log('\nNOTE: api/index.ts was moved to shared/api/index.old.ts');
  console.log('You need to manually split it into per-feature api files.');
  console.log('App.tsx, main.tsx also need manual import updates.');
}

main();
