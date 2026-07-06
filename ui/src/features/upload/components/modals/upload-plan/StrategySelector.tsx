import { useTranslation } from "react-i18next";
import { cn } from "../../../../../shared/utils/index";

export function StrategySelector({ strategy, onChange }) {
  const { t } = useTranslation();

  const strategies = [
    { id: "none", label: t("upload.none", "None"), sub: "Direct" },
    { id: "safe", label: t("upload.safe", "Safe"), sub: "Mirror" },
    { id: "fast", label: t("upload.fast", "Fast"), sub: "Parallel" }
  ];

  const getStrategyDescription = (id) => {
    switch (id) {
      case "fast":
        return t("upload.fastDesc", "Optimizes speed by splitting files and storing across providers.");
      case "safe":
        return t("upload.safeDesc", "Maximizes data safety by creating redundant copies across all platforms.");
      case "none":
      default:
        return t("upload.noneDesc", "Uploads the original file directly without splitting.");
    }
  };

  const handleKeyDown = (e, id) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onChange(id);
    }
  };

  return (
    <div className="config-group">
      <span className="config-group-title">
        {t("upload.strategy", "Storage Strategy")}
      </span>
      <div className="flex flex-col gap-3">
        <div 
          className="strategy-toggle h-14"
          role="radiogroup"
          aria-label={t("upload.strategy", "Storage Strategy")}
        >
          <div 
            className="strategy-toggle-thumb" 
            style={{ 
              width: "calc(33.33% - 5.33px)",
              transform: `translateX(${(() => {
                if (strategy === "safe") return "calc(100% + 4px)";
                if (strategy === "fast") return "calc(200% + 8px)";
                return "0";
              })()})` 
            }}
          />
          {strategies.map((s) => (
            <div 
              key={s.id}
              role="radio"
              aria-checked={strategy === s.id}
              tabIndex={0}
              className="relative z-10 flex-1 text-center flex flex-col items-center justify-center cursor-pointer outline-none rounded-xl"
              onClick={() => onChange(s.id)}
              onKeyDown={(e) => handleKeyDown(e, s.id)}
            >
              <span className={cn(
                "text-[10px] font-bold uppercase leading-none transition-colors",
                strategy === s.id ? "text-white" : "opacity-60"
              )}>
                {s.label}
              </span>
              <span className={cn(
                "text-[8px] font-bold uppercase mt-1 transition-opacity",
                strategy === s.id ? "text-white/70" : "opacity-40"
              )}>
                {s.sub}
              </span>
            </div>
          ))}
        </div>
        <div className="strategy-desc" aria-live="polite">
          {getStrategyDescription(strategy)}
        </div>
      </div>
    </div>
  );
}
