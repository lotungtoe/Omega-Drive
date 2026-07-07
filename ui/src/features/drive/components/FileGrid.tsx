import { useState, useRef, useEffect, useLayoutEffect, useMemo, memo, useContext } from 'react'
import { useTranslation } from 'react-i18next'
import { FileCard } from './FileCard/FileCard'
import { EmptyState } from '../../../shared/components/Common'

import { SortBar } from './Toolbar/SortBar'
import { ListHeader } from './Toolbar/ListHeader'
import {
  DriveControllerContext,
  MainAppUiActionsContext,
  MainAppUiStateContext,
} from '../pages/useMainAppContext'

function FileGridComponent({
  files,
  progressMap,
  view,
  dark,
  onFileDelete,
  onFileDownload,
  onFilePlay,
  onFilePreview,
  onFileStar,
  onFileRestore,
  onFileResume,
  onFileForward,
  onUpload,
  isDragOver,
  sort,
  setSort,
  setCurrentFolderId,
  hasMore,
  loadMore,
  loadingMore
}) {
  const { t } = useTranslation()
  const uiState = useContext(MainAppUiStateContext)
  const uiActions = useContext(MainAppUiActionsContext)
  const driveController = useContext(DriveControllerContext)
  const containerRef = useRef(null)
  const prevFilesRef = useRef(null)
  const [scrollTop, setScrollTop] = useState(0)
  const [selectedId, setSelectedId] = useState(null)

  const resolvedFiles = useMemo(() => files ?? driveController?.files ?? [], [files, driveController?.files])

  const resolvedProgressMap = useMemo(() => progressMap ?? uiState?.progressMap ?? {}, [progressMap, uiState?.progressMap])
  const resolvedView = view ?? uiState?.view ?? 'grid'
  const resolvedDark = dark ?? uiState?.dark ?? false
  const resolvedDelete = onFileDelete ?? driveController?.deleteItem ?? (() => {})
  const resolvedDownload = onFileDownload ?? uiActions?.handleDownload ?? (() => {})
  const resolvedPlay = onFilePlay ?? uiActions?.handlePlay ?? (() => {})
  const resolvedPreview = onFilePreview ?? uiActions?.handlePreview ?? uiActions?.openPreview ?? (() => {})
  const resolvedStar = onFileStar ?? driveController?.toggleStar ?? (() => {})
  const resolvedRestore = onFileRestore ?? driveController?.restoreFile ?? (() => {})
  const resolvedResume = onFileResume ?? uiActions?.resumeUpload ?? (() => {})
  const resolvedForward = onFileForward ?? driveController?.forwardFileToShared ?? (() => {})
  const resolvedUpload = onUpload ?? uiActions?.uploadPaths ?? (() => {})
  const resolvedIsDragOver = isDragOver ?? uiState?.isDragOver ?? false
  const resolvedSort = sort ?? uiState?.sort ?? { field: 'name', dir: 'asc' }
  const resolvedSetSort = setSort ?? uiActions?.setSort ?? (() => {})
  const resolvedSetCurrentFolderId = setCurrentFolderId ?? driveController?.setCurrentFolderId ?? (() => {})
  const resolvedHasMore = hasMore ?? driveController?.filesHasMore ?? false
  const resolvedLoadMore = useMemo(() => loadMore ?? driveController?.loadMore ?? (() => {}), [loadMore, driveController?.loadMore])
  const resolvedLoadingMore = loadingMore ?? driveController?.loadingMore ?? false

  if (resolvedFiles !== prevFilesRef.current) {
    prevFilesRef.current = resolvedFiles
    setSelectedId(null)
  }

  const [viewportHeight, setViewportHeight] = useState(globalThis.innerHeight)
  const [containerWidth, setContainerWidth] = useState(() => {
    return globalThis.innerWidth > 600 ? globalThis.innerWidth - 300 : globalThis.innerWidth
  })

  const useIsomorphicLayoutEffect = typeof globalThis === 'undefined' ? useEffect : useLayoutEffect

  useIsomorphicLayoutEffect(() => {
    let scrollParent = containerRef.current?.closest('[style*="overflow-y: auto"]')
      || containerRef.current?.parentElement

    while (scrollParent && scrollParent !== document.body) {
      const style = globalThis.getComputedStyle(scrollParent)
      if (style.overflowY === 'auto' || style.overflowY === 'scroll') break
      scrollParent = scrollParent.parentElement
    }

    if (!scrollParent || scrollParent === document.body) scrollParent = globalThis

    const updateMetrics = () => {
      if (containerRef.current) {
        const newWidth = containerRef.current.clientWidth
        if (newWidth > 0 && newWidth !== containerWidth) {
          setContainerWidth(newWidth)
        }
      }
      if (scrollParent === globalThis) {
        setViewportHeight(globalThis.innerHeight)
        setScrollTop(globalThis.scrollY)
      } else {
        setViewportHeight(scrollParent.clientHeight)
        setScrollTop(scrollParent.scrollTop)
      }
    }

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const width = entry.contentRect.width
        if (width > 0) {
          setContainerWidth(width)
          globalThis.requestAnimationFrame(updateMetrics)
        }
      }
    })

    if (containerRef.current) {
      resizeObserver.observe(containerRef.current)
      const initialWidth = containerRef.current.clientWidth
      if (initialWidth > 0) setContainerWidth(initialWidth)
    }

    scrollParent.addEventListener('scroll', handleScroll, { passive: true })
    globalThis.addEventListener('resize', updateMetrics)

    updateMetrics()

    function handleScroll() {
      if (scrollParent === globalThis) setScrollTop(globalThis.scrollY)
      else setScrollTop(scrollParent.scrollTop)
    }

    return () => {
      scrollParent.removeEventListener('scroll', handleScroll)
      globalThis.removeEventListener('resize', updateMetrics)
      resizeObserver.disconnect()
    }
  }, [resolvedView, containerWidth])

  const allItems = useMemo(() => resolvedFiles || [], [resolvedFiles])

  const GRID_CONFIG = useMemo(() => ({ gap: 16, itemWidth: 180, itemHeight: 200 }), [])
  const LIST_CONFIG = useMemo(() => ({ gap: 0, itemHeight: 48 }), [])

  const { visibleItems, totalHeight } = useMemo(() => {
    const items = []
    let height = 0
    const cols = Math.max(1, Math.floor((containerWidth + GRID_CONFIG.gap) / (GRID_CONFIG.itemWidth + GRID_CONFIG.gap)))

    if (resolvedView === 'grid') {
      const totalRows = Math.ceil(allItems.length / cols)
      height = totalRows * (GRID_CONFIG.itemHeight + GRID_CONFIG.gap)
      const startRow = Math.max(0, Math.floor(scrollTop / (GRID_CONFIG.itemHeight + GRID_CONFIG.gap)) - 3)
      const endRow = Math.min(totalRows, Math.ceil((scrollTop + viewportHeight) / (GRID_CONFIG.itemHeight + GRID_CONFIG.gap)) + 6)

      for (let i = startRow * cols; i < Math.min(endRow * cols, allItems.length); i++) {
        items.push({
          _idx: i,
          top: Math.floor(i / cols) * (GRID_CONFIG.itemHeight + GRID_CONFIG.gap),
          left: (i % cols) * (GRID_CONFIG.itemWidth + GRID_CONFIG.gap)
        })
      }
    } else {
      height = allItems.length * LIST_CONFIG.itemHeight
      const startIndex = Math.max(0, Math.floor(scrollTop / LIST_CONFIG.itemHeight) - 10)
      const endIndex = Math.min(allItems.length, Math.ceil((scrollTop + viewportHeight) / LIST_CONFIG.itemHeight) + 20)
      for (let i = startIndex; i < endIndex; i++) {
        items.push({ _idx: i, top: i * LIST_CONFIG.itemHeight, left: 0 })
      }
    }
    return { visibleItems: items, totalHeight: height }
  }, [allItems, resolvedView, containerWidth, scrollTop, viewportHeight, GRID_CONFIG, LIST_CONFIG])

  useEffect(() => {
    if (!resolvedHasMore || resolvedLoadingMore || allItems.length === 0) return
    if (scrollTop + viewportHeight >= totalHeight - 800) {
      resolvedLoadMore()
    }
  }, [scrollTop, viewportHeight, totalHeight, resolvedHasMore, resolvedLoadingMore, resolvedLoadMore, allItems.length])

  const isTrash = uiState?.activeSection === 'trash'

  if (allItems.length === 0) {
    return <EmptyState onUpload={resolvedUpload} isDragOver={resolvedIsDragOver} isTrash={isTrash} />
  }

  return (
    <div
      className="w-full"
      role="grid"
      aria-label={t('drive.ariaFileList')}
      onClick={() => setSelectedId(null)}
      onKeyDown={(e) => {
        if (e.key === 'Escape') setSelectedId(null)
      }}
      tabIndex={0}
      style={{ outline: 'none' }}
    >
      {resolvedView === 'grid' && <SortBar sort={resolvedSort} setSort={resolvedSetSort} />}
      {resolvedView === 'list' && (
        <ListHeader 
          sort={resolvedSort} 
          setSort={resolvedSetSort} 
          dark={resolvedDark} 
          isShared={uiState?.activeSection === 'shareddrive'} 
        />
      )}

      <div
        ref={containerRef}
        style={{ height: (totalHeight || 1) + (resolvedLoadingMore ? 60 : 0), position: 'relative', width: '100%', minHeight: '1px' }}
      >
        {visibleItems.map(item => {
          const file = allItems[item._idx]
          return (
          <div
            key={`item-${file.isFolder ? 'folder' : 'file'}-${file.id}`}
            style={{
              position: 'absolute',
              top: item.top,
              left: item.left,
              width: resolvedView === 'grid' ? GRID_CONFIG.itemWidth : '100%',
              height: resolvedView === 'grid' ? GRID_CONFIG.itemHeight : LIST_CONFIG.itemHeight,
              transition: 'opacity 0.2s ease',
              zIndex: file.isFolder ? 10 : 1
            }}
          >
            <FileCard
              file={file} view={resolvedView} dark={resolvedDark} progressMap={resolvedProgressMap}
              isSelected={selectedId === file.id}
              isShared={uiState?.activeSection === 'shareddrive'}
              onDownload={() => resolvedDownload(file)} onDelete={() => resolvedDelete(file)}
              onPlay={() => resolvedPlay(file)}
              onPreview={() => resolvedPreview(file)}
              onRestore={() => resolvedRestore(file.id)} onToggleStar={() => resolvedStar(file)}
              onResume={() => resolvedResume(file)}
              onForward={() => resolvedForward(file.id)}
              setCurrentFolderId={resolvedSetCurrentFolderId}
              onSelect={setSelectedId}
            />
          </div>
          )
        })}
        {resolvedLoadingMore && (
           <div style={{ position: 'absolute', top: totalHeight, left: 0, width: '100%', padding: '20px', textAlign: 'center', zIndex: 100 }}>
             <div className="w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full animate-spin mx-auto"></div>
           </div>
        )}
      </div>
    </div>
  )
}

const FileGridMemo = memo(FileGridComponent, (prev, next) => {
  return (
    prev.files === next.files &&
    prev.progressMap === next.progressMap &&
    prev.view === next.view &&
    prev.dark === next.dark &&
    prev.isDragOver === next.isDragOver &&
    prev.sort === next.sort &&
    prev.hasMore === next.hasMore &&
    prev.loadingMore === next.loadingMore
  )
})

export const FileGrid = FileGridMemo
