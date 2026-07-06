import { ERROR_CODES } from "./types";
import type { AppError } from "./types";

const FRIENDLY: Record<string, string> = {
  [ERROR_CODES.INVALID_INPUT]: "Dữ liệu nhập không hợp lệ.",
  [ERROR_CODES.NOT_FOUND]: "Không tìm thấy dữ liệu yêu cầu.",
  [ERROR_CODES.CONFLICT]: "Dữ liệu đang bị xung đột, vui lòng thử lại.",
  [ERROR_CODES.PERMISSION]: "Bạn không có quyền thực hiện thao tác này.",
  [ERROR_CODES.DB]: "Hệ thống lưu trữ gặp sự cố.",
  [ERROR_CODES.IO]: "Không thể đọc/ghi dữ liệu trên máy.",
  [ERROR_CODES.JSON]: "Dữ liệu cấu hình bị lỗi định dạng.",
  [ERROR_CODES.NETWORK]: "Không thể kết nối tới dịch vụ.",
  [ERROR_CODES.TIMEOUT]: "Yêu cầu bị timeout, vui lòng thử lại.",
  [ERROR_CODES.UNAVAILABLE]: "Dịch vụ tạm thời không sẵn sàng.",
  [ERROR_CODES.NOT_READY]: "Tài nguyên chưa sẵn sàng.",
  [ERROR_CODES.UPLOAD_FAILED]: "Tải lên thất bại.",
  [ERROR_CODES.UPLOAD_CONFLICT]: "Tệp trùng lặp, cần xác nhận ghi đè.",
  [ERROR_CODES.DOWNLOAD_FAILED]: "Tải xuống thất bại.",
  [ERROR_CODES.PLAYER_UNSUPPORTED]: "Video không hỗ trợ phát trên WebView.",
  [ERROR_CODES.PLAYER_INIT_FAILED]: "Không thể khởi tạo trình phát.",
  [ERROR_CODES.SETTINGS_INVALID]: "Cấu hình không hợp lệ.",
};

type UserMessage = {
  title: string;
  message: string;
  details: Record<string, unknown>;
};

export function toUserMessage(appError: AppError | string | null | undefined): UserMessage {
  if (typeof appError === "string") {
    return {
      title: "Có lỗi xảy ra",
      message: appError,
      details: {},
    };
  }

  const code = appError?.code || ERROR_CODES.UNKNOWN;
  const message = FRIENDLY[code] || appError?.message || "Đã xảy ra lỗi không xác định.";
  const details = {
    code,
    message: appError?.message,
    source: appError?.source,
    context: appError?.context,
    stack: appError?.stack,
    retryable: appError?.retryable,
  };

  return {
    title: "Có lỗi xảy ra",
    message,
    details,
  };
}
