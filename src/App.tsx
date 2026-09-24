import { useEffect } from "react";
import { useUi } from "@/store/ui";
import { useBootstrap } from "@/hooks/use-bulk-apply";
import { TitleBar } from "@/components/titlebar";
import { Sidebar } from "@/components/sidebar";
import { TweakDetailsDrawer } from "@/components/tweak-row";
import { DashboardPage } from "@/pages/dashboard";
import { CategoryPage } from "@/pages/category";
import { TweaksPage } from "@/pages/tweaks";
import { CleanupPage } from "@/pages/cleanup";
import { ChangesPage } from "@/pages/changes";
import { BackupPage } from "@/pages/backup";
import { LogsPage } from "@/pages/logs";
import { SettingsPage } from "@/pages/settings";

export default function App() {
  const { page, settingsLoaded, loadSettings } = useUi();
  useBootstrap();

  useEffect(() => {
    if (!settingsLoaded) void loadSettings();
  }, [settingsLoaded, loadSettings]);

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <Sidebar />
        <main className="min-w-0 flex-1 overflow-y-auto px-6 py-5">
          {page === "dashboard" && <DashboardPage />}
          {page === "performance" && <CategoryPage category="performance" title="Performance" />}
          {page === "gaming" && <CategoryPage category="gaming" title="Gaming" />}
          {page === "gpu" && <CategoryPage category="gpu" title="GPU" />}
          {page === "network" && <CategoryPage category="network" title="Network" />}
          {page === "cleanup" && <CleanupPage />}
          {page === "windows" && <CategoryPage category="windows" title="Windows" />}
          {page === "tweaks" && <TweaksPage />}
          {page === "changes" && <ChangesPage />}
          {page === "backup" && <BackupPage />}
          {page === "logs" && <LogsPage />}
          {page === "settings" && <SettingsPage />}
        </main>
      </div>
      <TweakDetailsDrawer />
    </div>
  );
}
