import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import HomeFeed from "./components/HomeFeed";
import StoryViewer from "./components/StoryViewer";
import SettingsPanel from "./components/SettingsPanel";
import ActivityIndicator, { ActivityItem } from "./components/ActivityIndicator";
import Logo from "./components/Logo";

export interface ImageRecord {
  id: string;
  path: string;
  date_taken: string | null;
  ai_analyzed: boolean;
  tags?: string[];
  vibe?: string;
}

export interface AiStatus {
  total_images: number;
  analyzed_images: number;
  pending_images: number;
  is_mock: boolean;
  engine_name: string;
  is_indexing_paused: boolean;
  is_analysis_paused: boolean;
  total_found: number;
  indexed_count: number;
}

export interface AiHealthStatus {
  available: boolean;
  provider?: string;
  vision_model?: string;
  vision_model_loaded?: boolean;
  text_model?: string;
  text_model_loaded?: boolean;
  message: string;
}

export interface Story {
  id: string;
  theme_type: string;
  caption: string;
  created_at: string;
  images: ImageRecord[];
  is_favorite: boolean;
  is_pinned: boolean;
}

export interface ScannedFolder {
  id: string;
  path: string;
  is_enabled: boolean;
  added_at: string;
}

type View = "home" | "favorites" | "settings";

let _nextId = 0;
function mkId() { return String(++_nextId); }

function App() {
  const [view, setView] = useState<View>("home");
  const [stories, setStories] = useState<Story[]>([]);
  const [activeStory, setActiveStory] = useState<Story | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [scanLog, setScanLog] = useState("Ready to index");
  const [isScanning, setIsScanning] = useState(false);
  const [showNoPhotosPopup, setShowNoPhotosPopup] = useState(false);
  const [aiHealth, setAiHealth] = useState<AiHealthStatus | null>(null);
  const [settingsInitialSection, setSettingsInitialSection] = useState<string | null>(null);
  const [activities, setActivities] = useState<ActivityItem[]>([]);
  const [scannedFolders, setScannedFolders] = useState<ScannedFolder[]>([]);
  const [folderAccessibility, setFolderAccessibility] = useState<Map<string, boolean>>(new Map());

  const addActivity = (label: string): string => {
    const id = mkId();
    setActivities((prev) => [...prev, { id, label }]);
    return id;
  };

  const removeActivity = (id: string) => {
    setActivities((prev) => prev.filter((a) => a.id !== id));
  };

  const loadStories = useCallback(async () => {
    try {
      const result = await invoke<Story[]>("get_stories");
      setStories(result);
    } catch (e) {
      console.error("Failed to load stories:", e);
    }
  }, []);

  const loadScannedFolders = useCallback(async () => {
    try {
      const folders = await invoke<ScannedFolder[]>("get_scanned_folders");
      setScannedFolders(folders);
      const accessResults: any[] = await invoke("check_folders_accessibility");
      const accessMap = new Map<string, boolean>();
      for (const r of accessResults) {
        accessMap.set(r.id, r.accessible);
      }
      setFolderAccessibility(accessMap);
    } catch (e) {
      console.error("Failed to load scanned folders:", e);
    }
  }, []);

  useEffect(() => {
    loadStories();
  }, [loadStories]);

  useEffect(() => {
    loadScannedFolders();
  }, [loadScannedFolders]);

  useEffect(() => {
    const unlistenIndexing = listen<{message: string; path: string}>("indexing-progress", (event) => {
      setActivities(prev => {
        const idx = prev.findIndex(a => a.id === "indexing");
        if (idx === -1) {
          return [...prev, { id: "indexing", label: "Indexing (1)", progress: 5, statusMessage: event.payload.message }];
        }
        const newActivities = [...prev];
        const current = newActivities[idx];
        const match = current.label.match(/\((\d+)\)/);
        const count = match ? parseInt(match[1]) + 1 : 1;
        newActivities[idx] = { ...current, label: `Indexing (${count})`, statusMessage: event.payload.message, progress: Math.min((current.progress || 0) + 0.2, 99) };
        return newActivities;
      });
      setScanLog(event.payload.message);
    });

    const unlistenAnalysis = listen<{message: string; id: string}>("analysis-progress", (event) => {
      setActivities(prev => {
        const idx = prev.findIndex(a => a.id === "analysis");
        if (idx === -1) {
          return [...prev, { id: "analysis", label: "AI Processing (1)", progress: 5, statusMessage: event.payload.message }];
        }
        const newActivities = [...prev];
        const current = newActivities[idx];
        const match = current.label.match(/\((\d+)\)/);
        const count = match ? parseInt(match[1]) + 1 : 1;
        newActivities[idx] = { ...current, label: `AI Processing (${count})`, statusMessage: event.payload.message, progress: Math.min((current.progress || 0) + 0.2, 99) };
        return newActivities;
      });
      setScanLog(event.payload.message);
      loadStories();
    });

    const unlistenRefresh = listen("refresh-stories", () => { loadStories(); });
    const unlistenNoPhotos = listen("no-photos-found", () => { setShowNoPhotosPopup(true); });

    const unlistenMobileScan = listen("trigger-mobile-scan", async (_event) => {
      const isIOS = navigator.userAgent.includes("iPhone") || navigator.userAgent.includes("iPad");
      if (isIOS) {
        try {
          const ios = await import("@gbyte/tauri-plugin-ios-photos");
          const status = await ios.requestPhotosAuth();
          if (status !== ios.PhotosAuthorizationStatus.authorized && status !== ios.PhotosAuthorizationStatus.limited) return;
          const albums = await ios.requestAlbums({ with: ios.PHAssetCollectionType.smartAlbum, subtype: ios.PHAssetCollectionSubtype.smartAlbumUserLibrary });
          const userAlbum = albums.find(() => true);
          if (!userAlbum) return;
          const medias = await ios.requestAlbumMedias({ id: userAlbum.id, height: 1080, width: 1080, quality: 80 });
          const items = medias || [];
          setActivities(prev => [...prev.filter(a => a.id !== "indexing"), { id: "indexing", label: `Indexing iOS Photos (${items.length})`, progress: 0 }]);
          for (let i = 0; i < items.length; i += 5) {
            const chunk = items.slice(i, i + 5);
            await Promise.all(chunk.map((item, idx) => invoke("index_ios_image_data", { data: item.data || "", fileName: `ios_${item.id}_${idx}.jpg` })));
            setActivities(prev => prev.map(a => a.id === "indexing" ? { ...a, progress: Math.min((i / items.length) * 100, 99) } : a));
          }
          setActivities(prev => prev.map(a => a.id === "indexing" ? { ...a, label: "iOS Scan Complete", progress: 100 } : a));
          setTimeout(() => setActivities(prev => prev.filter(a => a.id !== "indexing")), 5000);
          loadStories();
        } catch (e) { console.error("[App] iOS scan failed:", e); }
        return;
      }
      try {
        const { requestPermissions, getImages, MediaLibrarySource } = await import("@universalappfactory/tauri-plugin-medialibrary");
        await requestPermissions({ source: MediaLibrarySource.ExternalStorage });
        const result = await getImages({ limit: 1000, offset: 0, source: MediaLibrarySource.ExternalStorage });
        const images = result?.items || [];
        setActivities(prev => [...prev.filter(a => a.id !== "indexing"), { id: "indexing", label: `Indexing Mobile (${images.length})`, progress: 0 }]);
        for (let i = 0; i < images.length; i += 5) {
          const chunk = images.slice(i, i + 5);
          await Promise.all(chunk.map((img: any) => invoke("index_mobile_image", { path: img.path })));
          setActivities(prev => prev.map(a => a.id === "indexing" ? { ...a, progress: Math.min((i / images.length) * 100, 99) } : a));
        }
        setActivities(prev => prev.map(a => a.id === "indexing" ? { ...a, label: "Mobile Scan Complete", progress: 100 } : a));
        setTimeout(() => setActivities(prev => prev.filter(a => a.id !== "indexing")), 5000);
        loadStories();
      } catch (e) { console.error("[App] Android scan failed:", e); }
    });

    const unlistenScanComplete = listen<number>("scan-complete", (event) => {
      setActivities(prev => prev.map(a => a.id === "indexing" ? { ...a, label: `Scan Complete (${event.payload} found)`, progress: 100 } : a));
      setTimeout(() => setActivities(prev => prev.filter(a => a.id !== "indexing")), 5000);
      loadStories();
    });

    const unlistenAiHealth = listen<AiHealthStatus>("ai-health-status", (event) => { setAiHealth(event.payload); });
    const unlistenStoryUpdated = listen<{id: string; caption: string}>("story-updated", (event) => {
      setStories(prev => prev.map(s => s.id === event.payload.id ? { ...s, caption: event.payload.caption } : s));
    });

    invoke<AiHealthStatus>("check_ai_availability").then(setAiHealth).catch(console.error);

    // Auto-retry AI health check when disconnected
    const aiRetry = setInterval(() => {
      if (aiHealth !== null && !aiHealth.available) {
        invoke<AiHealthStatus>("check_ai_availability").then(setAiHealth).catch(console.error);
      }
    }, 15000);

    return () => {
      clearInterval(aiRetry);
      Promise.all([unlistenIndexing, unlistenAnalysis, unlistenRefresh, unlistenNoPhotos, unlistenMobileScan, unlistenScanComplete, unlistenAiHealth, unlistenStoryUpdated])
        .then(fns => fns.forEach(f => f()));
    };
  }, [loadStories, aiHealth]);

  const handleGenerateStory = async () => {
    setIsGenerating(true);
    const aid = addActivity("Generating story\u2026");
    try {
      const story = await invoke<Story | null>("generate_story");
      if (story) {
        setStories((prev) => [story, ...prev]);
        setActiveStory(story);
        setScanLog(story.theme_type === "random" ? "Template story (AI enrichment pending)" : `AI Story generated: ${story.caption}`);
      } else {
        setScanLog("No photos found. Add a folder first.");
      }
    } catch (e) {
      setScanLog(`Error: ${e}`);
    } finally {
      setIsGenerating(false);
      removeActivity(aid);
    }
  };

  const handleAddLocation = async () => {
    try {
      const dir = await open({ directory: true, multiple: false });
      if (!dir) return;
      setIsScanning(true);
      setView("settings");
      setScanLog(`Scanning: ${dir}\u2026`);
      setActivities(prev => [...prev.filter(a => a.id !== "indexing"), { id: "indexing", label: "Indexing Files", progress: 0 }]);
      await invoke<ScannedFolder>("add_scanned_folder", { path: dir }).catch(() => {});
      const count = await invoke<number>("start_scan", { dir });
      setScanLog(`Found ${count} images. Indexing...`);
      setActivities(prev => prev.map(a => a.id === "indexing" ? { ...a, label: `Indexing (${count})` } : a));
      loadStories();
      loadScannedFolders();
    } catch (e) { setScanLog(`Error: ${e}`); }
    finally { setIsScanning(false); }
  };

  const handleScanDevice = async () => {
    try {
      setIsScanning(true);
      setView("settings");
      setScanLog("Scanning device...");
      setActivities(prev => [...prev.filter(a => a.id !== "indexing"), { id: "indexing", label: "Scanning Device", progress: 0 }]);
      const count = await invoke<number>("start_scan_device");
      setScanLog(`Found ${count} images.`);
      setActivities(prev => prev.map(a => a.id === "indexing" ? { ...a, label: `Indexing (${count})` } : a));
      loadStories();
      loadScannedFolders();
    } catch (e) { setScanLog(`Error: ${e}`); }
    finally { setIsScanning(false); }
  };

  const handleAddScannedFolder = useCallback(async () => {
    try {
      const dir = await open({ directory: true, multiple: false, title: "Select a folder to scan" });
      if (!dir) return;
      const folder = await invoke<ScannedFolder>("add_scanned_folder", { path: dir });
      setScannedFolders(prev => [folder, ...prev]);
      setFolderAccessibility(prev => new Map(prev).set(folder.id, true));
      setIsScanning(true);
      setScanLog(`Scanning: ${dir}…`);
      setActivities(prev => [...prev.filter(a => a.id !== "indexing"), { id: "indexing", label: "Indexing Files", progress: 0 }]);
      const count = await invoke<number>("start_scan", { dir });
      setScanLog(`Found ${count} images. Indexing...`);
      setActivities(prev => prev.map(a => a.id === "indexing" ? { ...a, label: `Indexing (${count})` } : a));
      loadStories();
    } catch (e) {
      setScanLog(`Error adding folder: ${e}`);
    } finally {
      setIsScanning(false);
    }
  }, [loadStories]);

  const handleRemoveScannedFolder = useCallback(async (id: string) => {
    try {
      await invoke("remove_scanned_folder", { id });
      setScannedFolders(prev => prev.filter(f => f.id !== id));
      setFolderAccessibility(prev => { const m = new Map(prev); m.delete(id); return m; });
    } catch (e) {
      console.error("Failed to remove folder:", e);
    }
  }, []);

  const handleToggleScannedFolder = useCallback(async (id: string) => {
    setScannedFolders(prev => prev.map(f => f.id === id ? { ...f, is_enabled: !f.is_enabled } : f));
    try {
      const updated = await invoke<ScannedFolder>("toggle_scanned_folder", { id });
      setScannedFolders(prev => prev.map(f => f.id === id ? updated : f));
    } catch (e) {
      // Revert optimistic update on failure
      setScannedFolders(prev => prev.map(f => f.id === id ? { ...f, is_enabled: !f.is_enabled } : f));
      console.error("Failed to toggle folder:", e);
    }
  }, []);

  const handleToggleChildFolder = useCallback(async (parentPath: string, childName: string) => {
    try {
      await invoke<boolean>("toggle_child_override", { parentPath, childName });
      // Refetch folder children to update their disabled state
      // The FolderRow component will re-invoke get_folder_children when expanded again
    } catch (e) {
      console.error("Failed to toggle child folder:", e);
    }
  }, []);

  const handleReScanFolders = useCallback(async () => {
    setIsScanning(true);
    try {
      const count = await invoke<number>("start_scan_device");
      setScanLog(`Re-scan complete. Found ${count} images.`);
      loadStories();
      loadScannedFolders();
    } catch (e) {
      setScanLog(`Re-scan error: ${e}`);
    } finally {
      setIsScanning(false);
    }
  }, [loadStories, loadScannedFolders]);

  const navigateToSettingsSection = (sectionId: string) => {
    setSettingsInitialSection(sectionId);
    setView("settings");
  };

  const handleDeleteStory = async (id: string) => {
    try { await invoke("delete_story", { id }); setStories((prev) => prev.filter((s) => s.id !== id)); }
    catch (e) { console.error(e); }
  };

  const handleTogglePin = async (id: string) => {
    try {
      const is_pinned = await invoke<boolean>("toggle_story_pin", { id });
      setStories((prev) => [...prev].map((s) => (s.id === id ? { ...s, is_pinned } : s))
        .sort((a, b) => { if (a.is_pinned !== b.is_pinned) return a.is_pinned ? -1 : 1; return (new Date(b.created_at).getTime() || 0) - (new Date(a.created_at).getTime() || 0); }));
    } catch (e) { console.error(e); }
  };

  const handleToggleFavorite = async (id: string) => {
    try { const is_favorite = await invoke<boolean>("toggle_story_favorite", { id }); setStories((prev) => prev.map((s) => (s.id === id ? { ...s, is_favorite } : s))); }
    catch (e) { console.error(e); }
  };

  return (
    <div className="flex flex-col-reverse sm:flex-row w-screen h-screen overflow-hidden bg-surface-950 text-surface-50 font-sans selection:bg-neon-500/40">
      {activeStory && <StoryViewer story={activeStory} onClose={() => setActiveStory(null)} />}

      {/* Cyber Background */}
      <div className="cyber-bg">
        <div className="cyber-particles">
          {Array.from({ length: 12 }).map((_, i) => <div key={i} className="cyber-particle" />)}
        </div>
        <div className="cyber-scanline" />
      </div>

      {/* Navigation */}
      <nav className="relative z-10 flex flex-row sm:flex-col items-center justify-between sm:justify-start w-full sm:w-20 h-20 sm:h-full px-4 py-2 sm:py-12 sm:px-0 gap-2 sm:gap-6 shrink-0 border-t sm:border-t-0 sm:border-r border-neon-500/10 bg-surface-900/60 backdrop-blur-xl shadow-2xl">
        <div className="logo hidden sm:flex items-center justify-center mb-4 mt-2">
          <Logo size="md" animated />
        </div>

        <div className="flex flex-row sm:flex-col justify-center gap-4 sm:gap-4 flex-1 sm:w-full sm:px-3 h-full sm:h-auto items-center">
          <NavButton active={view === "home"} onClick={() => setView("home")} title="Home">
            <path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/>
          </NavButton>
          <NavButton active={view === "settings"} onClick={() => setView("settings")} title="Settings">
            <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/>
          </NavButton>
          <NavButton active={view === "favorites"} onClick={() => setView("favorites")} title="Favorites" purple fill>
            <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/>
          </NavButton>
          <button className="sm:hidden flex items-center justify-center w-12 aspect-square rounded-2xl bg-gradient-to-tr from-neon-500 to-neon-400 text-white shadow-lg shadow-neon-500/30 cursor-pointer transition-all duration-300 active:scale-95 group relative overflow-hidden" onClick={handleAddLocation} title="Add Folder" aria-label="Add Folder">
            <div className="absolute inset-0 bg-white/20 opacity-0 group-hover:opacity-100 transition-opacity" />
            <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/><line x1="12" y1="11" x2="12" y2="17"/><line x1="9" y1="14" x2="15" y2="14"/></svg>
          </button>
        </div>

        <ActivityIndicator activities={activities} />

        <div className="hidden sm:flex sm:w-full sm:px-3 sm:mb-4 h-full sm:h-auto items-center">
          <button className="flex sm:flex-col items-center justify-center w-12 sm:w-full aspect-square rounded-2xl bg-gradient-to-tr from-neon-500 to-neon-400 text-white shadow-lg shadow-neon-500/30 cursor-pointer transition-all duration-300 hover:scale-[1.05] hover:shadow-neon-500/50 active:scale-95 group relative overflow-hidden" onClick={handleAddLocation} title="Add Folder" aria-label="Add Folder">
            <div className="absolute inset-0 bg-white/20 opacity-0 group-hover:opacity-100 transition-opacity" />
            <svg className="w-6 h-6 sm:w-7 sm:h-7 sm:mb-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/><line x1="12" y1="11" x2="12" y2="17"/><line x1="9" y1="14" x2="15" y2="14"/></svg>
            <span className="hidden sm:inline text-[9px] font-bold uppercase tracking-wider opacity-90">Add</span>
          </button>
        </div>

      </nav>

      {/* Main Content */}
      <main className="relative z-10 flex-[1_1_100%] overflow-hidden flex flex-col">
        {view === "home" && (
          <HomeFeed stories={stories} onStoryClick={setActiveStory} onGenerate={handleGenerateStory} isGenerating={isGenerating}
            onDeleteStory={handleDeleteStory} onTogglePin={handleTogglePin} onToggleFavorite={handleToggleFavorite}
            aiHealth={aiHealth} onNavigateToSettings={navigateToSettingsSection} />
        )}
        {view === "favorites" && (
          <HomeFeed stories={stories.filter(s => s.is_favorite)} onStoryClick={setActiveStory} onGenerate={handleGenerateStory} isGenerating={isGenerating}
            onDeleteStory={handleDeleteStory} onTogglePin={handleTogglePin} onToggleFavorite={handleToggleFavorite}
            aiHealth={aiHealth} onNavigateToSettings={navigateToSettingsSection} title="Your Favorites" />
        )}
        {view === "settings" && (
          <SettingsPanel scanLog={scanLog} isScanning={isScanning} onScan={handleAddLocation} onScanDevice={handleScanDevice}
            aiHealth={aiHealth} setAiHealth={setAiHealth} initialSection={settingsInitialSection}
            onInitialSectionHandled={() => setSettingsInitialSection(null)}
            scannedFolders={scannedFolders} folderAccessibility={folderAccessibility}
            onAddScannedFolder={handleAddScannedFolder} onRemoveScannedFolder={handleRemoveScannedFolder}
            onToggleScannedFolder={handleToggleScannedFolder} onReScanFolders={handleReScanFolders}
            onToggleChildFolder={handleToggleChildFolder} />
        )}
      </main>

      {/* No Photos Popup */}
      {showNoPhotosPopup && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-surface-950/80 backdrop-blur-md animate-in fade-in duration-300">
          <div className="relative w-full max-w-sm cyber-card p-8 shadow-2xl shadow-black/50 animate-in zoom-in-95 duration-300">
            <div className="absolute -inset-0.5 bg-gradient-to-r from-neon-500/20 to-cyber-500/20 rounded-2xl blur opacity-50" />
            <div className="relative flex flex-col items-center text-center gap-6">
              <div className="w-20 h-20 rounded-full bg-neon-500/10 flex items-center justify-center text-neon-400 neon-glow">
                <svg className="w-10 h-10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242" /><path d="M12 12v9" /><path d="m8 17 4 4 4-4" /></svg>
              </div>
              <div className="flex flex-col gap-2">
                <h3 className="text-xl font-bold text-surface-50 neon-glow-text">Deep Silence...</h3>
                <p className="text-sm text-surface-400 leading-relaxed">No photos were found. Try adding a folder first.</p>
              </div>
              <button onClick={() => { setShowNoPhotosPopup(false); setView("settings"); }}
                className="w-full py-4 rounded-xl bg-gradient-to-r from-neon-600 to-neon-400 text-white font-bold text-sm tracking-wider uppercase hover:shadow-neon-500/40 active:scale-[0.98] transition-all shadow-lg shadow-neon-500/20">
                Go to Settings
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function NavButton({ active, onClick, title, children, purple, fill }: {
  active: boolean; onClick: () => void; title: string; children: React.ReactNode; purple?: boolean; fill?: boolean;
}) {
  let cls = "flex items-center justify-center w-12 sm:w-full aspect-square rounded-2xl border-none cursor-pointer transition-all duration-300 active:scale-95 group relative overflow-hidden ";
  if (active) {
    if (purple) cls += "text-cyber-400 ";
    else cls += "text-neon-400 ";
  } else {
    cls += "text-surface-400 hover:text-neon-300 hover:bg-neon-500/5 ";
  }
  const glow = active ? (purple ? "cyber-glow-purple" : "neon-glow") : "";
  const bg = active ? (purple ? "bg-cyber-500/10" : "bg-neon-500/10") : "opacity-0";
  return (
    <button className={`${cls} ${glow}`} onClick={onClick} title={title} aria-label={title}>
      <div className={`absolute inset-0 transition-opacity duration-300 ${bg}`} />
      <svg className={`w-6 h-6 relative z-10 transition-transform group-hover:scale-110 ${fill ? 'fill-current' : ''}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        {children}
      </svg>
    </button>
  );
}

export default App;
