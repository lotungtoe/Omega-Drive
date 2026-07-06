import { useTranslation } from "react-i18next";

import { cn } from "../../../../../shared/utils/index";
import { Button } from "../../../../../components/ui/be-ui-button";



export function ProfileSidebar({ 

  profiles, 

  activeProfileId, 

  onSelectProfile, 

  onDeleteProfile, 

  onCreateProfile,

  isCreating 

}) {

  const { t } = useTranslation();



  return (

    <nav className="profile-sidebar" aria-label={t("upload.profiles", "Profiles")}>

      <div className="flex items-center justify-between pb-2 mb-2 border-b border-[var(--gd-outline-variant)]">

        <span className="text-[10px] font-bold uppercase opacity-50">

          {t("upload.profiles", "PROFILES")}

        </span>

        <Button

          variant="ghost"
          size="sm"
          onClick={onCreateProfile}

          disabled={isCreating}

          aria-label={t("upload.createNew", "Create new profile")}

        >

          {isCreating ? "..." : t("upload.createNew", "+ NEW")}

        </Button>

      </div>

      <ul 

        className="flex flex-col gap-2 overflow-y-auto scrollbar-none pr-1 m-0 p-0"

        aria-label={t("upload.profileList", "Profile list")}

      >

        {profiles.map((p) => {

          const isActive = activeProfileId === p.id;

          return (

            <li key={p.id} className="relative group">

              <Button

                variant="ghost"
                size="md"
                onClick={() => onSelectProfile(p.id)}

                className={cn(

                  "w-full text-left",

                  isActive && "active"

                )}

                aria-current={isActive ? "true" : "false"}

              >

                <div className="flex justify-between items-center w-full">

                  <span className="text-xs font-bold truncate pr-2">{p.name}</span>

                  <div className={cn(

                    "w-1.5 h-1.5 rounded-full transition-all",

                    isActive ? "bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.5)]" : "bg-black/10 dark:bg-white/10"

                  )} />

                </div>

              </Button>

              <Button

                variant="ghost"
                size="icon"
                onClick={(e) => {

                  e.stopPropagation();

                  onDeleteProfile(p.id);

                }}

                className="absolute -top-1 -right-1 z-20 !h-4 !w-4 !rounded-full bg-red-500 text-white opacity-0 group-hover:opacity-100 transition-all hover:scale-110 shadow-lg focus:opacity-100"

                aria-label={t("upload.deleteProfile", "Delete profile")}

              >

                <span className="text-[8px]">✕</span>

              </Button>

            </li>

          );

        })}

      </ul>

    </nav>

  );

}

