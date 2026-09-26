import { useTranslation } from "react-i18next";
import { m } from "framer-motion";
import { cn } from "../../../../../shared/utils/index";
import { Button } from "../../../../../components/ui/be-ui-button";
import { Icons } from "./Icons";

export function ProviderSelector({ 
  providerMode, 
  onChange, 
  telegramAuthorized 
}) {
  const { t } = useTranslation();

  const providers = [
    { id: "discord", icon: <Icons.Discord />, label: "Discord", available: true },
    { id: "telegram", icon: <Icons.Telegram />, label: "Telegram", available: telegramAuthorized }
  ];

  const handleKeyDown = (e, provider) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (provider.available) {
        onChange(provider.id);
      }
    }
  };

  return (
    <m.div 
      initial={{ opacity: 0, scaleY: 0 }}
      animate={{ opacity: 1, scaleY: 1 }}
      exit={{ opacity: 0, scaleY: 0 }}
      style={{ transformOrigin: "top" }}
      className="config-group"
    >
      <span className="config-group-title">
        {t("upload.providers", "Storage Providers")}
      </span>
      <div className="flex gap-3" role="radiogroup" aria-label={t("upload.providers", "Storage Providers")}>
        {providers.map((p) => {
          const isActive = providerMode === p.id;
          return (
            <Button
              key={p.id}
              variant={isActive ? "primary" : "ghost"}
              size="md"
              role="radio"
              aria-checked={isActive}
              aria-disabled={!p.available}
              onClick={() => p.available && onChange(p.id)}
              onKeyDown={(e) => handleKeyDown(e, p)}
              className={cn(
                "provider-chip flex-1 flex-col",
                isActive && "active",
                !p.available && "opacity-30 grayscale"
              )}
            >
              {p.icon}
              <span className="text-[11px] font-bold uppercase">{p.label}</span>
              {!p.available && p.id === "telegram" && (
                <span className="text-[8px] opacity-60 normal-case">{t("upload.notAuthorized", "Not Auth")}</span>
              )}
            </Button>
          );
        })}
      </div>
    </m.div>
  );
}
