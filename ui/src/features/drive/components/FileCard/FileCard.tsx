import { useState, useRef, useEffect, memo } from 'react'
import { useDraggable, useDroppable } from '@dnd-kit/core'
import { useTranslation } from 'react-i18next'
import { open } from "@tauri-apps/plugin-dialog";
import { Star, Play, Eye, Download, Trash2, MoreVertical, RotateCcw, Copy, Users } from 'lucide-react'
import { FileIcon, FolderIcon } from '../../../../shared/components/Icons'
import { getFileType, formatSize, formatDateSafe, cn } from '../../../../shared/utils/index'
import { resumeUploadTask } from '../../../upload/services/uploadService'
import { toUserMessage } from '../../../../shared/services/errors/toUserMessage'

function useFileCardLogic(props) {
  const { t } = useTranslation()
  const { file, setCurrentFolderId, onPreview } = props
  const [menuOpen, setMenuOpen] = useState(false)
  const menuRef = useRef<any>(null)

  const isFolder = file.isFolder
  const fileType = isFolder
    ? { labelKey: 'fileType.folder', group: 'folder', ext: '' }
    : getFileType(file.filename, file.kind)
  const date = formatDateSafe(file.created_at || file.last_modified)
  const isVideo = !isFolder && ['video'].includes(fileType.group)
  const isStarred = file.starred
  const isError = file.status === 'error'
  const isTrashed = file.status === 'trashed'
  const isSharedFile = !isFolder && file.drive_scope === 'shared'

  useEffect(() => {
    const handler = (e) => {
      if (menuRef.current && !menuRef.current.contains(e.target)) setMenuOpen(false)
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [])

  // --- dnd-kit: Draggable hook (all items can be dragged) ---
  const dragId = isFolder ? `folder-${file.id}` : `file-${file.id}`;
  const { attributes: dragAttributes, listeners: dragListeners, setNodeRef: setDragRef, isDragging } = useDraggable({
    id: dragId,
    data: {
      type: isFolder ? 'folder' : 'file',
      id: file.id,
      name: isFolder ? file.name : file.filename,
      isFolder,
      driveScope: file.drive_scope || null,
    },
  });

  // --- dnd-kit: Droppable hook (only folders accept drops) ---
  const { setNodeRef: setDropRef, isOver } = useDroppable({
    id: `drop-folder-${file.id}`,
    disabled: !isFolder,
    data: { type: 'folder', id: file.id, targetScope: file.drive_scope || null },
  });

  // Merge drag + drop refs into one callback ref
  const mergedRef = (node) => {
    setDragRef(node);
    if (isFolder) setDropRef(node);
  };

  const handleDoubleClick = () => {
    if (isFolder) {
      setCurrentFolderId(file.id);
    } else {
      onPreview();
    }
  };

  return {
    menuOpen, setMenuOpen, menuRef,
    isFolder, fileType, date, isVideo, isStarred, isError, isTrashed, isSharedFile,
    isImage: !isFolder && ['image'].includes(fileType.group),
    handleDoubleClick,
    // dnd-kit refs & state
    mergedRef, dragAttributes, dragListeners, isDragging, isOver,
    handleSelect: (e?) => {
      if (props.onSelect) {
        if (e) e.stopPropagation();
        props.onSelect(file.id);
      }
    },
    handleResume: async (e) => {
      if (e) e.stopPropagation();
      try {
        const currentPath = file.local_path;
        
        const resume = async (path) => {
          const sessionId = `resume-${file.id}`;
          await resumeUploadTask(sessionId, file.id, path, file.folder_id, file.drive_scope || 'my');
        };

        if (currentPath) {
          await resume(currentPath);
        } else {
          const selected = await open({
            multiple: false,
            directory: false,
            title: t('upload.resumePickFile', { filename: file.filename })
          });
          if (selected) await resume(selected);
        }
      } catch (err) {
        const msg = toUserMessage(err);
        console.error("Resume error:", err);
        if (err.toString().includes("not found") || err.toString().includes("No such file")) {
           const selected = await open({
            multiple: false,
            directory: false,
            title: t('upload.resumePickFile', { filename: file.filename })
          });
          if (selected) {
             const sessionId = `resume-${file.id}`;
             await resumeUploadTask(sessionId, file.id, selected, file.folder_id, file.drive_scope || 'my');
          }
        } else if (typeof globalThis !== 'undefined') {
          globalThis.alert(msg.message);
        }
      }
    }
  }
}

function ListActions({ 
  isFolder, isVideo, isImage, isStarred, isTrashed, isSharedFile, status, 
  onDownload, onDelete, onRestore, onPlay, onPreview, onToggleStar, onResume, onForward 
}) {
  const { t } = useTranslation()
  const canPlayVideo = isVideo && status === 'ready'
  const canResumeUpload = ['uploading', 'processing'].includes(status)

  if (isTrashed) {
    return (
      <div 
        onPointerDown={(e) => e.stopPropagation()}
        style={{ width: 120, display: 'flex', alignItems: 'center', justifyContent: 'flex-end', gap: 2, flexShrink: 0, opacity: 0, transition: 'opacity 0.15s' }}
        className="group-hover:!opacity-100"
      >
        <button type="button" onClick={onRestore} className="gd-icon-btn" style={{ width: 32, height: 32 }} title={t('drive.restore')}>
          <RotateCcw size={16} />
        </button>
        <button type="button" onClick={onDelete} className="gd-icon-btn" style={{ width: 32, height: 32 }} title={t('common.delete')}>
          <Trash2 size={16} />
        </button>
      </div>
    )
  }

  return (
    <div 
      onPointerDown={(e) => e.stopPropagation()}
      style={{ width: 120, display: 'flex', alignItems: 'center', justifyContent: 'flex-end', gap: 2, flexShrink: 0, opacity: 0, transition: 'opacity 0.15s' }}
      className="group-hover:!opacity-100"
    >
      {!isFolder && (
        <>
          <button type="button" onClick={onToggleStar} className="gd-icon-btn" style={{ width: 32, height: 32 }} title={isStarred ? t('drive.unstar') : t('drive.star')}>
            <Star size={16} fill={isStarred ? '#fbbc04' : 'none'} color={isStarred ? '#fbbc04' : '#5f6368'} />
          </button>
          {canPlayVideo && (
            <button type="button" onClick={onPlay} className="gd-icon-btn" style={{ width: 32, height: 32 }} title={t('player.play')}>
              <Play size={16} color="#4285f4" fill="#4285f4" />
            </button>
          )}
          {isImage && status === 'ready' && (
            <button type="button" onClick={onPreview} className="gd-icon-btn" style={{ width: 32, height: 32 }} title={t('drive.preview')}>
              <Eye size={16} color="#4285f4" />
            </button>
          )}
          <button type="button" onClick={onDownload} className="gd-icon-btn" style={{ width: 32, height: 32 }} title={t('drive.download')}>
            <Download size={16} />
          </button>
          {!isSharedFile && (
            <button type="button" onClick={onForward} className="gd-icon-btn" style={{ width: 32, height: 32 }} title={t('drive.moveToShared')}>
              <Copy size={16} />
            </button>
          )}
          {canResumeUpload && (
            <button type="button" onClick={(e) => { e.stopPropagation(); onResume(); }} className="gd-icon-btn" style={{ width: 32, height: 32, color: '#fb8c00' }} title={t('upload.resumeUpload')}>
              <Play size={16} fill="#fb8c00" />
            </button>
          )}
        </>
      )}
      <button type="button" onClick={onDelete} className="gd-icon-btn" style={{ width: 32, height: 32 }} title={t('common.delete')}>
        <Trash2 size={16} />
      </button>
    </div>
  )
}

function GridMenu({ 
  isFolder, isVideo, isStarred, isTrashed, status, fileId, isSharedFile,
  setCurrentFolderId, onPlay, onRestore, onPreview, onDownload, onToggleStar, onDelete, onResume, onForward, setMenuOpen 
}) {
  const { t } = useTranslation()
  const canPlayVideo = isVideo && status === 'ready'
  const canResumeUpload = ['uploading', 'processing'].includes(status)

  return (
    <div 
      onPointerDown={(e) => e.stopPropagation()}
      className="gd-menu show"
      style={{ position: 'absolute', right: 0, top: 32, zIndex: 50 }}
    >
      {isFolder ? (
        <button type="button" className="gd-menu-item" onClick={() => { setCurrentFolderId(fileId); setMenuOpen(false) }}>
          <Eye size={18} /> {t('drive.openFolder')}
        </button>
      ) : (
        <>
          {canPlayVideo && (
            <button type="button" className="gd-menu-item" onClick={() => { onPlay(); setMenuOpen(false) }}>
              <Play size={18} /> {t('player.play')}
            </button>
          )}
          {isTrashed ? (
            <button type="button" className="gd-menu-item" onClick={() => { onRestore(); setMenuOpen(false) }}>
              <RotateCcw size={18} /> {t('drive.restore')}
            </button>
          ) : (
            <>
              <button type="button" className="gd-menu-item" onClick={() => { onPreview(); setMenuOpen(false) }}>
                <Eye size={18} /> {t('drive.preview')}
              </button>
              <button type="button" className="gd-menu-item" onClick={() => { onDownload(); setMenuOpen(false) }}>
                <Download size={18} /> {t('drive.download')}
              </button>
              <button type="button" className="gd-menu-item" onClick={() => { onToggleStar(); setMenuOpen(false) }}>
                <Star size={18} fill={isStarred ? '#fbbc04' : 'none'} color={isStarred ? '#fbbc04' : 'currentColor'} /> 
                {isStarred ? t('drive.unstar') : t('drive.star')}
              </button>
              {!isSharedFile && (
                <button type="button" className="gd-menu-item" onClick={() => { onForward(); setMenuOpen(false) }}>
                  <Copy size={18} /> {t('drive.moveToShared')}
                </button>
              )}
            </>
          )}
        </>
      )}
      <div className="gd-menu-divider" />
      <button type="button" className="gd-menu-item danger" onClick={() => { onDelete(); setMenuOpen(false) }}>
        <Trash2 size={18} /> {isTrashed ? t('drive.deleteForever') : t('common.delete')}
      </button>
      {canResumeUpload && (
        <button type="button" className="gd-menu-item" style={{ color: '#fb8c00' }} onClick={() => { onResume(); setMenuOpen(false) }}>
          <Play size={18} fill="#fb8c00" /> {t('upload.resumeUpload')}
        </button>
      )}
    </div>
  )
}

function FileCardList(props) {
  const { t } = useTranslation()
  const { file, isSelected, onDownload, onDelete, onRestore, onPlay, onPreview, onToggleStar, onForward, progressPercentage } = props
  const {
    isFolder, fileType, date, isVideo, isStarred, isError, isTrashed, isImage, isSharedFile,
    handleDoubleClick, handleSelect, handleResume,
    mergedRef, dragAttributes, dragListeners, isDragging, isOver,
  } = useFileCardLogic(props)

  return (
    <div
      ref={mergedRef}
      {...dragAttributes}
      {...dragListeners}
      role="treeitem"
      aria-selected={isSelected}
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          if (!isError) handleDoubleClick()
        } else if (e.key === ' ') {
          e.preventDefault();
          if (!isError) handleSelect()
        }
      }}
      onClick={handleSelect}
      className={cn(
        "gd-list-row group transition-all duration-200 ease-out",
        isSelected && "is-selected",
        isError && "opacity-60 grayscale",
        isDragging && "opacity-40 scale-[0.98]",
        isOver && isFolder && "ring-2 ring-blue-400 bg-blue-50/30"
      )}
      onDoubleClick={isError ? null : handleDoubleClick}
      style={{ zIndex: 1, position: 'relative', backgroundColor: isOver && isFolder ? 'var(--gd-blue-surface)' : undefined }}
    >
      <div style={{ width: 36, height: 36, display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, position: 'relative' }}>
        {isFolder ? <FolderIcon size={32} /> : <FileIcon filename={file.filename} kind={file.kind} size={24} />}
        {isStarred && (
          <div style={{
            position: 'absolute',
            top: -2,
            left: -2,
            backgroundColor: 'var(--gd-surface)',
            borderRadius: '50%',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: 1,
          }}>
            <Star size={11} fill="#fbbc04" color="#fbbc04" />
          </div>
        )}
      </div>

      <div style={{ flex: 1, minWidth: 0, paddingLeft: 8 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
          <div style={{ 
            flex: 1,
            minWidth: 0,
            fontSize: 14, 
            fontFamily: "'Google Sans', sans-serif",
            color: 'var(--gd-on-surface)', 
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis'
          }}>
            {isFolder ? file.name : file.filename}
            {isError && <span style={{ marginLeft: 8, color: 'var(--gd-error)', fontSize: 11 }}>[{t('common.error')}]</span>}
            {['uploading', 'processing'].includes(file.status) && (
              <span style={{ marginLeft: 8, color: '#fb8c00', fontSize: 11, fontWeight: 500 }}>
                [{t('upload.inProgress', { percent: progressPercentage ?? Math.round((file.parts_done / (file.parts_total || 1)) * 100) })}]
              </span>
            )}
          </div>
        </div>
      </div>

      {props.isShared && (
        <div style={{ width: 180, flexShrink: 0, display: 'flex', alignItems: 'center', gap: 8, paddingLeft: 12 }} className="hidden lg:flex">
          {file.sharer_name ? (
            <>
              <div style={{ width: 24, height: 24, borderRadius: '50%', backgroundColor: 'var(--gd-secondary-container)', overflow: 'hidden', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0 }}>
                {file.sharer_avatar ? (
                  <img src={file.sharer_avatar} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover' }} />
                ) : (
                  <span style={{ fontSize: 10, color: 'var(--gd-on-secondary-container)', fontWeight: 600 }}>
                    {file.sharer_name.charAt(0).toUpperCase()}
                  </span>
                )}
              </div>
              <span style={{ fontSize: 12, color: 'var(--gd-on-surface)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                {file.sharer_name}
              </span>
            </>
          ) : (
            <span style={{ fontSize: 12, color: 'var(--gd-on-surface-variant)', opacity: 0.5 }}>--</span>
          )}
        </div>
      )}

      <div style={{ width: 100, flexShrink: 0, textAlign: 'center' }} className="hidden lg:block">
        <span style={{ fontSize: 12, color: 'var(--gd-on-surface-variant)' }}>
          {t(fileType.labelKey, { ext: (fileType.ext || '').toUpperCase() })}
        </span>
      </div>

      <div style={{ width: 160, flexShrink: 0, fontSize: 12, color: 'var(--gd-on-surface-variant)', textAlign: 'center' }} className="hidden md:block">
        {date}
      </div>

      <div style={{ width: 100, flexShrink: 0, fontSize: 12, color: 'var(--gd-on-surface-variant)', textAlign: 'center' }} className="hidden sm:block">
        {isFolder ? '--' : formatSize(file.size)}
      </div>

      <ListActions 
        isFolder={isFolder}
        isVideo={isVideo}
        isStarred={isStarred}
        isTrashed={isTrashed}
        isImage={isImage}
        onDownload={onDownload}
        onDelete={onDelete}
        onRestore={onRestore}
        onPlay={onPlay}
        onPreview={onPreview}
        onToggleStar={onToggleStar}
        status={file.status}
        isSharedFile={isSharedFile}
        onResume={handleResume}
        onForward={onForward}
      />
    </div>
  )
}

const PreviewSection = ({ file, isStarred, isImage, onToggleStar, onPreview, previewBg, t }) => (
  <div style={{ 
    height: 140,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    borderBottom: '1px solid var(--gd-outline-variant)',
    backgroundColor: previewBg,
    position: 'relative',
    borderTopLeftRadius: 'var(--gd-radius-sm)',
    borderTopRightRadius: 'var(--gd-radius-sm)',
    transition: 'background-color 0.2s ease-out'
  }}>
    {file.isFolder ? <FolderIcon size={48} /> : <FileIcon filename={file.filename} kind={file.kind} size={48} />}

    <button type="button"
      onPointerDown={(e) => e.stopPropagation()}
      onClick={(e) => { e.stopPropagation(); onToggleStar(); }}
      style={{ position: 'absolute', top: 8, right: 8, opacity: isStarred ? 1 : 0, transition: 'opacity 0.15s' }}
      className="gd-icon-btn group-hover:!opacity-100"
    >
      <Star size={18} fill={isStarred ? '#fbbc04' : 'none'} color={isStarred ? '#fbbc04' : '#5f6368'} />
    </button>

    {isImage && file.status === 'ready' && (
      <div style={{ position: 'absolute', bottom: 8, right: 8, display: 'flex', gap: 6, opacity: 0, transition: 'opacity 0.15s' }} className="group-hover:!opacity-100">
        <button type="button"
          onPointerDown={(e) => e.stopPropagation()}
          onClick={(e) => { e.stopPropagation(); onPreview(); }}
          style={{ 
            width: 32, height: 32, borderRadius: '50%', backgroundColor: 'rgba(0, 0, 0, 0.6)', 
            display: 'flex', alignItems: 'center', justifyContent: 'center', border: 'none', cursor: 'pointer' 
          }}
          title={t('drive.preview')}
        >
          <Eye size={16} color="white" />
        </button>
      </div>
    )}
  </div>
);

const DetailsSection = ({ 
  file, menuOpen, setMenuOpen, menuRef, isFolder, isVideo, isStarred, isTrashed, 
  isSharedFile, setCurrentFolderId, onPlay, onRestore, onPreview, onDownload, onToggleStar, onDelete, 
  handleResume, onForward, progressPercentage, t 
}) => (
  <div style={{ padding: '10px 12px', display: 'flex', alignItems: 'center', gap: 8, flex: 1 }}>
    {isFolder ? <FolderIcon size={16} /> : <FileIcon filename={file.filename} kind={file.kind} size={14} />}
    <div style={{ 
      flex: 1, minWidth: 0, fontSize: 13, fontFamily: "'Google Sans', sans-serif", fontWeight: 500, 
      color: 'var(--gd-on-surface)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' 
    }} title={isFolder ? file.name : file.filename}>
      {isFolder ? file.name : file.filename}
      {file.status === 'error' && <span style={{ marginLeft: 8, color: 'var(--gd-error)', fontSize: 11 }}>[{t('common.error')}]</span>}
      {['uploading', 'processing'].includes(file.status) && (
        <span style={{ marginLeft: 8, color: '#fb8c00', fontSize: 11, fontWeight: 500 }}>
          [{t('upload.inProgress', { percent: progressPercentage ?? Math.round((file.parts_done / (file.parts_total || 1)) * 100) })}]
        </span>
      )}
    </div>
    {isSharedFile && (
      <span
        title={t('sidebar.sharedDrive')}
        aria-label={t('sidebar.sharedDrive')}
        style={{ display: 'inline-flex', alignItems: 'center', color: 'var(--gd-on-surface-variant)', flexShrink: 0 }}
      >
        <Users size={14} />
      </span>
    )}

    <div ref={menuRef} style={{ position: 'relative', flexShrink: 0 }}>
      <button type="button"
        onPointerDown={(e) => e.stopPropagation()}
        onClick={(e) => { e.stopPropagation(); setMenuOpen(v => !v); }}
        className="gd-icon-btn group-hover:!opacity-100"
        style={{ width: 28, height: 28, opacity: menuOpen ? 1 : 0, transition: 'opacity 0.15s' }}
      >
        <MoreVertical size={16} />
      </button>
      
      {menuOpen && (
        <GridMenu 
          isFolder={isFolder}
          isVideo={isVideo}
          isStarred={isStarred}
          isTrashed={isTrashed}
          isSharedFile={isSharedFile}
          fileId={file.id}
          setCurrentFolderId={setCurrentFolderId}
          onPlay={onPlay}
          onRestore={onRestore}
          onPreview={onPreview}
          onDownload={onDownload}
          onToggleStar={onToggleStar}
          onDelete={onDelete}
          status={file.status}
          onResume={handleResume}
          onForward={onForward}
          setMenuOpen={setMenuOpen}
        />
      )}
    </div>
  </div>
);

function FileCardGrid(props) {
  const { t } = useTranslation()
  const { file, dark, isSelected, onDownload, onDelete, onRestore, onPlay, onPreview, onToggleStar, onForward, setCurrentFolderId, progressPercentage } = props
  const {
    menuOpen, setMenuOpen, menuRef,
    isFolder, isVideo, isStarred, isError, isTrashed, isImage, isSharedFile,
    handleDoubleClick, handleSelect, handleResume,
    mergedRef, dragAttributes, dragListeners, isDragging, isOver,
  } = useFileCardLogic(props)

  let zIndex = 1;
  if (isDragging) zIndex = 9999;
  if (menuOpen) zIndex = 100;

  const containerBg = isOver && isFolder ? 'var(--gd-blue-surface)' : undefined;
  
  let previewBg = dark ? '#2d2e30' : '#f8f9fa';
  if ((isOver && isFolder) || isSelected) {
    previewBg = 'transparent';
  }

  return (
    <div
      ref={mergedRef}
      {...dragAttributes}
      {...dragListeners}
      role="treeitem"
      aria-selected={isSelected}
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          if (!isError) handleDoubleClick()
        } else if (e.key === ' ') {
          e.preventDefault();
          if (!isError) handleSelect()
        }
      }}
      onClick={handleSelect}
      className={cn(
        "gd-file-card group transition-all duration-200 ease-out",
        isSelected && "is-selected",
        isError && "opacity-60 grayscale",
        isDragging && "opacity-40 scale-[0.95]",
        isOver && isFolder && "ring-2 ring-blue-400"
      )}
      onDoubleClick={isError ? null : handleDoubleClick}
      style={{ zIndex, backgroundColor: containerBg }}
    >
      <PreviewSection 
        file={file}
        isStarred={isStarred}
        isImage={isImage}
        onToggleStar={onToggleStar}
        onPreview={onPreview}
        previewBg={previewBg}
        t={t}
      />
      
      <DetailsSection 
        file={file}
        menuOpen={menuOpen}
        setMenuOpen={setMenuOpen}
        menuRef={menuRef}
        isFolder={isFolder}
        isVideo={isVideo}
        isStarred={isStarred}
        isTrashed={isTrashed}
        isSharedFile={isSharedFile}
        setCurrentFolderId={setCurrentFolderId}
        onPlay={onPlay}
        onRestore={onRestore}
        onPreview={onPreview}
        onDownload={onDownload}
        onToggleStar={onToggleStar}
        onDelete={onDelete}
        handleResume={handleResume}
        onForward={onForward}
        progressPercentage={progressPercentage}
        t={t}
      />
    </div>
  )
}

function FileCardInner(props) {
  const { file, view, progressMap } = props;
  let progressPercentage = null;

  if (file && ['uploading', 'processing'].includes(file.status)) {
    const pmap = progressMap || {};
    const liveSession = pmap[`up-${file.id}`] || pmap[`resume-${file.id}`];
    
    if (typeof liveSession?.percentage === 'number') {
      progressPercentage = liveSession.percentage;
    } else {
      const done = file.parts_done || 0;
      const total = file.parts_total || 1;
      progressPercentage = Math.round((done / total) * 100);
    }
  }

  if (view === 'list') {
    return <FileCardList {...props} progressPercentage={progressPercentage} />
  }
  return <FileCardGrid {...props} progressPercentage={progressPercentage} />
}

export const FileCard = memo(FileCardInner, (prev, next) => {
  if (prev.file.id !== next.file.id) return false;
  if (prev.file.status !== next.file.status) return false;
  if (prev.file.starred !== next.file.starred) return false;
  if (prev.isSelected !== next.isSelected) return false;
  if (prev.view !== next.view) return false;
  if (prev.dark !== next.dark) return false;

  const getProgress = (map, id) => map?.[`up-${id}`] || map?.[`resume-${id}`] || map?.[`dl-${id}`];
  
  const p1 = getProgress(prev.progressMap, prev.file.id);
  const p2 = getProgress(next.progressMap, next.file.id);

  if (p1?.percentage !== p2?.percentage) return false;
  if (p1?.phase !== p2?.phase) return false;

  return true;
});
