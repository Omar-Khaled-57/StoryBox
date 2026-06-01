import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AiStatus, AiHealthStatus, ScannedFolder } from "../App";
import Logo from "./Logo";

interface SettingsPanelProps {
  scanLog: string;
  isScanning: boolean;
  onScan: () => void;
  onScanDevice: () => void;
  aiHealth: AiHealthStatus | null;
  setAiHealth: (health: AiHealthStatus | null) => void;
  initialSection?: string | null;
  onInitialSectionHandled?: () => void;
  scannedFolders: ScannedFolder[];
  folderAccessibility: Map<string, boolean>;
  onAddScannedFolder: () => void;
  onRemoveScannedFolder: (id: string) => void;
  onToggleScannedFolder: (id: string) => void;
  onReScanFolders: () => void;
  onToggleChildFolder: (parentPath: string, childName: string) => void;
}

export default function SettingsPanel({
  scanLog, isScanning, onScan, onScanDevice, aiHealth, setAiHealth, initialSection, onInitialSectionHandled,
  scannedFolders, folderAccessibility, onAddScannedFolder, onRemoveScannedFolder, onToggleScannedFolder, onReScanFolders,
  onToggleChildFolder
}: SettingsPanelProps) {
  const [aiStatus, setAiStatus] = useState<AiStatus | null>(null);
  const [loadingStatus, setLoadingStatus] = useState(true);
  const [toast, setToast] = useState<{ message: string; type: "success" | "error" | "info" } | null>(null);

  const showToast = (message: string, type: "success" | "error" | "info" = "info") => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 4000);
  };

  const [loadingAction, setLoadingAction] = useState<string | null>(null);

  const fetchStatus = async () => {
    try { setAiStatus(await invoke<AiStatus>("get_ai_status")); }
    catch (e) { console.error(e); }
    finally { setLoadingStatus(false); }
  };

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (initialSection) {
      const el = document.getElementById(initialSection);
      if (el) { el.scrollIntoView({ behavior: "smooth", block: "start" }); onInitialSectionHandled?.(); }
    }
  }, [initialSection, onInitialSectionHandled]);

  const analyzedPct = aiStatus && aiStatus.total_images > 0 ? Math.round((aiStatus.analyzed_images / aiStatus.total_images) * 100) : 0;
  const indexingPct = aiStatus && aiStatus.total_found > 0 ? Math.round((aiStatus.indexed_count / aiStatus.total_found) * 100) : 100;
  const isPrioritizingIndexing = aiStatus && aiStatus.total_found > 5 && (aiStatus.indexed_count / aiStatus.total_found) < 0.7;

  return (
    <div className="absolute inset-0 overflow-y-auto overflow-x-hidden scroll-smooth">
      <div className="max-w-3xl mx-auto px-6 py-10 pb-40 flex flex-col gap-8 animate-fade-in">
        <header className="mb-2">
          <div className="flex items-center gap-3 mb-2">
            <div className="p-2.5 rounded-2xl bg-neon-500/10 border border-neon-500/20 shadow-lg shadow-neon-500/5">
              <SettingsIcon />
            </div>
            <div>
              <h1 className="text-3xl font-bold tracking-tight text-surface-50 leading-none neon-glow-text">Settings</h1>
              <p className="text-surface-400 text-sm mt-1.5 font-medium">Configure your library and AI parameters.</p>
            </div>
          </div>
          {toast && (
            <div className={`fixed top-4 left-1/2 -translate-x-1/2 z-[9999] backdrop-blur-xl border rounded-xl px-5 py-3 shadow-2xl shadow-black/50 text-sm font-medium animate-scale-in whitespace-pre-line max-w-md text-center flex items-center gap-3 ${
              toast.type === "success"
                ? "bg-emerald-900/95 border-emerald-500/30 text-emerald-300"
                : toast.type === "error"
                ? "bg-red-900/95 border-red-500/30 text-red-300"
                : "bg-surface-900/95 border-neon-500/30 text-surface-50"
            }`}>
              {toast.type === "success" && <CheckIcon />}
              {toast.type === "error" && <InfoIcon />}
              {toast.type === "info" && <ClockIcon />}
              {toast.message}
            </div>
          )}
        </header>

        <Section icon={<FolderIcon />} title="Library Locations" description="Manage where StoryBox3 looks for your memories.">
          <div className="flex flex-col gap-5">
            <div className="relative">
              <div className="flex items-center gap-3 bg-surface-900/60 border border-neon-500/10 rounded-xl px-4 py-3.5">
                <div className="flex items-center justify-center w-5 h-5">
                  {isScanning ? <span className="w-2 h-2 rounded-full bg-neon-400 animate-pulse" /> : <div className="w-1.5 h-1.5 rounded-full bg-surface-600" />}
                </div>
                <span className="font-mono text-[11px] text-neon-300 flex-1 min-w-0 truncate tracking-tight uppercase">
                  {isScanning && aiStatus ? `Discovering: ${aiStatus.total_found} found...` : scanLog}
                </span>
              </div>
            </div>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <button className="btn-secondary flex items-center justify-center gap-2.5 px-5 py-3.5 font-bold text-xs uppercase tracking-widest disabled:opacity-50" onClick={onScanDevice} disabled={isScanning}>
                <DeviceIcon /><span>Scan Device</span>
              </button>
              <button className="btn-primary flex items-center justify-center gap-2.5 px-5 py-3.5 font-bold text-xs uppercase tracking-widest disabled:opacity-50" onClick={onScan} disabled={isScanning}>
                <FolderPlusIcon /><span>{isScanning ? "Scanning..." : "Add Folder"}</span>
              </button>
            </div>
            {aiStatus && (aiStatus.pending_images > 0 || aiStatus.is_indexing_paused) && (
              <button onClick={async () => { await invoke(aiStatus.is_indexing_paused ? "resume_indexing" : "stop_indexing"); fetchStatus(); }}
                className={`w-full py-3 rounded-xl border font-bold text-[10px] uppercase tracking-[0.2em] transition-all active:scale-95 ${
                  aiStatus.is_indexing_paused ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-400" : "bg-amber-500/10 border-amber-500/30 text-amber-400"
                }`}>
                {aiStatus.is_indexing_paused ? "Resume Indexing" : "Pause Indexing"}
              </button>
            )}
          </div>
        </Section>

        <Section icon={<FolderTreeIcon />} title="Scanned Folders" description="Manage which folders StoryBox3 is allowed to scan for photos.">
          <ScannedFoldersSection
            folders={scannedFolders}
            accessibility={folderAccessibility}
            isScanning={isScanning}
            onAdd={onAddScannedFolder}
            onRemove={onRemoveScannedFolder}
            onToggle={onToggleScannedFolder}
            onReScan={onReScanFolders}
            onToggleChild={onToggleChildFolder}
          />
        </Section>

        <Section icon={<BrainIcon />} title="AI Storyteller" description="Configure OpenRouter, Ollama, or Mock mode for story generation.">
          <AiSettingsSection />
        </Section>

        <Section id="ai-health-check" icon={<ShieldIcon />} title="AI Health Check" description="Verify connection to your AI provider.">
          <div className="flex flex-col gap-4">
            {!aiHealth ? (
              <div className="flex flex-col items-center justify-center py-4 gap-3 bg-surface-900/40 rounded-2xl border border-neon-500/10 italic text-surface-500 text-xs">
                Checking AI availability...
              </div>
            ) : (
              <div className="flex flex-col gap-4">
                <div className={`p-4 rounded-2xl border flex items-start gap-4 transition-all ${
                  aiHealth.available ? "bg-emerald-500/5 border-emerald-500/20 text-emerald-400" : "bg-red-500/5 border-red-500/20 text-red-400"
                }`}>
                  <div className={`p-2 rounded-xl ${aiHealth.available ? "bg-emerald-500/10" : "bg-red-500/10"}`}>
                    {aiHealth.available ? <CheckIcon /> : <InfoIcon />}
                  </div>
                  <div className="flex flex-col gap-1">
                    <span className="text-sm font-bold uppercase tracking-tight">{aiHealth.available ? "Connected" : "Disconnected"}</span>
                    <p className="text-xs opacity-80 leading-relaxed font-medium">{aiHealth.message}</p>
                  </div>
                </div>
                {aiHealth.available && aiHealth.vision_model && (
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <ModelBadge label="Vision" name={aiHealth.vision_model} loaded={aiHealth.vision_model_loaded} />
                    <ModelBadge label="Text" name={aiHealth.text_model} loaded={aiHealth.text_model_loaded} />
                  </div>
                )}
                <button onClick={async () => { setAiHealth(null); setAiHealth(await invoke<AiHealthStatus>("check_ai_availability")); }}
                  className="btn-neon w-full py-3 text-xs font-bold uppercase tracking-widest">
                  Refresh Health Status
                </button>
              </div>
            )}
          </div>
        </Section>

        <Section icon={<SparkIcon />} title="AI Intelligence" description="Real-time status of semantic analysis and image tagging.">
          {loadingStatus ? (
            <div className="flex flex-col items-center justify-center py-8 gap-3">
              <span className="w-8 h-8 border-3 border-surface-800 border-t-neon-500 rounded-full animate-spin" />
              <span className="text-xs font-bold uppercase tracking-widest text-surface-500">Connecting...</span>
            </div>
          ) : aiStatus ? (
            <div className="flex flex-col gap-6">
              <div className="p-5 bg-surface-900/40 rounded-2xl border border-neon-500/10">
                <div className="flex justify-between items-end mb-3">
                  <div className="flex flex-col">
                    <span className="text-[10px] font-bold uppercase tracking-widest text-surface-400 mb-1">
                      {isPrioritizingIndexing ? "Flash Indexing" : "AI Queue"}
                    </span>
                    <span className={`text-sm font-bold ${aiStatus.pending_images > 0 ? (isPrioritizingIndexing ? "text-cyan-400" : "text-amber-400") : "text-emerald-400"}`}>
                      {isPrioritizingIndexing ? "Indexing photos..." : (aiStatus.pending_images > 0 ? "Processing Active" : "Up to Date")}
                    </span>
                  </div>
                  <div className="flex flex-col items-end text-right">
                    <div className="flex items-baseline gap-1.5">
                      <span className="text-2xl font-black text-white">{isPrioritizingIndexing ? aiStatus.indexed_count : aiStatus.analyzed_images}</span>
                      <span className="text-xs font-bold text-surface-500">/ {isPrioritizingIndexing ? aiStatus.total_found : aiStatus.total_images}</span>
                    </div>
                    <span className="text-[9px] font-bold text-surface-500 uppercase tracking-widest">{isPrioritizingIndexing ? "Images Indexed" : "Images Analyzed"}</span>
                  </div>
                </div>
                <div className="h-3 bg-surface-800/50 rounded-full overflow-hidden p-1 border border-neon-500/10 relative">
                  <div className="h-full bg-gradient-to-r from-neon-500 to-cyber-500 rounded-full transition-all duration-500 ease-out"
                    style={{ width: `${isPrioritizingIndexing ? indexingPct : analyzedPct}%` }} />
                  {aiStatus.is_analysis_paused && !isPrioritizingIndexing && (
                    <div className="absolute inset-0 bg-surface-900/40 backdrop-blur-[1px] flex items-center justify-center">
                      <span className="text-[8px] font-black text-white/50 uppercase tracking-[0.3em]">Paused</span>
                    </div>
                  )}
                </div>
                <div className="flex justify-between items-center mt-4 pt-4 border-t border-neon-500/10">
                  <button onClick={async () => { await invoke(aiStatus.is_analysis_paused ? "resume_analysis" : "stop_analysis"); fetchStatus(); }}
                    className={`px-4 py-2 rounded-lg border font-bold text-[9px] uppercase tracking-widest transition-all active:scale-95 ${
                      aiStatus.is_analysis_paused ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-400" : "bg-surface-800 border-neon-500/10 text-surface-400"
                    }`}>
                    {aiStatus.is_analysis_paused ? "Resume Analysis" : "Pause Analysis"}
                  </button>
                </div>
              </div>
              <div className="grid grid-cols-3 gap-3">
                <StatBox label="Library" value={aiStatus.total_images} icon={<ImageIcon />} />
                <StatBox label="Analyzed" value={aiStatus.analyzed_images} color="emerald" icon={<CheckIcon />} />
                <StatBox label="Pending" value={aiStatus.pending_images} color={aiStatus.pending_images > 0 ? "amber" : "dim"} icon={<ClockIcon />} />
              </div>
              <div className="flex flex-col gap-0.5 rounded-xl border border-neon-500/10 overflow-hidden">
                <div className="flex justify-between items-center px-4 py-3 bg-surface-900/40">
                  <span className="text-[10px] font-bold uppercase tracking-widest text-surface-400">Engine</span>
                  <div className="flex items-center gap-2">
                    <span className={`w-1.5 h-1.5 rounded-full ${aiStatus.is_mock ? "bg-amber-400" : "bg-emerald-400"}`} />
                    <span className="text-xs font-bold text-surface-100">{aiStatus.engine_name}</span>
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <div className="flex items-center gap-3 p-4 bg-red-500/10 border border-red-500/20 rounded-xl text-red-400 text-sm font-medium">
              <InfoIcon />AI Status Unavailable
            </div>
          )}
        </Section>

        <Section icon={<TrashIcon />} title="Maintenance" description="Manage your generated stories.">
          <div className="flex flex-col gap-4">
            <MaintRow title="Repair AI Analysis" desc="Re-queue images with garbled tags for re-analysis."
              action="Start Repair" loading={loadingAction === "repair"}
              onClick={async () => {
                setLoadingAction("repair");
                try {
                  const c = await invoke<number>("trigger_junk_reanalysis");
                  showToast(`Re-queued ${c} images for re-analysis.`, "success");
                } catch (e) {
                  showToast(`Repair failed: ${e}`, "error");
                } finally {
                  setLoadingAction(null);
                }
              }} />
            <MaintRow title="Diagnostic Dry Run" desc="Test image analysis and caption generation."
              action="Test AI" color="purple" loading={loadingAction === "dryrun"}
              onClick={async () => {
                setLoadingAction("dryrun");
                try {
                  const r: any = await invoke("test_ai_generation");
                  showToast(`Dry Run: ${r.success ? "OK" : "FAIL"}\n${r.generated_caption || ""}`, r.success ? "success" : "error");
                } catch (e) {
                  showToast(`Dry Run failed: ${e}`, "error");
                } finally {
                  setLoadingAction(null);
                }
              }} />
            <MaintRow title="Delete All Stories" desc="Remove non-favorited stories permanently."
              action="Clear All" color="red" loading={loadingAction === "clear"}
              onClick={async () => {
                setLoadingAction("clear");
                try {
                  await invoke("delete_all_stories");
                  showToast("Non-favorited stories deleted.", "success");
                } catch (e) {
                  showToast(`Delete failed: ${e}`, "error");
                } finally {
                  setLoadingAction(null);
                }
              }} />
            <MaintRow title="Reset App" desc="Clear everything and restart."
              action="Reset" color="red" loading={loadingAction === "reset"}
              onClick={async () => {
                if (!confirm("Delete EVERYTHING?")) return;
                setLoadingAction("reset");
                try {
                  await invoke("reset_app");
                } catch (e) {
                  showToast(`Reset failed: ${e}`, "error");
                } finally {
                  setLoadingAction(null);
                }
              }} />
          </div>
        </Section>

        <Section icon={<InfoIcon />} title="App Information" description="Technical details.">
          <div className="flex flex-col rounded-2xl border border-neon-500/10 overflow-hidden bg-surface-900/40">
            <ManifestRow label="Version" value="3.0.0" />
            <ManifestRow label="Architecture" value="Tauri 2 · React · Rust" />
            <ManifestRow label="AI Provider" value="OpenRouter (Cloud)" />
            <ManifestRow label="Backend DB" value="SQLite via sqlx" />
            <ManifestRow label="Free Models" value="Gemma 3 / Qwen VL / Llama 3.1" last />
          </div>
        </Section>

        <footer className="mt-4 flex flex-col items-center gap-4 py-6 px-10 rounded-3xl bg-neon-500/5 border border-neon-500/10 text-center">
          <p className="text-surface-400 text-xs font-medium max-w-sm leading-relaxed">
            StoryBox3 uses OpenRouter for cloud-based AI via a secure server-side proxy. No API key is stored in the app.
          </p>
          <div className="flex gap-6">
            <Logo variant="full" size="lg" animated />
          </div>
          <span className="text-[9px] text-surface-600 tracking-widest uppercase">Popcorn</span>
        </footer>
      </div>
    </div>
  );
}

// ── Scanned Folders Section ──
function ScannedFoldersSection({ folders, accessibility, isScanning, onAdd, onRemove, onToggle, onReScan, onToggleChild }: {
  folders: ScannedFolder[];
  accessibility: Map<string, boolean>;
  isScanning: boolean;
  onAdd: () => void;
  onRemove: (id: string) => void;
  onToggle: (id: string) => void;
  onReScan: () => void;
  onToggleChild: (parentPath: string, childName: string) => void;
}) {
  const inaccessibleCount = folders.filter(f => accessibility.get(f.id) === false).length;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2">
        <button className="btn-primary flex items-center justify-center gap-2 px-5 py-3 font-bold text-xs uppercase tracking-widest disabled:opacity-50 flex-1" onClick={onAdd} disabled={isScanning}>
          <FolderPlusIcon /><span>Add Folder</span>
        </button>
        <button className="btn-secondary flex items-center justify-center gap-2 px-4 py-3 font-bold text-xs uppercase tracking-widest disabled:opacity-50" onClick={onReScan} disabled={isScanning}>
          <RefreshIcon /><span>Re-scan</span>
        </button>
      </div>

      {inaccessibleCount > 0 && (
        <div className="flex items-center gap-2.5 px-4 py-3 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-400 text-xs font-medium">
          <InfoIcon />{inaccessibleCount} folder(s) are no longer accessible. Remove or update their paths.
        </div>
      )}

      <div className="flex flex-col gap-1.5 max-h-80 overflow-y-auto pr-1 scroll-smooth">
        {folders.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-6 gap-2 bg-surface-900/30 rounded-xl border border-dashed border-neon-500/10">
            <span className="text-surface-500 text-xs font-medium">No folders configured yet.</span>
            <span className="text-surface-600 text-[10px]">Click "Add Folder" to get started.</span>
          </div>
        ) : (
          folders.map(f => {
            const accessible = accessibility.get(f.id);
            const isMissing = accessible === false;
            return (
              <FolderRow
                key={f.id}
                folder={f}
                isMissing={isMissing}
                onToggle={onToggle}
                onRemove={onRemove}
                onToggleChild={onToggleChild}
              />
            );
          })
        )}
      </div>
    </div>
  );
}

function FolderRow({ folder, isMissing, onToggle, onRemove, onToggleChild }: {
  folder: ScannedFolder;
  isMissing: boolean;
  onToggle: (id: string) => void;
  onRemove: (id: string) => void;
  onToggleChild: (parentPath: string, childName: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [children, setChildren] = useState<{ name: string; path: string; disabled: boolean }[] | null>(null);
  const [loadingChildren, setLoadingChildren] = useState(false);

  const loadChildren = useCallback(async () => {
    if (children !== null || isMissing) return;
    setLoadingChildren(true);
    try {
      const result = await invoke<{ name: string; path: string; disabled: boolean }[]>("get_folder_children", { path: folder.path });
      setChildren(result);
    } catch { setChildren([]); }
    finally { setLoadingChildren(false); }
  }, [folder.path, children, isMissing]);

  const handleToggle = () => {
    const next = !expanded;
    setExpanded(next);
    if (next) loadChildren();
  };

  const handleChildToggle = (childName: string) => {
    // Optimistic update — flip disabled state locally, then sync to backend
    setChildren(prev => prev?.map(c =>
      c.name === childName ? { ...c, disabled: !c.disabled } : c
    ) ?? prev);
    onToggleChild(folder.path, childName);
  };

  return (
    <div className="flex flex-col">
      <div className={`flex items-center gap-1.5 px-3.5 py-2.5 rounded-xl border transition-all duration-200 ${
        isMissing
          ? "bg-red-900/20 border-red-500/20"
          : folder.is_enabled
            ? "bg-surface-900/60 border-neon-500/10"
            : "bg-surface-900/30 border-surface-700/30 opacity-60"
      }`}>
        <button
          onClick={handleToggle}
          className={`shrink-0 w-4 h-4 flex items-center justify-center text-surface-500 hover:text-surface-300 transition-colors ${!isMissing ? "" : "invisible"}`}
          title={expanded ? "Collapse" : "Expand"}
        >
          {loadingChildren ? (
            <span className="w-2.5 h-2.5 border-2 border-surface-600 border-t-current rounded-full animate-spin" />
          ) : (
            <svg className={`w-3 h-3 transition-transform duration-200 ${expanded ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3"><polyline points="9 18 15 12 9 6"/></svg>
          )}
        </button>
        <button
          onClick={() => onToggle(folder.id)}
          className={`shrink-0 w-10 h-6 rounded-full transition-colors duration-200 p-1 flex items-center ${
            folder.is_enabled ? (isMissing ? "bg-red-500/40" : "bg-neon-500") : "bg-surface-700"
          }`}
          title={folder.is_enabled ? "Enabled" : "Disabled"}
        >
          <span className={`w-4 h-4 bg-white rounded-full shadow transition-transform duration-200 ${
            folder.is_enabled ? "translate-x-4" : "translate-x-0"
          }`} />
        </button>
        <div className="flex-1 min-w-0 flex flex-col gap-0">
          <span className={`text-xs font-bold truncate ${isMissing ? "text-red-400" : "text-surface-100"}`}
            title={folder.path}>
            {folder.path}
          </span>
          <div className="flex items-center gap-2">
            <span className={`text-[9px] font-semibold uppercase tracking-wider ${
              isMissing ? "text-red-400" : folder.is_enabled ? "text-emerald-400" : "text-surface-500"
            }`}>
              {isMissing ? "Missing" : folder.is_enabled ? "Enabled" : "Disabled"}
            </span>
            {children !== null && (
              <span className="text-[9px] text-surface-500">
                {children.length} subfolder{children.length !== 1 ? "s" : ""}
              </span>
            )}
          </div>
        </div>
        <button onClick={() => onRemove(folder.id)}
          className="shrink-0 p-1.5 rounded-lg text-surface-500 hover:text-red-400 hover:bg-red-500/10 transition-colors active:scale-90"
          title="Remove folder">
          <TrashIconSmall />
        </button>
      </div>
      {expanded && children !== null && children.length > 0 && (
        <div className="flex flex-col ml-5 mt-1 gap-1">
          {children.map(child => {
            const childEnabled = !child.disabled && folder.is_enabled;
            return (
              <div key={child.path} className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-surface-900/20 border border-neon-500/5">
                <button
                  onClick={() => handleChildToggle(child.name)}
                  className={`shrink-0 w-7 h-4 rounded-full transition-colors duration-200 p-0.5 flex items-center ${
                    childEnabled ? "bg-neon-500/60" : "bg-surface-700/60"
                  }`}
                  title={childEnabled ? "Enabled" : "Disabled"}
                >
                  <span className={`w-3 h-3 bg-white rounded-full shadow transition-transform duration-200 ${
                    childEnabled ? "translate-x-3" : "translate-x-0"
                  }`} />
                </button>
                <svg className="shrink-0 w-3 h-3 text-surface-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
                <span className="text-[11px] text-surface-300 truncate flex-1" title={child.path}>{child.name}</span>
                <span className={`text-[8px] font-semibold uppercase tracking-wider ${
                  childEnabled ? "text-emerald-400/60" : "text-surface-500/60"
                }`}>
                  {childEnabled ? "Enabled" : "Disabled"}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

// ── AI Settings (with OpenRouter support) ──
function AiSettingsSection() {
  const [provider, setProvider] = useState<string>("openrouter");
  const [url, setUrl] = useState("https://openrouter.ai/api/v1");
  const [model, setModel] = useState("meta-llama/llama-3.1-8b-instruct");
  const [visionModel, setVisionModel] = useState("qwen/qwen-vl-plus");
  const [apiKey, setApiKey] = useState("");
  const [proxyUrl, setProxyUrl] = useState("");
  const [autoGenInterval, setAutoGenInterval] = useState(12);
  const [cleanupInterval, setCleanupInterval] = useState(24);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    invoke<[string, string, string, string, string, string, number, number]>("get_ai_settings")
      .then(([p, u, m, v, k, pr, ag, cl]) => {
        setProvider(p); setUrl(u); setModel(m); setVisionModel(v); setApiKey(k); setProxyUrl(pr); setAutoGenInterval(ag); setCleanupInterval(cl);
      })
      .catch(console.error);
  }, []);

  const handleSave = async () => {
    setSaving(true); setMessage("");
    try {
      await invoke("update_ai_settings", {
        provider, baseUrl: url, modelName: model, visionModelName: visionModel, apiKey, proxyUrl,
        autoGenIntervalHours: Number(autoGenInterval), cleanupIntervalHours: Number(cleanupInterval)
      });
      setMessage("Saved!"); setTimeout(() => setMessage(""), 3000);
    } catch (e) { setMessage(`Error: ${e}`); }
    finally { setSaving(false); }
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <label className="text-[10px] font-bold uppercase tracking-widest text-surface-400">AI Provider</label>
        <div className="grid grid-cols-3 gap-2">
          {(["mock", "ollama", "openrouter"] as const).map((p) => (
            <button key={p} onClick={() => setProvider(p)}
              className={`px-3 py-2.5 rounded-xl text-xs font-bold transition-all border ${
                provider === p ? "bg-neon-500/10 border-neon-500 text-neon-400" : "bg-surface-900 border-neon-500/10 text-surface-400 hover:border-neon-500/30"
              }`}>
              {p === "mock" ? "Mock" : p === "ollama" ? "Ollama" : "OpenRouter"}
            </button>
          ))}
        </div>
      </div>

      {/* ── Story Generation Automation ── */}
      <div className="flex flex-col gap-4 p-4 bg-gradient-to-br from-neon-500/5 to-cyber-500/10 rounded-2xl border border-neon-500/15">
        <div className="flex items-center gap-2">
          <svg className="w-4 h-4 text-neon-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
          <h3 className="text-xs font-bold tracking-widest text-neon-300 uppercase">Auto Story Generation</h3>
        </div>
        <p className="text-[10px] text-surface-500 leading-relaxed">
          Stories are generated automatically on a schedule. The app checks every 30 minutes
          and creates a new story when the interval has passed since the last generation.
          Old unpinned stories are cleaned up after the expiration period.
        </p>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div className="flex flex-col gap-1.5">
            <label className="text-[10px] font-bold uppercase tracking-widest text-surface-400">Generate Every (hours)</label>
            <input type="number" min="1" value={autoGenInterval} onChange={e => setAutoGenInterval(parseInt(e.target.value) || 1)}
              className="cyber-input" />
            <p className="text-[9px] text-surface-600">How often a new story is auto-generated</p>
          </div>
          <div className="flex flex-col gap-1.5">
            <label className="text-[10px] font-bold uppercase tracking-widest text-surface-400">Expire After (hours)</label>
            <input type="number" min="1" value={cleanupInterval} onChange={e => setCleanupInterval(parseInt(e.target.value) || 1)}
              className="cyber-input" />
            <p className="text-[9px] text-surface-600">Unpinned stories older than this are auto-deleted</p>
          </div>
        </div>
      </div>

      {provider === "openrouter" && (
        <div className="flex flex-col gap-4 p-4 bg-surface-900/60 rounded-2xl border border-neon-500/10 animate-scale-in">
          <div className="flex flex-col gap-1.5">
            <label className="text-[10px] font-bold uppercase tracking-widest text-surface-400">API Key</label>
            <input type="password" value={apiKey} onChange={e => setApiKey(e.target.value)}
              placeholder="sk-or-v1-..." className="cyber-input font-mono" />
          </div>
          <div className="flex flex-col gap-1.5">
            <label className="text-[10px] font-bold uppercase tracking-widest text-surface-400">Proxy URL (optional)</label>
            <input type="text" value={proxyUrl} onChange={e => setProxyUrl(e.target.value)}
              placeholder="https://your-project.supabase.co/functions/v1/openrouter-proxy" className="cyber-input font-mono" />
            <p className="text-[9px] text-surface-600">If set, requests go through this proxy instead of directly (key stays server-side)</p>
          </div>
          <div className="flex flex-col gap-1.5">
            <label className="text-[10px] font-bold uppercase tracking-widest text-surface-400">API Base URL</label>
            <input type="text" value={url} onChange={e => setUrl(e.target.value)}
              className="cyber-input" />
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div className="flex flex-col gap-1.5">
              <label className="text-[10px] font-bold uppercase tracking-widest text-surface-500">Text Model</label>
              <input type="text" value={model} onChange={e => setModel(e.target.value)}
                placeholder="meta-llama/llama-3.1-8b-instruct" className="cyber-input" />
            </div>
            <div className="flex flex-col gap-1.5">
              <label className="text-[10px] font-bold uppercase tracking-widest text-surface-500">Vision Model</label>
              <input type="text" value={visionModel} onChange={e => setVisionModel(e.target.value)}
                placeholder="qwen/qwen-vl-plus" className="cyber-input" />
            </div>
          </div>
          <p className="text-[10px] text-surface-500 font-medium">
            Free models: <span className="text-neon-400">google/gemma-3-27b-it</span> (text), <span className="text-neon-400">qwen/qwen-vl-plus</span> (vision), <span className="text-neon-400">meta-llama/llama-3.1-8b-instruct</span> (text)
          </p>
        </div>
      )}

      {provider === "ollama" && (
        <div className="flex flex-col gap-4 p-4 bg-surface-900/60 rounded-2xl border border-neon-500/10 animate-scale-in">
          <div className="flex flex-col gap-1.5">
            <label className="text-[10px] font-bold uppercase tracking-widest text-surface-500">Ollama URL</label>
            <input type="text" value={url} onChange={e => setUrl(e.target.value)}
              className="cyber-input" />
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div className="flex flex-col gap-1.5">
              <label className="text-[10px] font-bold uppercase tracking-widest text-surface-500">Text Model</label>
              <input type="text" value={model} onChange={e => setModel(e.target.value)} placeholder="llama3" className="cyber-input" />
            </div>
            <div className="flex flex-col gap-1.5">
              <label className="text-[10px] font-bold uppercase tracking-widest text-surface-500">Vision Model</label>
              <input type="text" value={visionModel} onChange={e => setVisionModel(e.target.value)} placeholder="moondream" className="cyber-input" />
            </div>
          </div>
        </div>
      )}

      <div className="flex items-center justify-between gap-4 mt-2">
        <span className="text-xs font-semibold text-neon-400/80">{message}</span>
        <button onClick={handleSave} disabled={saving}
          className="px-6 py-2.5 rounded-xl bg-neon-500 text-white text-xs font-bold uppercase tracking-wider hover:bg-neon-400 active:scale-95 transition-all disabled:opacity-50 neon-glow">
          {saving ? "Saving..." : "Save Config"}
        </button>
      </div>
    </div>
  );
}

function Section({ id, icon, title, description, children }: { id?: string; icon: React.ReactNode; title: string; description: string; children: React.ReactNode }) {
  return (
    <section id={id} className="relative group">
      <div className="cyber-card p-6 flex flex-col gap-5 relative z-0">
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2.5">
            <span className="text-neon-400 w-5 h-5">{icon}</span>
            <h2 className="text-lg font-bold text-surface-50 tracking-tight">{title}</h2>
          </div>
          <p className="text-xs text-surface-500 font-medium ml-7 tracking-wide">{description}</p>
        </div>
        <div className="flex flex-col flex-1">{children}</div>
      </div>
    </section>
  );
}

function StatBox({ label, value, color, icon }: { label: string; value: number; color?: "emerald" | "amber" | "dim"; icon: React.ReactNode }) {
  const cls = color === "emerald" ? "text-emerald-400 bg-emerald-400/5" : color === "amber" ? "text-amber-400 bg-amber-400/5" : color === "dim" ? "text-surface-500 bg-surface-500/5" : "text-neon-400 bg-neon-400/5";
  return (
    <div className="flex flex-col gap-1.5 bg-surface-900/60 border border-neon-500/10 rounded-2xl p-4 transition-transform hover:scale-[1.02]">
      <div className={`p-1.5 rounded-lg w-fit ${cls}`}>{icon}</div>
      <div className="flex flex-col">
        <span className="text-xl font-black text-white tracking-tight leading-none">{value.toLocaleString()}</span>
        <span className="text-[10px] font-bold uppercase tracking-widest text-surface-500">{label}</span>
      </div>
    </div>
  );
}

function ModelBadge({ label, name, loaded }: { label: string; name?: string; loaded?: boolean }) {
  return (
    <div className="flex flex-col gap-2 p-3 bg-surface-900/60 border border-neon-500/10 rounded-xl">
      <div className="flex justify-between items-center">
        <span className="text-[10px] font-bold uppercase tracking-widest text-surface-500">{label}</span>
        {loaded !== undefined && (
          <span className={`text-[9px] font-black uppercase px-2 py-0.5 rounded-full ${loaded ? "bg-emerald-500/20 text-emerald-400" : "bg-red-500/20 text-red-400"}`}>
            {loaded ? "Ready" : "Missing"}
          </span>
        )}
      </div>
      <span className="text-xs font-bold text-surface-200 truncate">{name || "Not Specified"}</span>
    </div>
  );
}

function MaintRow({ title, desc, action, color, loading, onClick }: { title: string; desc: string; action: string; color?: string; loading?: boolean; onClick: () => Promise<void> }) {
  const isRed = color === "red";
  const isPurple = color === "purple";
  const borderCls = isRed ? "border-red-500/10" : isPurple ? "border-cyber-500/10" : "border-neon-500/10";
  const bgCls = isRed ? "bg-red-500/5" : isPurple ? "bg-cyber-500/5" : "bg-neon-500/5";
  const textCls = isRed ? "text-red-400" : isPurple ? "text-cyber-400" : "text-neon-400";
  const btnBg = isRed ? "bg-red-500/10" : isPurple ? "bg-cyber-500/10" : "bg-neon-500/10";
  const btnBorder = isRed ? "border-red-500/20" : isPurple ? "border-cyber-500/20" : "border-neon-500/20";
  const btnHover = isRed ? "hover:bg-red-500" : isPurple ? "hover:bg-cyber-500" : "hover:bg-neon-500";
  return (
    <div className={`p-4 ${bgCls} ${borderCls} rounded-2xl flex flex-col sm:flex-row items-center justify-between gap-4`}>
      <div className="flex flex-col gap-0.5">
        <span className={`text-sm font-bold ${textCls}`}>{title}</span>
        <span className="text-[10px] text-surface-500 font-medium">{desc}</span>
      </div>
      <button onClick={onClick} disabled={loading}
        className={`shrink-0 px-5 py-2.5 rounded-xl font-bold text-xs uppercase tracking-widest transition-all active:scale-95 flex items-center gap-2 ${
          loading
            ? "bg-surface-800 border-surface-700 text-surface-500 cursor-not-allowed"
            : `${btnBg} ${btnBorder} ${textCls} ${btnHover} hover:text-white`
        }`}>
        {loading ? (
          <><span className="w-3.5 h-3.5 border-2 border-surface-600 border-t-current rounded-full animate-spin" />{action}</>
        ) : action}
      </button>
    </div>
  );
}

function ManifestRow({ label, value, last }: { label: string; value: string; last?: boolean }) {
  return (
    <div className={`flex justify-between items-center px-5 py-3.5 hover:bg-neon-500/5 transition-colors ${!last ? "border-b border-neon-500/10" : ""}`}>
      <span className="text-xs font-bold uppercase tracking-widest text-surface-500">{label}</span>
      <span className="text-xs font-bold text-surface-200">{value}</span>
    </div>
  );
}

// Icons
function SettingsIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="w-6 h-6"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/></svg>; }
function FolderIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-5 h-5"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>; }
function SparkIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-5 h-5"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>; }
function InfoIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="w-5 h-5"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>; }
function DeviceIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="w-4 h-4"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>; }
function FolderPlusIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="w-4 h-4"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/><line x1="12" y1="11" x2="12" y2="17"/><line x1="9" y1="14" x2="15" y2="14"/></svg>; }
function ImageIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="w-4 h-4"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>; }
function CheckIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="w-4 h-4"><polyline points="20 6 9 17 4 12"/></svg>; }
function ClockIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="w-4 h-4"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>; }
function BrainIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-5 h-5"><path d="M12 2a10 10 0 0110 10c0 5-4 8-10 8S2 17 2 12 7 2 12 2z"/><path d="M8 12h8"/><path d="M12 8v8"/></svg>; }
function TrashIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="w-5 h-5"><path d="M3 6h18m-2 0v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6m3 0V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>; }
function ShieldIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="w-5 h-5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>; }
function FolderTreeIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-5 h-5"><path d="M2 4h4l2 3h14v11H2V4z"/><path d="M8 12h4"/><path d="M14 12h2"/><path d="M8 16h8"/></svg>; }
function RefreshIcon() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="w-4 h-4"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/></svg>; }
function TrashIconSmall() { return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="w-4 h-4"><path d="M3 6h18m-2 0v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6m3 0V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>; }
