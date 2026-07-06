import { save } from "@tauri-apps/plugin-dialog";
import { DriveApi } from "../../../api/index";

type DownloadableFile = {
  filename?: string | null;
};

export async function selectDownloadPath(file: DownloadableFile): Promise<string | null> {
  const options = {
    title: "Save file",
  };

  if (file.filename) {
    return save({ ...options, defaultPath: file.filename });
  }

  return save(options);
}

export async function startDownload(fileId: number | string, savePath: string): Promise<unknown> {
  return DriveApi.queueDownload(fileId, savePath);
}

export async function listDownloadJobs(): Promise<unknown> {
  return DriveApi.listDownloadJobs();
}

export async function pauseDownload(jobId: number | string): Promise<unknown> {
  return DriveApi.pauseDownload(jobId);
}

export async function resumeDownload(jobId: number | string): Promise<unknown> {
  return DriveApi.resumeDownload(jobId);
}

export async function cancelDownload(jobId: number | string): Promise<unknown> {
  return DriveApi.cancelDownload(jobId);
}

export async function retryDownload(jobId: number | string): Promise<unknown> {
  return DriveApi.retryDownload(jobId);
}

export async function openDownloadFile(path: string): Promise<unknown> {
  return DriveApi.openDownloadFile(path);
}

export async function openDownloadFolder(path: string): Promise<unknown> {
  return DriveApi.openDownloadFolder(path);
}
