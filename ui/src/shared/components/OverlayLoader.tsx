
import { useTranslation } from 'react-i18next';

export function OverlayLoader({ message }) {
  const { t } = useTranslation();
  const label = message || t('common.loading');

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 160,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        backgroundColor: "rgba(0, 0, 0, 0.72)",
        color: "rgba(255,255,255,0.78)",
        fontSize: 14,
        fontWeight: 600,
        letterSpacing: "0.03em",
      }}
    >
      {label}
    </div>
  );
}
