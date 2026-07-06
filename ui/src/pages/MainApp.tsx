import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { uploadAudioAttachment, addAudioTrack } from '../features/player/services/playerService';
import { MainAppContent } from '../features/drive/pages/MainAppContent';
import { MainAppProvider } from '../features/drive/pages/MainAppProvider';

function AttachmentPickerListener() {
  useEffect(() => {
    const unlisten = listen<{ file_id: number; type: string }>(
      'open-attachment-picker',
      async (event) => {
        const { file_id: videoFileId, type } = event.payload;
        if (type !== 'audio') return;

        const selected = await open({
          multiple: false,
          filters: [{ name: 'Audio Files', extensions: ['mp3', 'aac', 'ogg', 'wav', 'flac', 'm4a', 'opus'] }],
        });
        if (!selected || Array.isArray(selected)) return;

        try {
          const audioFileId = await uploadAudioAttachment(videoFileId, selected);
          await addAudioTrack(videoFileId, audioFileId);
        } catch (err) {
          console.error('[AudioAttach] Upload failed:', err);
        }
      },
    );
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  return null;
}

function MainApp() {
  return (
    <MainAppProvider>
      <AttachmentPickerListener />
      <MainAppContent />
    </MainAppProvider>
  );
}

export { MainApp };
export default MainApp;
